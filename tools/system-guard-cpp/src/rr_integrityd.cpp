#include "rr_db.hpp"
#include "rr_policy.hpp"
#include "rr_util.hpp"

#include <Security/Security.h>
#include <libproc.h>
#include <netinet/in.h>
#include <signal.h>
#include <sys/proc_info.h>

#include <chrono>
#include <cmath>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <optional>
#include <sstream>
#include <string>
#include <thread>
#include <unordered_map>
#include <utility>
#include <vector>

namespace fs = std::filesystem;

namespace rr::guard {

namespace {

constexpr const char* kDefaultHome = "/var/db/rr-integrity";
constexpr const char* kDefaultPolicy = "/Users/nnos/Projects/rational-reserve/config/rr-policy.toml";
constexpr const char* kDefaultLog = "events.jsonl";
constexpr const char* kDefaultDb = "findings.db";

struct ProcessSample {
  int pid = 0;
  int ppid = 0;
  int uid = 0;
  std::string comm;
  std::string parent_comm;
  std::string exe;
  std::string command_line;
  std::string signer;
  uint64_t total_cpu_ns = 0;
  uint64_t rss_kb = 0;
  uint64_t vsz_kb = 0;
  int thread_count = 0;
  int listener_count = 0;
  int network_socket_count = 0;
  bool trusted = false;
  bool actionable = false;
  bool apple = false;
};

struct ProcessAggregate {
  std::string subject;
  std::string comm;
  std::string parent_comm;
  std::string exe;
  std::string command_line;
  std::string signer;
  uint64_t total_cpu_ns = 0;
  uint64_t rss_kb = 0;
  uint64_t vsz_kb = 0;
  int process_count = 0;
  int thread_count = 0;
  int listener_count = 0;
  int network_socket_count = 0;
  bool trusted = false;
  bool actionable = false;
  bool apple = false;
};

struct Profile {
  int sample_count = 0;
  double avg_cpu_pct = 0.0;
  double avg_rss_kb = 0.0;
  double avg_thread_count = 0.0;
  double avg_listener_count = 0.0;
  double avg_network_socket_count = 0.0;
  double cpu_m2 = 0.0;
  double rss_m2 = 0.0;
  double thread_m2 = 0.0;
  double listener_m2 = 0.0;
  double network_socket_m2 = 0.0;
  double max_cpu_pct = 0.0;
  uint64_t max_rss_kb = 0;
  int max_thread_count = 0;
  int max_listener_count = 0;
  int max_network_socket_count = 0;
  uint64_t last_total_cpu_ns = 0;
  std::string last_seen_at;
  std::string baseline_signer;
  std::string baseline_parent_comm;
  bool listener_seen = false;
  bool network_seen = false;
  bool seen = false;
};

struct SocketActivity {
  int listeners = 0;
  int active_network = 0;
};

struct ScoredReason {
  std::string name;
  double weight = 0.0;
};

struct AnomalyDecision {
  double raw_score = 0.0;
  double threshold_offset = 0.0;
  double effective_score = 0.0;
  std::string severity = "info";
  std::string recommended_action = "observe";
  std::vector<ScoredReason> reasons;
};

struct ContextCounts {
  int parent_seen_count = 0;
  int signer_seen_count = 0;
};

std::string default_string(const char* value) {
  return value ? std::string(value) : std::string();
}

bool is_apple_path(const std::string& exe) {
  return starts_with(exe, "/System/") || starts_with(exe, "/usr/libexec/") ||
         starts_with(exe, "/usr/sbin/");
}

bool list_contains(const std::vector<std::string>& values, const std::string& needle) {
  for (const auto& value : values) {
    if (value == needle) {
      return true;
    }
  }
  return false;
}

bool path_matches(const std::vector<std::string>& values, const std::string& path) {
  for (const auto& value : values) {
    if (starts_with(path, value)) {
      return true;
    }
  }
  return false;
}

std::string process_subject(const std::string& exe, const std::string& comm) {
  if (!exe.empty()) {
    return exe;
  }
  return comm;
}

std::optional<ProcessSample> read_process_sample(pid_t pid, const Policy& policy) {
  proc_bsdinfo bsd{};
  if (proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, &bsd, PROC_PIDTBSDINFO_SIZE) != PROC_PIDTBSDINFO_SIZE) {
    return std::nullopt;
  }

  proc_taskallinfo task{};
  if (proc_pidinfo(pid, PROC_PIDTASKALLINFO, 0, &task, PROC_PIDTASKALLINFO_SIZE) != PROC_PIDTASKALLINFO_SIZE) {
    return std::nullopt;
  }

  char path_buf[PROC_PIDPATHINFO_MAXSIZE] = {0};
  std::string exe;
  const int path_len = proc_pidpath(pid, path_buf, sizeof(path_buf));
  if (path_len > 0) {
    exe.assign(path_buf, static_cast<size_t>(path_len));
  }

  char name_buf[PROC_PIDPATHINFO_MAXSIZE] = {0};
  std::string comm;
  if (proc_name(pid, name_buf, sizeof(name_buf)) > 0) {
    comm = name_buf;
  }
  if (comm.empty()) {
    comm = exe.empty() ? std::to_string(pid) : fs::path(exe).filename().string();
  }

  std::string parent_comm;
  if (bsd.pbi_ppid > 0) {
    char parent_name_buf[PROC_PIDPATHINFO_MAXSIZE] = {0};
    if (proc_name(bsd.pbi_ppid, parent_name_buf, sizeof(parent_name_buf)) > 0) {
      parent_comm = parent_name_buf;
    }
  }

  std::string command_line = exe;
  if (command_line.empty()) {
    command_line = comm;
  }

  ProcessSample sample;
  sample.pid = pid;
  sample.ppid = static_cast<int>(bsd.pbi_ppid);
  sample.uid = static_cast<int>(bsd.pbi_uid);
  sample.comm = comm;
  sample.parent_comm = parent_comm;
  sample.exe = exe;
  sample.command_line = command_line;
  sample.signer = "unsigned";
  sample.total_cpu_ns = static_cast<uint64_t>(task.ptinfo.pti_total_user + task.ptinfo.pti_total_system);
  sample.rss_kb = static_cast<uint64_t>(task.ptinfo.pti_resident_size / 1024);
  sample.vsz_kb = static_cast<uint64_t>(task.ptinfo.pti_virtual_size / 1024);
  sample.thread_count = task.ptinfo.pti_threadnum;
  sample.apple = is_apple_path(sample.exe);
  sample.trusted = sample.apple || list_contains(policy.trusted_process_names, sample.comm) ||
                   path_matches(policy.trusted_path_prefixes, sample.exe);
  sample.actionable = list_contains(policy.actionable_process_names, sample.comm) ||
                      path_matches(policy.actionable_path_prefixes, sample.exe);
  return sample;
}

std::string code_signing_identity(const std::string& exe) {
  if (exe.empty()) {
    return "unknown";
  }

  CFURLRef url = CFURLCreateFromFileSystemRepresentation(
      kCFAllocatorDefault,
      reinterpret_cast<const UInt8*>(exe.c_str()),
      static_cast<CFIndex>(exe.size()),
      false);
  if (url == nullptr) {
    return "unknown";
  }

  SecStaticCodeRef code = nullptr;
  if (SecStaticCodeCreateWithPath(url, kSecCSDefaultFlags, &code) != errSecSuccess || code == nullptr) {
    CFRelease(url);
    return "unknown";
  }

  CFDictionaryRef info = nullptr;
  const auto status = SecCodeCopySigningInformation(
      code, kSecCSSigningInformation | kSecCSRequirementInformation, &info);
  std::string result = "unsigned";
  if (status == errSecSuccess && info != nullptr) {
    CFStringRef identifier = static_cast<CFStringRef>(
        CFDictionaryGetValue(info, kSecCodeInfoIdentifier));
    if (identifier != nullptr) {
      char buffer[512] = {0};
      if (CFStringGetCString(identifier, buffer, sizeof(buffer), kCFStringEncodingUTF8)) {
        result = buffer;
      }
    }
    CFRelease(info);
  }

  CFRelease(code);
  CFRelease(url);
  return result;
}

SocketActivity collect_socket_activity(pid_t pid) {
  const int fd_bytes = proc_pidinfo(pid, PROC_PIDLISTFDS, 0, nullptr, 0);
  if (fd_bytes <= 0) {
    return {};
  }

  std::vector<proc_fdinfo> fds(static_cast<size_t>(fd_bytes / static_cast<int>(sizeof(proc_fdinfo))) + 1);
  const int got = proc_pidinfo(pid, PROC_PIDLISTFDS, 0, fds.data(),
                               static_cast<int>(fds.size() * sizeof(proc_fdinfo)));
  if (got <= 0) {
    return {};
  }

  const size_t count = static_cast<size_t>(got / sizeof(proc_fdinfo));
  SocketActivity activity;
  for (size_t i = 0; i < count; ++i) {
    if (fds[i].proc_fdtype != PROX_FDTYPE_SOCKET) {
      continue;
    }
    socket_fdinfo info{};
    const int info_bytes = proc_pidfdinfo(pid, fds[i].proc_fd, PROC_PIDFDSOCKETINFO, &info,
                                          PROC_PIDFDSOCKETINFO_SIZE);
    if (info_bytes != PROC_PIDFDSOCKETINFO_SIZE) {
      continue;
    }
    if (info.psi.soi_family != AF_INET && info.psi.soi_family != AF_INET6) {
      continue;
    }
    if (info.psi.soi_kind == SOCKINFO_TCP) {
      if (info.psi.soi_proto.pri_tcp.tcpsi_state == TSI_S_LISTEN) {
        activity.listeners += 1;
      } else {
        activity.active_network += 1;
      }
    } else if (info.psi.soi_kind == SOCKINFO_IN) {
      activity.active_network += 1;
    }
  }
  return activity;
}

std::vector<ProcessSample> collect_process_samples(const Policy& policy) {
  std::vector<pid_t> pids(static_cast<size_t>(proc_listallpids(nullptr, 0) / sizeof(pid_t)) + 64);
  const int bytes = proc_listallpids(pids.data(), static_cast<int>(pids.size() * sizeof(pid_t)));
  if (bytes <= 0) {
    return {};
  }
  const size_t count = static_cast<size_t>(bytes / sizeof(pid_t));
  std::vector<ProcessSample> samples;
  samples.reserve(count);
  for (size_t i = 0; i < count; ++i) {
    const pid_t pid = pids[i];
    if (pid <= 0) {
      continue;
    }
    auto sample = read_process_sample(pid, policy);
    if (sample.has_value()) {
      const auto sockets = collect_socket_activity(pid);
      sample->listener_count = sockets.listeners;
      sample->network_socket_count = sockets.active_network;
      sample->signer = code_signing_identity(sample->exe);
      samples.push_back(std::move(*sample));
    }
  }
  return samples;
}

std::unordered_map<std::string, ProcessAggregate> aggregate_processes(
    const std::vector<ProcessSample>& samples) {
  std::unordered_map<std::string, ProcessAggregate> aggregates;
  for (const auto& sample : samples) {
    const std::string subject = process_subject(sample.exe, sample.comm);
    auto& agg = aggregates[subject];
    if (agg.subject.empty()) {
      agg.subject = subject;
      agg.comm = sample.comm;
      agg.parent_comm = sample.parent_comm;
      agg.exe = sample.exe;
      agg.command_line = sample.command_line;
      agg.signer = sample.signer;
      agg.trusted = sample.trusted;
      agg.actionable = sample.actionable;
      agg.apple = sample.apple;
    }
    agg.total_cpu_ns += sample.total_cpu_ns;
    agg.rss_kb += sample.rss_kb;
    agg.vsz_kb += sample.vsz_kb;
    agg.process_count += 1;
    agg.thread_count += sample.thread_count;
    agg.listener_count += sample.listener_count;
    agg.network_socket_count += sample.network_socket_count;
  }
  return aggregates;
}

std::unordered_map<std::string, std::pair<std::string, std::string>> collect_launchd_snapshot(
    const Policy& policy) {
  std::unordered_map<std::string, std::pair<std::string, std::string>> snapshot;
  for (const auto& root : policy.protected_launchd_paths) {
    fs::path root_path(root);
    if (!fs::exists(root_path)) {
      continue;
    }
    for (const auto& entry : fs::directory_iterator(root_path)) {
      if (!entry.is_regular_file() || entry.path().extension() != ".plist") {
        continue;
      }
      const auto mtime = fs::last_write_time(entry.path()).time_since_epoch().count();
      snapshot[entry.path().string()] = {
          sha256_file(entry.path()),
          std::to_string(static_cast<long long>(mtime)),
      };
    }
  }
  return snapshot;
}

std::string profile_json(const Profile& profile) {
  std::ostringstream out;
  out << "{"
      << "\"sample_count\":" << profile.sample_count << ","
      << "\"avg_cpu_pct\":" << std::fixed << std::setprecision(4) << profile.avg_cpu_pct << ","
      << "\"avg_rss_kb\":" << std::fixed << std::setprecision(2) << profile.avg_rss_kb << ","
      << "\"avg_thread_count\":" << std::fixed << std::setprecision(2) << profile.avg_thread_count << ","
      << "\"avg_listener_count\":" << std::fixed << std::setprecision(2) << profile.avg_listener_count << ","
      << "\"avg_network_socket_count\":" << std::fixed << std::setprecision(2) << profile.avg_network_socket_count << ","
      << "\"cpu_m2\":" << std::fixed << std::setprecision(4) << profile.cpu_m2 << ","
      << "\"rss_m2\":" << std::fixed << std::setprecision(4) << profile.rss_m2 << ","
      << "\"thread_m2\":" << std::fixed << std::setprecision(4) << profile.thread_m2 << ","
      << "\"listener_m2\":" << std::fixed << std::setprecision(4) << profile.listener_m2 << ","
      << "\"network_socket_m2\":" << std::fixed << std::setprecision(4) << profile.network_socket_m2 << ","
      << "\"max_cpu_pct\":" << std::fixed << std::setprecision(4) << profile.max_cpu_pct << ","
      << "\"max_rss_kb\":" << profile.max_rss_kb << ","
      << "\"max_thread_count\":" << profile.max_thread_count << ","
      << "\"max_listener_count\":" << profile.max_listener_count << ","
      << "\"max_network_socket_count\":" << profile.max_network_socket_count << ","
      << "\"listener_seen\":" << (profile.listener_seen ? "true" : "false") << ","
      << "\"network_seen\":" << (profile.network_seen ? "true" : "false") << ","
      << "\"last_total_cpu_ns\":" << profile.last_total_cpu_ns << ","
      << "\"baseline_signer\":" << json_escape(profile.baseline_signer) << ","
      << "\"baseline_parent_comm\":" << json_escape(profile.baseline_parent_comm) << ","
      << "\"last_seen_at\":" << json_escape(profile.last_seen_at)
      << "}";
  return out.str();
}

void record_finding(sqlite3* db, const fs::path& log_path, int run_id, const std::string& category,
                    const std::string& severity, const std::string& subject,
                    const std::string& payload);

Profile load_profile(sqlite3* db, const std::string& subject) {
  sqlite3_stmt* stmt = nullptr;
  Profile profile;
  const char* sql =
      "SELECT sample_count, avg_cpu_pct, avg_rss_kb, avg_thread_count, avg_listener_count, "
      "avg_network_socket_count, cpu_m2, rss_m2, thread_m2, listener_m2, network_socket_m2, "
      "max_cpu_pct, max_rss_kb, max_thread_count, max_listener_count, max_network_socket_count, "
      "last_total_cpu_ns, last_seen_at, baseline_signer, baseline_parent_comm, "
      "profile_json "
      "FROM process_profiles WHERE subject = ?1";
  sqlite3_prepare_v2(db, sql, -1, &stmt, nullptr);
  sqlite3_bind_text(stmt, 1, subject.c_str(), -1, SQLITE_TRANSIENT);
  if (sqlite3_step(stmt) == SQLITE_ROW) {
    profile.seen = true;
    profile.sample_count = sqlite3_column_int(stmt, 0);
    profile.avg_cpu_pct = sqlite3_column_double(stmt, 1);
    profile.avg_rss_kb = sqlite3_column_double(stmt, 2);
    profile.avg_thread_count = sqlite3_column_double(stmt, 3);
    profile.avg_listener_count = sqlite3_column_double(stmt, 4);
    profile.avg_network_socket_count = sqlite3_column_double(stmt, 5);
    profile.cpu_m2 = sqlite3_column_double(stmt, 6);
    profile.rss_m2 = sqlite3_column_double(stmt, 7);
    profile.thread_m2 = sqlite3_column_double(stmt, 8);
    profile.listener_m2 = sqlite3_column_double(stmt, 9);
    profile.network_socket_m2 = sqlite3_column_double(stmt, 10);
    profile.max_cpu_pct = sqlite3_column_double(stmt, 11);
    profile.max_rss_kb = static_cast<uint64_t>(sqlite3_column_int64(stmt, 12));
    profile.max_thread_count = sqlite3_column_int(stmt, 13);
    profile.max_listener_count = sqlite3_column_int(stmt, 14);
    profile.max_network_socket_count = sqlite3_column_int(stmt, 15);
    profile.last_total_cpu_ns = static_cast<uint64_t>(sqlite3_column_int64(stmt, 16));
    profile.last_seen_at = default_string(reinterpret_cast<const char*>(sqlite3_column_text(stmt, 17)));
    profile.baseline_signer = default_string(reinterpret_cast<const char*>(sqlite3_column_text(stmt, 18)));
    profile.baseline_parent_comm = default_string(reinterpret_cast<const char*>(sqlite3_column_text(stmt, 19)));
    const auto meta = default_string(reinterpret_cast<const char*>(sqlite3_column_text(stmt, 20)));
    profile.listener_seen = meta.find("\"listener_seen\":true") != std::string::npos;
    profile.network_seen = meta.find("\"network_seen\":true") != std::string::npos;
  }
  sqlite3_finalize(stmt);
  return profile;
}

void save_profile(sqlite3* db, const ProcessAggregate& aggregate, double cpu_pct,
                  const std::string& seen_at) {
  Profile existing = load_profile(db, aggregate.subject);
  if (!existing.seen) {
    sqlite3_stmt* stmt = nullptr;
    sqlite3_prepare_v2(
        db,
        "INSERT INTO process_profiles(subject, sample_count, avg_cpu_pct, avg_rss_kb, avg_thread_count, "
        "avg_listener_count, avg_network_socket_count, cpu_m2, rss_m2, thread_m2, listener_m2, network_socket_m2, "
        "max_cpu_pct, max_rss_kb, max_thread_count, max_listener_count, max_network_socket_count, "
        "last_total_cpu_ns, first_seen_at, last_seen_at, baseline_signer, baseline_parent_comm, profile_json) "
        "VALUES(?1, 1, ?2, ?3, ?4, ?5, ?6, 0, 0, 0, 0, 0, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        -1, &stmt, nullptr);
    sqlite3_bind_text(stmt, 1, aggregate.subject.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_double(stmt, 2, cpu_pct);
    sqlite3_bind_double(stmt, 3, static_cast<double>(aggregate.rss_kb));
    sqlite3_bind_double(stmt, 4, static_cast<double>(aggregate.thread_count));
    sqlite3_bind_double(stmt, 5, static_cast<double>(aggregate.listener_count));
    sqlite3_bind_double(stmt, 6, static_cast<double>(aggregate.network_socket_count));
    sqlite3_bind_double(stmt, 7, cpu_pct);
    sqlite3_bind_int64(stmt, 8, static_cast<sqlite3_int64>(aggregate.rss_kb));
    sqlite3_bind_int(stmt, 9, aggregate.thread_count);
    sqlite3_bind_int(stmt, 10, aggregate.listener_count);
    sqlite3_bind_int(stmt, 11, aggregate.network_socket_count);
    sqlite3_bind_int64(stmt, 12, static_cast<sqlite3_int64>(aggregate.total_cpu_ns));
    sqlite3_bind_text(stmt, 13, seen_at.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 14, seen_at.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 15, aggregate.signer.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_bind_text(stmt, 16, aggregate.parent_comm.c_str(), -1, SQLITE_TRANSIENT);
    Profile created;
    created.sample_count = 1;
    created.avg_cpu_pct = cpu_pct;
    created.avg_rss_kb = static_cast<double>(aggregate.rss_kb);
    created.avg_thread_count = static_cast<double>(aggregate.thread_count);
    created.avg_listener_count = static_cast<double>(aggregate.listener_count);
    created.avg_network_socket_count = static_cast<double>(aggregate.network_socket_count);
    created.max_cpu_pct = cpu_pct;
    created.max_rss_kb = aggregate.rss_kb;
    created.max_thread_count = aggregate.thread_count;
    created.max_listener_count = aggregate.listener_count;
    created.max_network_socket_count = aggregate.network_socket_count;
    created.last_total_cpu_ns = aggregate.total_cpu_ns;
    created.last_seen_at = seen_at;
    created.baseline_signer = aggregate.signer;
    created.baseline_parent_comm = aggregate.parent_comm;
    created.listener_seen = aggregate.listener_count > 0;
    created.network_seen = aggregate.network_socket_count > 0;
    const auto json = profile_json(created);
    sqlite3_bind_text(stmt, 17, json.c_str(), -1, SQLITE_TRANSIENT);
    sqlite3_step(stmt);
    sqlite3_finalize(stmt);
    return;
  }

  const int count = existing.sample_count + 1;
  const double avg_cpu =
      ((existing.avg_cpu_pct * existing.sample_count) + cpu_pct) / static_cast<double>(count);
  const double avg_rss =
      ((existing.avg_rss_kb * existing.sample_count) + static_cast<double>(aggregate.rss_kb)) /
      static_cast<double>(count);
  const double avg_threads =
      ((existing.avg_thread_count * existing.sample_count) + static_cast<double>(aggregate.thread_count)) /
      static_cast<double>(count);
  const double avg_listeners =
      ((existing.avg_listener_count * existing.sample_count) + static_cast<double>(aggregate.listener_count)) /
      static_cast<double>(count);
  const double avg_network_sockets =
      ((existing.avg_network_socket_count * existing.sample_count) +
       static_cast<double>(aggregate.network_socket_count)) /
      static_cast<double>(count);
  Profile next = existing;
  const auto update_m2 = [](double old_mean, double new_mean, double old_m2, double value) {
    const double delta = value - old_mean;
    const double delta2 = value - new_mean;
    return old_m2 + (delta * delta2);
  };
  next.sample_count = count;
  next.avg_cpu_pct = avg_cpu;
  next.avg_rss_kb = avg_rss;
  next.avg_thread_count = avg_threads;
  next.avg_listener_count = avg_listeners;
  next.avg_network_socket_count = avg_network_sockets;
  next.cpu_m2 = update_m2(existing.avg_cpu_pct, avg_cpu, existing.cpu_m2, cpu_pct);
  next.rss_m2 = update_m2(existing.avg_rss_kb, avg_rss, existing.rss_m2, static_cast<double>(aggregate.rss_kb));
  next.thread_m2 =
      update_m2(existing.avg_thread_count, avg_threads, existing.thread_m2, static_cast<double>(aggregate.thread_count));
  next.listener_m2 = update_m2(existing.avg_listener_count, avg_listeners, existing.listener_m2,
                               static_cast<double>(aggregate.listener_count));
  next.network_socket_m2 = update_m2(existing.avg_network_socket_count, avg_network_sockets,
                                     existing.network_socket_m2,
                                     static_cast<double>(aggregate.network_socket_count));
  next.max_cpu_pct = std::max(existing.max_cpu_pct, cpu_pct);
  next.max_rss_kb = std::max(existing.max_rss_kb, aggregate.rss_kb);
  next.max_thread_count = std::max(existing.max_thread_count, aggregate.thread_count);
  next.max_listener_count = std::max(existing.max_listener_count, aggregate.listener_count);
  next.max_network_socket_count =
      std::max(existing.max_network_socket_count, aggregate.network_socket_count);
  next.last_total_cpu_ns = aggregate.total_cpu_ns;
  next.last_seen_at = seen_at;
  if (next.baseline_signer.empty()) {
    next.baseline_signer = aggregate.signer;
  }
  if (next.baseline_parent_comm.empty()) {
    next.baseline_parent_comm = aggregate.parent_comm;
  }
  next.listener_seen = existing.listener_seen || aggregate.listener_count > 0;
  next.network_seen = existing.network_seen || aggregate.network_socket_count > 0;

  sqlite3_stmt* stmt = nullptr;
  sqlite3_prepare_v2(
      db,
      "UPDATE process_profiles "
      "SET sample_count = ?2, avg_cpu_pct = ?3, avg_rss_kb = ?4, avg_thread_count = ?5, avg_listener_count = ?6, "
      "avg_network_socket_count = ?7, cpu_m2 = ?8, rss_m2 = ?9, thread_m2 = ?10, listener_m2 = ?11, "
      "network_socket_m2 = ?12, max_cpu_pct = ?13, max_rss_kb = ?14, max_thread_count = ?15, "
      "max_listener_count = ?16, max_network_socket_count = ?17, last_total_cpu_ns = ?18, last_seen_at = ?19, "
      "baseline_signer = ?20, baseline_parent_comm = ?21, profile_json = ?22 "
      "WHERE subject = ?1",
      -1, &stmt, nullptr);
  sqlite3_bind_text(stmt, 1, aggregate.subject.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_int(stmt, 2, next.sample_count);
  sqlite3_bind_double(stmt, 3, next.avg_cpu_pct);
  sqlite3_bind_double(stmt, 4, next.avg_rss_kb);
  sqlite3_bind_double(stmt, 5, next.avg_thread_count);
  sqlite3_bind_double(stmt, 6, next.avg_listener_count);
  sqlite3_bind_double(stmt, 7, next.avg_network_socket_count);
  sqlite3_bind_double(stmt, 8, next.cpu_m2);
  sqlite3_bind_double(stmt, 9, next.rss_m2);
  sqlite3_bind_double(stmt, 10, next.thread_m2);
  sqlite3_bind_double(stmt, 11, next.listener_m2);
  sqlite3_bind_double(stmt, 12, next.network_socket_m2);
  sqlite3_bind_double(stmt, 13, next.max_cpu_pct);
  sqlite3_bind_int64(stmt, 14, static_cast<sqlite3_int64>(next.max_rss_kb));
  sqlite3_bind_int(stmt, 15, next.max_thread_count);
  sqlite3_bind_int(stmt, 16, next.max_listener_count);
  sqlite3_bind_int(stmt, 17, next.max_network_socket_count);
  sqlite3_bind_int64(stmt, 18, static_cast<sqlite3_int64>(next.last_total_cpu_ns));
  sqlite3_bind_text(stmt, 19, next.last_seen_at.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 20, next.baseline_signer.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 21, next.baseline_parent_comm.c_str(), -1, SQLITE_TRANSIENT);
  const auto json = profile_json(next);
  sqlite3_bind_text(stmt, 22, json.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_step(stmt);
  sqlite3_finalize(stmt);
}

int load_context_count(sqlite3* db, const std::string& kind, const std::string& subject,
                       const std::string& value) {
  if (value.empty()) {
    return 0;
  }
  sqlite3_stmt* stmt = nullptr;
  sqlite3_prepare_v2(
      db,
      "SELECT sample_count FROM process_context_observations "
      "WHERE kind = ?1 AND subject = ?2 AND value = ?3",
      -1, &stmt, nullptr);
  sqlite3_bind_text(stmt, 1, kind.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 2, subject.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 3, value.c_str(), -1, SQLITE_TRANSIENT);
  int count = 0;
  if (sqlite3_step(stmt) == SQLITE_ROW) {
    count = sqlite3_column_int(stmt, 0);
  }
  sqlite3_finalize(stmt);
  return count;
}

void save_context_observation(sqlite3* db, const std::string& kind, const std::string& subject,
                              const std::string& value, const std::string& seen_at) {
  if (value.empty()) {
    return;
  }
  sqlite3_stmt* stmt = nullptr;
  sqlite3_prepare_v2(
      db,
      "INSERT INTO process_context_observations(kind, subject, value, sample_count, first_seen_at, last_seen_at) "
      "VALUES(?1, ?2, ?3, 1, ?4, ?5) "
      "ON CONFLICT(kind, subject, value) DO UPDATE SET sample_count = sample_count + 1, last_seen_at = excluded.last_seen_at",
      -1, &stmt, nullptr);
  sqlite3_bind_text(stmt, 1, kind.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 2, subject.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 3, value.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 4, seen_at.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 5, seen_at.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_step(stmt);
  sqlite3_finalize(stmt);
}

AnomalyDecision score_anomaly(const Policy& policy, const ProcessAggregate& aggregate,
                              const Profile& profile, double cpu_pct,
                              const ContextCounts& contexts) {
  AnomalyDecision decision;
  const auto adaptive_reason = [&](const std::string& name, double base_weight, double observed,
                                   double mean, double m2, int n, double z_threshold, double max_observed) {
    if (n < 2 || observed <= mean) {
      return;
    }
    double intensity = 0.0;
    const double variance = m2 / static_cast<double>(n - 1);
    const double stdev = variance > 0.0 ? std::sqrt(variance) : 0.0;
    if (stdev > 0.0) {
      const double z = (observed - mean) / stdev;
      if (z >= z_threshold) {
        intensity = z / z_threshold;
      }
    } else {
      const double floor = std::max(1.0, mean);
      const double ratio = observed / floor;
      if (ratio >= policy.zero_variance_spike_factor && observed > max_observed) {
        intensity = ratio / policy.zero_variance_spike_factor;
      }
    }
    if (intensity > 0.0) {
      decision.reasons.push_back({name, base_weight * std::min(3.0, intensity)});
    }
  };

  adaptive_reason("cpu_spike", policy.cpu_spike_score, cpu_pct, profile.avg_cpu_pct, profile.cpu_m2,
                  profile.sample_count, policy.cpu_z_threshold, profile.max_cpu_pct);
  adaptive_reason("rss_spike", policy.rss_spike_score, static_cast<double>(aggregate.rss_kb),
                  profile.avg_rss_kb, profile.rss_m2, profile.sample_count, policy.rss_z_threshold,
                  static_cast<double>(profile.max_rss_kb));
  adaptive_reason("thread_spike", policy.thread_spike_score, static_cast<double>(aggregate.thread_count),
                  profile.avg_thread_count, profile.thread_m2, profile.sample_count, policy.thread_z_threshold,
                  static_cast<double>(profile.max_thread_count));
  if (aggregate.listener_count > 0 && !profile.listener_seen) {
    decision.reasons.push_back({"new_listener_behavior", policy.new_listener_score});
  }
  if (aggregate.network_socket_count > 0 && !profile.network_seen) {
    decision.reasons.push_back({"new_network_activity", policy.new_network_activity_score});
  }
  adaptive_reason("listener_count_spike", policy.listener_spike_score,
                  static_cast<double>(aggregate.listener_count), profile.avg_listener_count,
                  profile.listener_m2, profile.sample_count, policy.listener_z_threshold,
                  static_cast<double>(profile.max_listener_count));
  adaptive_reason("network_activity_spike", policy.network_activity_spike_score,
                  static_cast<double>(aggregate.network_socket_count), profile.avg_network_socket_count,
                  profile.network_socket_m2, profile.sample_count, policy.network_z_threshold,
                  static_cast<double>(profile.max_network_socket_count));
  if (!profile.baseline_signer.empty() && !aggregate.signer.empty() &&
      profile.baseline_signer != aggregate.signer) {
    const double modifier =
        contexts.signer_seen_count <= 0 ? 1.0 : (contexts.signer_seen_count < 3 ? 0.75 : 0.5);
    decision.reasons.push_back({"signer_drift", policy.signer_drift_score * modifier});
  }
  if (!profile.baseline_parent_comm.empty() && !aggregate.parent_comm.empty() &&
      profile.baseline_parent_comm != aggregate.parent_comm) {
    const double modifier =
        contexts.parent_seen_count <= 0 ? 1.0 : (contexts.parent_seen_count < 3 ? 0.75 : 0.5);
    decision.reasons.push_back({"parent_drift", policy.parent_drift_score * modifier});
  }

  for (const auto& reason : decision.reasons) {
    decision.raw_score += reason.weight;
  }
  if (aggregate.actionable) {
    decision.raw_score += policy.actionable_score_boost;
  }
  if (!aggregate.trusted) {
    decision.raw_score += policy.untrusted_score_boost;
  }
  if (aggregate.trusted) {
    decision.threshold_offset += policy.trusted_subject_threshold_delta;
  }
  if (aggregate.apple) {
    decision.threshold_offset += policy.apple_subject_threshold_delta;
  }
  decision.effective_score = std::max(0.0, decision.raw_score - decision.threshold_offset);

  if (decision.effective_score >= policy.terminate_score_threshold) {
    decision.severity = "critical";
    decision.recommended_action = "terminate";
  } else if (decision.effective_score >= policy.contain_score_threshold) {
    decision.severity = "critical";
    decision.recommended_action = "contain";
  } else if (decision.effective_score >= policy.alert_score_threshold) {
    decision.severity = "warning";
    decision.recommended_action = "alert";
  } else if (decision.effective_score >= policy.log_score_threshold) {
    decision.severity = "info";
    decision.recommended_action = "observe";
  }
  return decision;
}

std::vector<int> subject_pids(const std::vector<ProcessSample>& samples, const std::string& subject) {
  std::vector<int> pids;
  for (const auto& sample : samples) {
    if (process_subject(sample.exe, sample.comm) == subject) {
      pids.push_back(sample.pid);
    }
  }
  return pids;
}

void enforce_decision(sqlite3* db, const fs::path& log_path, int run_id, const Policy& policy,
                      const std::string& subject, const std::vector<int>& pids,
                      const AnomalyDecision& decision) {
  if (!policy.enforce || pids.empty()) {
    return;
  }
  int signal_to_send = 0;
  if (decision.recommended_action == "contain") {
    signal_to_send = SIGSTOP;
  } else if (decision.recommended_action == "terminate") {
    signal_to_send = SIGTERM;
  } else {
    return;
  }

  std::vector<int> affected;
  for (const int pid : pids) {
    if (pid > 1 && kill(pid, signal_to_send) == 0) {
      affected.push_back(pid);
    }
  }
  if (affected.empty()) {
    return;
  }

  std::ostringstream payload;
  payload << "{\"action\":" << json_escape(decision.recommended_action)
          << ",\"signal\":" << signal_to_send << ",\"effective_score\":" << std::fixed
          << std::setprecision(2) << decision.effective_score << ",\"pids\":[";
  for (size_t i = 0; i < affected.size(); ++i) {
    if (i) payload << ",";
    payload << affected[i];
  }
  payload << "]}";
  record_finding(db, log_path, run_id, "process_action", "critical", subject, payload.str());
}

double cpu_pct_from_delta(const Profile& profile, const ProcessAggregate& aggregate,
                          const std::string& now) {
  if (!profile.seen || profile.last_seen_at.empty() || aggregate.total_cpu_ns < profile.last_total_cpu_ns) {
    return 0.0;
  }
  std::tm tm{};
  std::istringstream in(profile.last_seen_at);
  in >> std::get_time(&tm, "%Y-%m-%dT%H:%M:%SZ");
  if (in.fail()) {
    return 0.0;
  }
  const auto last = std::chrono::system_clock::from_time_t(timegm(&tm));
  std::tm tm_now{};
  std::istringstream now_in(now);
  now_in >> std::get_time(&tm_now, "%Y-%m-%dT%H:%M:%SZ");
  if (now_in.fail()) {
    return 0.0;
  }
  const auto current = std::chrono::system_clock::from_time_t(timegm(&tm_now));
  const auto elapsed =
      std::chrono::duration_cast<std::chrono::milliseconds>(current - last).count() / 1000.0;
  if (elapsed <= 0.0) {
    return 0.0;
  }
  const auto cpu_delta = static_cast<double>(aggregate.total_cpu_ns - profile.last_total_cpu_ns) / 1'000'000'000.0;
  return (cpu_delta / elapsed) * 100.0;
}

void insert_process_sample(sqlite3* db, int run_id, const ProcessSample& sample, const std::string& ts) {
  sqlite3_stmt* stmt = nullptr;
  sqlite3_prepare_v2(
      db,
      "INSERT INTO process_samples(run_id, ts, pid, ppid, uid, comm, exe, command_line, "
      "parent_comm, signer, cpu_total_ns, rss_kb, vsz_kb, thread_count, listener_count, network_socket_count, "
      "trusted, actionable, apple) "
      "VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
      -1, &stmt, nullptr);
  sqlite3_bind_int(stmt, 1, run_id);
  sqlite3_bind_text(stmt, 2, ts.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_int(stmt, 3, sample.pid);
  sqlite3_bind_int(stmt, 4, sample.ppid);
  sqlite3_bind_int(stmt, 5, sample.uid);
  sqlite3_bind_text(stmt, 6, sample.comm.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 7, sample.exe.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 8, sample.command_line.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 9, sample.parent_comm.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 10, sample.signer.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_int64(stmt, 11, static_cast<sqlite3_int64>(sample.total_cpu_ns));
  sqlite3_bind_int64(stmt, 12, static_cast<sqlite3_int64>(sample.rss_kb));
  sqlite3_bind_int64(stmt, 13, static_cast<sqlite3_int64>(sample.vsz_kb));
  sqlite3_bind_int(stmt, 14, sample.thread_count);
  sqlite3_bind_int(stmt, 15, sample.listener_count);
  sqlite3_bind_int(stmt, 16, sample.network_socket_count);
  sqlite3_bind_int(stmt, 17, sample.trusted ? 1 : 0);
  sqlite3_bind_int(stmt, 18, sample.actionable ? 1 : 0);
  sqlite3_bind_int(stmt, 19, sample.apple ? 1 : 0);
  sqlite3_step(stmt);
  sqlite3_finalize(stmt);
}

void write_event(const fs::path& log_path, const std::string& category, const std::string& severity,
                 const std::string& subject, const std::string& payload) {
  std::ofstream out(log_path, std::ios::app);
  out << "{\"ts\":" << json_escape(utc_now()) << ",\"category\":" << json_escape(category)
      << ",\"severity\":" << json_escape(severity) << ",\"subject\":" << json_escape(subject)
      << ",\"payload\":" << payload << "}\n";
}

void record_finding(sqlite3* db, const fs::path& log_path, int run_id, const std::string& category,
                    const std::string& severity, const std::string& subject,
                    const std::string& payload) {
  sqlite3_stmt* stmt = nullptr;
  const auto ts = utc_now();
  sqlite3_prepare_v2(
      db,
      "INSERT INTO findings(run_id, ts, category, severity, subject, payload_json) "
      "VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
      -1, &stmt, nullptr);
  sqlite3_bind_int(stmt, 1, run_id);
  sqlite3_bind_text(stmt, 2, ts.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 3, category.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 4, severity.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 5, subject.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 6, payload.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_step(stmt);
  sqlite3_finalize(stmt);
  write_event(log_path, category, severity, subject, payload);
}

std::string json_map_payload(const std::unordered_map<std::string, std::string>& values) {
  std::ostringstream out;
  out << "{";
  bool first = true;
  for (const auto& [key, value] : values) {
    if (!first) {
      out << ",";
    }
    first = false;
    out << json_escape(key) << ":" << json_escape(value);
  }
  out << "}";
  return out.str();
}

void save_baseline(sqlite3* db, const std::string& category, const std::string& payload_json) {
  sqlite3_stmt* stmt = nullptr;
  sqlite3_prepare_v2(
      db,
      "INSERT INTO baselines(category, payload_json, updated_at) VALUES(?1, ?2, ?3) "
      "ON CONFLICT(category) DO UPDATE SET payload_json = excluded.payload_json, updated_at = excluded.updated_at",
      -1, &stmt, nullptr);
  const auto now = utc_now();
  sqlite3_bind_text(stmt, 1, category.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 2, payload_json.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 3, now.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_step(stmt);
  sqlite3_finalize(stmt);
}

std::string baseline_json(const std::unordered_map<std::string, std::pair<std::string, std::string>>& snapshot) {
  std::ostringstream out;
  out << "{";
  bool first = true;
  for (const auto& [path, meta] : snapshot) {
    if (!first) {
      out << ",";
    }
    first = false;
    out << json_escape(path) << ":{\"sha256\":" << json_escape(meta.first)
        << ",\"mtime\":" << json_escape(meta.second) << "}";
  }
  out << "}";
  return out.str();
}

std::unordered_map<std::string, std::pair<std::string, std::string>> load_launchd_baseline(sqlite3* db) {
  std::unordered_map<std::string, std::pair<std::string, std::string>> result;
  sqlite3_stmt* stmt = nullptr;
  sqlite3_prepare_v2(db, "SELECT payload_json FROM baselines WHERE category = 'launchd'", -1, &stmt, nullptr);
  if (sqlite3_step(stmt) == SQLITE_ROW) {
    const auto raw = default_string(reinterpret_cast<const char*>(sqlite3_column_text(stmt, 0)));
    size_t pos = 0;
    while ((pos = raw.find('"', pos)) != std::string::npos) {
      const size_t end = raw.find('"', pos + 1);
      if (end == std::string::npos) break;
      const auto key = raw.substr(pos + 1, end - pos - 1);
      const auto sha_pos = raw.find("\"sha256\":\"", end);
      const auto sha_end = raw.find('"', sha_pos + 10);
      const auto mtime_pos = raw.find("\"mtime\":\"", sha_end);
      const auto mtime_end = raw.find('"', mtime_pos + 9);
      if (sha_pos == std::string::npos || sha_end == std::string::npos || mtime_pos == std::string::npos ||
          mtime_end == std::string::npos) {
        break;
      }
      result[key] = {raw.substr(sha_pos + 10, sha_end - (sha_pos + 10)),
                     raw.substr(mtime_pos + 9, mtime_end - (mtime_pos + 9))};
      pos = mtime_end + 1;
    }
  }
  sqlite3_finalize(stmt);
  return result;
}

int begin_run(sqlite3* db) {
  sqlite3_stmt* stmt = nullptr;
  sqlite3_prepare_v2(db, "INSERT INTO scan_runs(started_at, status) VALUES(?1, 'running')", -1, &stmt, nullptr);
  const auto now = utc_now();
  sqlite3_bind_text(stmt, 1, now.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_step(stmt);
  sqlite3_finalize(stmt);
  return static_cast<int>(sqlite3_last_insert_rowid(db));
}

void finish_run(sqlite3* db, int run_id, const std::string& status, const std::string& error = "") {
  sqlite3_stmt* stmt = nullptr;
  sqlite3_prepare_v2(db, "UPDATE scan_runs SET finished_at = ?1, status = ?2, error = ?3 WHERE id = ?4", -1,
                     &stmt, nullptr);
  const auto now = utc_now();
  sqlite3_bind_text(stmt, 1, now.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 2, status.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_text(stmt, 3, error.c_str(), -1, SQLITE_TRANSIENT);
  sqlite3_bind_int(stmt, 4, run_id);
  sqlite3_step(stmt);
  sqlite3_finalize(stmt);
}

void scan_once(const fs::path& home, const fs::path& policy_path) {
  fs::create_directories(home);
  const auto db_path = home / kDefaultDb;
  const auto log_path = home / kDefaultLog;
  SqliteDb db(db_path);
  init_schema(db);

  const auto policy = load_policy(policy_path);
  const int run_id = begin_run(db.get());
  try {
    const auto launchd_current = collect_launchd_snapshot(policy);
    const auto launchd_baseline = load_launchd_baseline(db.get());
    for (const auto& [path, meta] : launchd_current) {
      if (!launchd_baseline.count(path)) {
        record_finding(db.get(), log_path, run_id, "launchd", "warning", path,
                       "{\"change\":\"added\",\"sha256\":" + json_escape(meta.first) +
                           ",\"mtime\":" + json_escape(meta.second) + "}");
      }
    }
    for (const auto& [path, meta] : launchd_baseline) {
      if (!launchd_current.count(path)) {
        record_finding(db.get(), log_path, run_id, "launchd", "critical", path,
                       "{\"change\":\"removed\",\"sha256\":" + json_escape(meta.first) +
                           ",\"mtime\":" + json_escape(meta.second) + "}");
      }
    }
    save_baseline(db.get(), "launchd", baseline_json(launchd_current));

    const auto samples = collect_process_samples(policy);
    const auto aggregates = aggregate_processes(samples);
    const auto ts = utc_now();
    for (const auto& sample : samples) {
      insert_process_sample(db.get(), run_id, sample, ts);
    }
    for (const auto& [subject, aggregate] : aggregates) {
      const auto existing = load_profile(db.get(), subject);
      const double cpu_pct = cpu_pct_from_delta(existing, aggregate, ts);
      if (existing.seen && existing.sample_count >= policy.min_profile_samples) {
        ContextCounts contexts;
        contexts.parent_seen_count =
            load_context_count(db.get(), "parent", subject, aggregate.parent_comm);
        contexts.signer_seen_count =
            load_context_count(db.get(), "signer", subject, aggregate.signer);
        const auto decision = score_anomaly(policy, aggregate, existing, cpu_pct, contexts);
        if (!decision.reasons.empty() &&
            decision.effective_score >= policy.log_score_threshold) {
          std::ostringstream payload;
          payload << "{\"reasons\":[";
          for (size_t i = 0; i < decision.reasons.size(); ++i) {
            if (i) payload << ",";
            payload << "{\"name\":" << json_escape(decision.reasons[i].name)
                    << ",\"weight\":" << std::fixed << std::setprecision(2)
                    << decision.reasons[i].weight << "}";
          }
          payload << "],\"cpu_pct\":" << std::fixed << std::setprecision(4) << cpu_pct
                  << ",\"rss_kb\":" << aggregate.rss_kb
                  << ",\"thread_count\":" << aggregate.thread_count
                  << ",\"listener_count\":" << aggregate.listener_count
                  << ",\"network_socket_count\":" << aggregate.network_socket_count
                  << ",\"process_count\":" << aggregate.process_count
                  << ",\"parent_comm\":" << json_escape(aggregate.parent_comm)
                  << ",\"signer\":" << json_escape(aggregate.signer)
                  << ",\"trusted\":" << (aggregate.trusted ? "true" : "false")
                  << ",\"apple\":" << (aggregate.apple ? "true" : "false")
                  << ",\"parent_seen_count\":" << contexts.parent_seen_count
                  << ",\"signer_seen_count\":" << contexts.signer_seen_count
                  << ",\"baseline_signer\":" << json_escape(existing.baseline_signer)
                  << ",\"baseline_parent_comm\":" << json_escape(existing.baseline_parent_comm)
                  << ",\"raw_score\":" << std::fixed << std::setprecision(2) << decision.raw_score
                  << ",\"threshold_offset\":" << std::fixed << std::setprecision(2) << decision.threshold_offset
                  << ",\"effective_score\":" << std::fixed << std::setprecision(2) << decision.effective_score
                  << ",\"recommended_action\":" << json_escape(decision.recommended_action) << "}";
          record_finding(db.get(), log_path, run_id, "process_anomaly", decision.severity, subject, payload.str());
          enforce_decision(db.get(), log_path, run_id, policy, subject, subject_pids(samples, subject), decision);
        }
      }
      save_profile(db.get(), aggregate, cpu_pct, ts);
      save_context_observation(db.get(), "parent", subject, aggregate.parent_comm, ts);
      save_context_observation(db.get(), "signer", subject, aggregate.signer, ts);
    }

    finish_run(db.get(), run_id, "ok");
  } catch (const std::exception& ex) {
    finish_run(db.get(), run_id, "error", ex.what());
    throw;
  }
}

void daemon_mode(const fs::path& home, const fs::path& policy_path) {
  const auto policy = load_policy(policy_path);
  const int interval = std::max(30, std::min(policy.listener_interval_seconds, policy.launchd_interval_seconds));
  while (true) {
    scan_once(home, policy_path);
    std::this_thread::sleep_for(std::chrono::seconds(interval));
  }
}

}  // namespace

Policy load_policy(const fs::path& path) {
  Policy policy;
  std::ifstream in(path);
  if (!in) {
    throw std::runtime_error("unable to open policy file: " + path.string());
  }

  std::string line;
  std::string current_array;
  auto* current_vector = static_cast<std::vector<std::string>*>(nullptr);
  while (std::getline(in, line)) {
    auto comment = line.find('#');
    if (comment != std::string::npos) {
      line = line.substr(0, comment);
    }
    auto trim = [](std::string value) {
      const auto start = value.find_first_not_of(" \t\r\n");
      if (start == std::string::npos) return std::string();
      const auto end = value.find_last_not_of(" \t\r\n");
      return value.substr(start, end - start + 1);
    };
    line = trim(line);
    if (line.empty() || line == "[guard]") {
      continue;
    }
    if (!current_array.empty()) {
      if (line == "]") {
        current_array.clear();
        current_vector = nullptr;
        continue;
      }
      if (current_vector != nullptr) {
        auto value = trim(line);
        if (!value.empty() && value.back() == ',') value.pop_back();
        value = trim(value);
        if (value.size() >= 2 && value.front() == '"' && value.back() == '"') {
          value = value.substr(1, value.size() - 2);
        }
        current_vector->push_back(value);
      }
      continue;
    }

    const auto eq = line.find('=');
    if (eq == std::string::npos) {
      continue;
    }
    const auto key = trim(line.substr(0, eq));
    auto value = trim(line.substr(eq + 1));
    if (value == "[") {
      current_array = key;
      if (key == "trusted_path_prefixes") current_vector = &policy.trusted_path_prefixes;
      else if (key == "trusted_process_names") current_vector = &policy.trusted_process_names;
      else if (key == "protected_launchd_paths") current_vector = &policy.protected_launchd_paths;
      else if (key == "actionable_path_prefixes") current_vector = &policy.actionable_path_prefixes;
      else if (key == "actionable_process_names") current_vector = &policy.actionable_process_names;
      continue;
    }
    if (value == "true" || value == "false") {
      const bool parsed = value == "true";
      if (key == "enforce") policy.enforce = parsed;
      continue;
    }
    if (value.find('.') != std::string::npos) {
      const double parsed = std::stod(value);
      if (key == "cpu_z_threshold") policy.cpu_z_threshold = parsed;
      else if (key == "rss_z_threshold") policy.rss_z_threshold = parsed;
      else if (key == "thread_z_threshold") policy.thread_z_threshold = parsed;
      else if (key == "listener_z_threshold") policy.listener_z_threshold = parsed;
      else if (key == "zero_variance_spike_factor") policy.zero_variance_spike_factor = parsed;
      else if (key == "trusted_subject_threshold_delta") policy.trusted_subject_threshold_delta = parsed;
      else if (key == "apple_subject_threshold_delta") policy.apple_subject_threshold_delta = parsed;
      else if (key == "actionable_score_boost") policy.actionable_score_boost = parsed;
      else if (key == "untrusted_score_boost") policy.untrusted_score_boost = parsed;
      else if (key == "cpu_spike_score") policy.cpu_spike_score = parsed;
      else if (key == "rss_spike_score") policy.rss_spike_score = parsed;
      else if (key == "thread_spike_score") policy.thread_spike_score = parsed;
      else if (key == "new_listener_score") policy.new_listener_score = parsed;
      else if (key == "listener_spike_score") policy.listener_spike_score = parsed;
      else if (key == "signer_drift_score") policy.signer_drift_score = parsed;
      else if (key == "parent_drift_score") policy.parent_drift_score = parsed;
      else if (key == "log_score_threshold") policy.log_score_threshold = parsed;
      else if (key == "alert_score_threshold") policy.alert_score_threshold = parsed;
      else if (key == "contain_score_threshold") policy.contain_score_threshold = parsed;
      else if (key == "terminate_score_threshold") policy.terminate_score_threshold = parsed;
      continue;
    }
    if (!value.empty() && std::all_of(value.begin(), value.end(), [](unsigned char c) {
          return std::isdigit(c);
        })) {
      const int parsed = std::stoi(value);
      if (key == "listener_interval_seconds") policy.listener_interval_seconds = parsed;
      else if (key == "launchd_interval_seconds") policy.launchd_interval_seconds = parsed;
      else if (key == "min_profile_samples") policy.min_profile_samples = parsed;
    }
  }
  return policy;
}

SqliteDb::SqliteDb(const fs::path& path) {
  if (sqlite3_open(path.string().c_str(), &db_) != SQLITE_OK) {
    throw std::runtime_error("sqlite open failed");
  }
}

SqliteDb::~SqliteDb() {
  if (db_ != nullptr) {
    sqlite3_close(db_);
  }
}

void SqliteDb::exec(const std::string& sql) const {
  char* err = nullptr;
  if (sqlite3_exec(db_, sql.c_str(), nullptr, nullptr, &err) != SQLITE_OK) {
    const std::string message = err ? err : "sqlite exec failed";
    sqlite3_free(err);
    throw std::runtime_error(message);
  }
}

void init_schema(const SqliteDb& db) {
  db.exec(
      "CREATE TABLE IF NOT EXISTS scan_runs ("
      "id INTEGER PRIMARY KEY AUTOINCREMENT,"
      "started_at TEXT NOT NULL,"
      "finished_at TEXT,"
      "status TEXT NOT NULL,"
      "error TEXT);"
      "CREATE TABLE IF NOT EXISTS findings ("
      "id INTEGER PRIMARY KEY AUTOINCREMENT,"
      "run_id INTEGER NOT NULL,"
      "ts TEXT NOT NULL,"
      "category TEXT NOT NULL,"
      "severity TEXT NOT NULL,"
      "subject TEXT NOT NULL,"
      "payload_json TEXT NOT NULL);"
      "CREATE TABLE IF NOT EXISTS baselines ("
      "category TEXT PRIMARY KEY,"
      "payload_json TEXT NOT NULL,"
      "updated_at TEXT NOT NULL);"
      "CREATE TABLE IF NOT EXISTS process_samples ("
      "id INTEGER PRIMARY KEY AUTOINCREMENT,"
      "run_id INTEGER NOT NULL,"
      "ts TEXT NOT NULL,"
      "pid INTEGER NOT NULL,"
      "ppid INTEGER NOT NULL,"
      "uid INTEGER NOT NULL,"
      "comm TEXT NOT NULL,"
      "exe TEXT NOT NULL,"
      "command_line TEXT NOT NULL,"
      "parent_comm TEXT NOT NULL DEFAULT '',"
      "signer TEXT NOT NULL DEFAULT '',"
      "cpu_total_ns INTEGER NOT NULL,"
      "rss_kb INTEGER NOT NULL,"
      "vsz_kb INTEGER NOT NULL,"
      "thread_count INTEGER NOT NULL,"
      "listener_count INTEGER NOT NULL DEFAULT 0,"
      "network_socket_count INTEGER NOT NULL DEFAULT 0,"
      "trusted INTEGER NOT NULL,"
      "actionable INTEGER NOT NULL,"
      "apple INTEGER NOT NULL);"
      "CREATE TABLE IF NOT EXISTS process_profiles ("
      "subject TEXT PRIMARY KEY,"
      "sample_count INTEGER NOT NULL,"
      "avg_cpu_pct REAL NOT NULL,"
      "avg_rss_kb REAL NOT NULL,"
      "avg_thread_count REAL NOT NULL DEFAULT 0,"
      "avg_listener_count REAL NOT NULL DEFAULT 0,"
      "avg_network_socket_count REAL NOT NULL DEFAULT 0,"
      "cpu_m2 REAL NOT NULL DEFAULT 0,"
      "rss_m2 REAL NOT NULL DEFAULT 0,"
      "thread_m2 REAL NOT NULL DEFAULT 0,"
      "listener_m2 REAL NOT NULL DEFAULT 0,"
      "network_socket_m2 REAL NOT NULL DEFAULT 0,"
      "max_cpu_pct REAL NOT NULL,"
      "max_rss_kb INTEGER NOT NULL,"
      "max_thread_count INTEGER NOT NULL DEFAULT 0,"
      "max_listener_count INTEGER NOT NULL DEFAULT 0,"
      "max_network_socket_count INTEGER NOT NULL DEFAULT 0,"
      "last_total_cpu_ns INTEGER NOT NULL,"
      "first_seen_at TEXT NOT NULL DEFAULT '',"
      "last_seen_at TEXT NOT NULL DEFAULT '',"
      "baseline_signer TEXT NOT NULL DEFAULT '',"
      "baseline_parent_comm TEXT NOT NULL DEFAULT '',"
      "profile_json TEXT NOT NULL DEFAULT '{}');"
      "CREATE TABLE IF NOT EXISTS process_context_observations ("
      "kind TEXT NOT NULL,"
      "subject TEXT NOT NULL,"
      "value TEXT NOT NULL,"
      "sample_count INTEGER NOT NULL DEFAULT 0,"
      "first_seen_at TEXT NOT NULL DEFAULT '',"
      "last_seen_at TEXT NOT NULL DEFAULT '',"
      "PRIMARY KEY(kind, subject, value));");

  try {
    db.exec("ALTER TABLE process_samples ADD COLUMN parent_comm TEXT NOT NULL DEFAULT '';");
  } catch (const std::exception&) {
  }
  try {
    db.exec("ALTER TABLE process_samples ADD COLUMN signer TEXT NOT NULL DEFAULT '';");
  } catch (const std::exception&) {
  }
  try {
    db.exec("ALTER TABLE process_samples ADD COLUMN listener_count INTEGER NOT NULL DEFAULT 0;");
  } catch (const std::exception&) {
  }
  try {
    db.exec("ALTER TABLE process_samples ADD COLUMN network_socket_count INTEGER NOT NULL DEFAULT 0;");
  } catch (const std::exception&) {
  }
  try {
    db.exec("ALTER TABLE process_profiles ADD COLUMN max_listener_count INTEGER NOT NULL DEFAULT 0;");
  } catch (const std::exception&) {
  }
  try {
    db.exec("ALTER TABLE process_profiles ADD COLUMN avg_thread_count REAL NOT NULL DEFAULT 0;");
  } catch (const std::exception&) {
  }
  try {
    db.exec("ALTER TABLE process_profiles ADD COLUMN avg_listener_count REAL NOT NULL DEFAULT 0;");
  } catch (const std::exception&) {
  }
  try {
    db.exec("ALTER TABLE process_profiles ADD COLUMN avg_network_socket_count REAL NOT NULL DEFAULT 0;");
  } catch (const std::exception&) {
  }
  try {
    db.exec("ALTER TABLE process_profiles ADD COLUMN cpu_m2 REAL NOT NULL DEFAULT 0;");
  } catch (const std::exception&) {
  }
  try {
    db.exec("ALTER TABLE process_profiles ADD COLUMN rss_m2 REAL NOT NULL DEFAULT 0;");
  } catch (const std::exception&) {
  }
  try {
    db.exec("ALTER TABLE process_profiles ADD COLUMN thread_m2 REAL NOT NULL DEFAULT 0;");
  } catch (const std::exception&) {
  }
  try {
    db.exec("ALTER TABLE process_profiles ADD COLUMN listener_m2 REAL NOT NULL DEFAULT 0;");
  } catch (const std::exception&) {
  }
  try {
    db.exec("ALTER TABLE process_profiles ADD COLUMN network_socket_m2 REAL NOT NULL DEFAULT 0;");
  } catch (const std::exception&) {
  }
  try {
    db.exec("ALTER TABLE process_profiles ADD COLUMN max_thread_count INTEGER NOT NULL DEFAULT 0;");
  } catch (const std::exception&) {
  }
  try {
    db.exec("ALTER TABLE process_profiles ADD COLUMN max_network_socket_count INTEGER NOT NULL DEFAULT 0;");
  } catch (const std::exception&) {
  }
  try {
    db.exec("ALTER TABLE process_profiles ADD COLUMN baseline_signer TEXT NOT NULL DEFAULT '';");
  } catch (const std::exception&) {
  }
  try {
    db.exec("ALTER TABLE process_profiles ADD COLUMN baseline_parent_comm TEXT NOT NULL DEFAULT '';");
  } catch (const std::exception&) {
  }
}

}  // namespace rr::guard

int main(int argc, char** argv) {
  using namespace rr::guard;

  std::string command = "scan-once";
  fs::path home = kDefaultHome;
  fs::path policy = kDefaultPolicy;

  for (int i = 1; i < argc; ++i) {
    const std::string arg = argv[i];
    if (arg == "scan-once" || arg == "daemon") {
      command = arg;
    } else if (arg == "--home" && i + 1 < argc) {
      home = argv[++i];
    } else if (arg == "--policy" && i + 1 < argc) {
      policy = argv[++i];
    } else {
      std::cerr << "unknown argument: " << arg << "\n";
      return 64;
    }
  }

  try {
    if (command == "daemon") {
      daemon_mode(home, policy);
    } else {
      scan_once(home, policy);
    }
    return 0;
  } catch (const std::exception& ex) {
    std::cerr << ex.what() << "\n";
    return 1;
  }
}
