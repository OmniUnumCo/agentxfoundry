#include <filesystem>
#include <iostream>
#include <sqlite3.h>
#include <stdexcept>
#include <string>
#include <vector>

namespace fs = std::filesystem;

namespace rr::guard {

class SqliteDb {
 public:
  explicit SqliteDb(const fs::path& path) {
    if (sqlite3_open(path.string().c_str(), &db_) != SQLITE_OK) {
      const std::string message =
          "sqlite open failed for " + path.string() + ": " +
          (db_ ? sqlite3_errmsg(db_) : "unknown error");
      if (db_ != nullptr) {
        sqlite3_close(db_);
        db_ = nullptr;
      }
      throw std::runtime_error(message);
    }
  }
  ~SqliteDb() {
    if (db_ != nullptr) {
      sqlite3_close(db_);
    }
  }
  sqlite3* get() const { return db_; }

 private:
  sqlite3* db_ = nullptr;
};

}  // namespace rr::guard

int main(int argc, char** argv) {
  fs::path home = "/var/db/rr-integrity";
  std::string command = "findings";
  int limit = 20;
  std::string subject;

  for (int i = 1; i < argc; ++i) {
    const std::string arg = argv[i];
    if (arg == "findings" || arg == "profiles" || arg == "runs" || arg == "anomalies") {
      command = arg;
    } else if (arg == "inspect" && i + 1 < argc) {
      command = arg;
      subject = argv[++i];
    } else if (arg == "--home" && i + 1 < argc) {
      home = argv[++i];
    } else if (arg == "--limit" && i + 1 < argc) {
      limit = std::stoi(argv[++i]);
    }
  }

  try {
    rr::guard::SqliteDb db(home / "findings.db");
    sqlite3_stmt* stmt = nullptr;
    if (command == "profiles") {
      sqlite3_prepare_v2(
          db.get(),
          "SELECT subject, sample_count, avg_cpu_pct, avg_rss_kb, avg_thread_count, avg_listener_count, "
          "avg_network_socket_count, max_cpu_pct, max_rss_kb, max_thread_count, max_listener_count, "
          "max_network_socket_count, baseline_signer, baseline_parent_comm, profile_json "
          "FROM process_profiles ORDER BY sample_count DESC LIMIT ?1",
          -1, &stmt, nullptr);
      sqlite3_bind_int(stmt, 1, limit);
      while (sqlite3_step(stmt) == SQLITE_ROW) {
        std::cout << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 0))
                  << " samples=" << sqlite3_column_int(stmt, 1)
                  << " avg_cpu=" << sqlite3_column_double(stmt, 2)
                  << " avg_rss_kb=" << sqlite3_column_double(stmt, 3)
                  << " avg_threads=" << sqlite3_column_double(stmt, 4)
                  << " avg_listeners=" << sqlite3_column_double(stmt, 5)
                  << " avg_network_sockets=" << sqlite3_column_double(stmt, 6)
                  << " max_cpu=" << sqlite3_column_double(stmt, 7)
                  << " max_rss_kb=" << sqlite3_column_int64(stmt, 8)
                  << " max_threads=" << sqlite3_column_int(stmt, 9)
                  << " max_listeners=" << sqlite3_column_int(stmt, 10)
                  << " max_network_sockets=" << sqlite3_column_int(stmt, 11)
                  << " baseline_signer=" << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 12))
                  << " baseline_parent=" << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 13))
                  << " meta=" << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 14)) << "\n";
      }
    } else if (command == "inspect") {
      sqlite3_stmt* resolve_stmt = nullptr;
      std::string resolved_subject = subject;
      sqlite3_prepare_v2(
          db.get(),
          "SELECT exe FROM process_samples WHERE exe = ?1 OR comm = ?1 "
          "ORDER BY id DESC LIMIT 1",
          -1, &resolve_stmt, nullptr);
      sqlite3_bind_text(resolve_stmt, 1, subject.c_str(), -1, SQLITE_TRANSIENT);
      if (sqlite3_step(resolve_stmt) == SQLITE_ROW) {
        resolved_subject =
            reinterpret_cast<const char*>(sqlite3_column_text(resolve_stmt, 0));
      }
      sqlite3_finalize(resolve_stmt);

      sqlite3_prepare_v2(
          db.get(),
          "SELECT subject, sample_count, avg_cpu_pct, avg_rss_kb, avg_thread_count, avg_listener_count, "
          "avg_network_socket_count, max_cpu_pct, max_rss_kb, max_thread_count, max_listener_count, "
          "max_network_socket_count, baseline_signer, "
          "baseline_parent_comm, profile_json "
          "FROM process_profiles WHERE subject = ?1",
          -1, &stmt, nullptr);
      sqlite3_bind_text(stmt, 1, resolved_subject.c_str(), -1, SQLITE_TRANSIENT);
      if (sqlite3_step(stmt) == SQLITE_ROW) {
        std::cout << "profile\n";
        std::cout << "subject=" << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 0)) << "\n";
        std::cout << "sample_count=" << sqlite3_column_int(stmt, 1) << "\n";
        std::cout << "avg_cpu=" << sqlite3_column_double(stmt, 2) << "\n";
        std::cout << "avg_rss_kb=" << sqlite3_column_double(stmt, 3) << "\n";
        std::cout << "avg_threads=" << sqlite3_column_double(stmt, 4) << "\n";
        std::cout << "avg_listeners=" << sqlite3_column_double(stmt, 5) << "\n";
        std::cout << "avg_network_sockets=" << sqlite3_column_double(stmt, 6) << "\n";
        std::cout << "max_cpu=" << sqlite3_column_double(stmt, 7) << "\n";
        std::cout << "max_rss_kb=" << sqlite3_column_int64(stmt, 8) << "\n";
        std::cout << "max_threads=" << sqlite3_column_int(stmt, 9) << "\n";
        std::cout << "max_listeners=" << sqlite3_column_int(stmt, 10) << "\n";
        std::cout << "max_network_sockets=" << sqlite3_column_int(stmt, 11) << "\n";
        std::cout << "baseline_signer=" << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 12)) << "\n";
        std::cout << "baseline_parent_comm=" << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 13)) << "\n";
        std::cout << "meta=" << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 14)) << "\n";
      } else {
        std::cout << "subject not found: " << subject << "\n";
      }
      sqlite3_finalize(stmt);
      stmt = nullptr;

      std::cout << "recent_samples\n";
      sqlite3_prepare_v2(
          db.get(),
          "SELECT ts, pid, comm, parent_comm, signer, rss_kb, listener_count, network_socket_count, trusted, actionable, apple "
          "FROM process_samples WHERE exe = ?1 OR comm = ?1 "
          "ORDER BY id DESC LIMIT ?2",
          -1, &stmt, nullptr);
      sqlite3_bind_text(stmt, 1, resolved_subject.c_str(), -1, SQLITE_TRANSIENT);
      sqlite3_bind_text(stmt, 2, subject.c_str(), -1, SQLITE_TRANSIENT);
      sqlite3_bind_int(stmt, 3, limit);
      while (sqlite3_step(stmt) == SQLITE_ROW) {
        std::cout
            << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 0))
            << " pid=" << sqlite3_column_int(stmt, 1)
            << " comm=" << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 2))
            << " parent=" << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 3))
            << " signer=" << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 4))
            << " rss_kb=" << sqlite3_column_int64(stmt, 5)
            << " listeners=" << sqlite3_column_int(stmt, 6)
            << " network_sockets=" << sqlite3_column_int(stmt, 7)
            << " trusted=" << sqlite3_column_int(stmt, 8)
            << " actionable=" << sqlite3_column_int(stmt, 9)
            << " apple=" << sqlite3_column_int(stmt, 10)
            << "\n";
      }
      sqlite3_finalize(stmt);
      stmt = nullptr;

      std::cout << "parent_contexts\n";
      sqlite3_prepare_v2(
          db.get(),
          "SELECT value, sample_count, last_seen_at "
          "FROM process_context_observations "
          "WHERE kind = 'parent' AND subject = ?1 "
          "ORDER BY sample_count DESC, value ASC LIMIT ?2",
          -1, &stmt, nullptr);
      sqlite3_bind_text(stmt, 1, resolved_subject.c_str(), -1, SQLITE_TRANSIENT);
      sqlite3_bind_int(stmt, 2, limit);
      while (sqlite3_step(stmt) == SQLITE_ROW) {
        std::cout << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 0))
                  << " samples=" << sqlite3_column_int(stmt, 1)
                  << " last_seen=" << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 2))
                  << "\n";
      }
      sqlite3_finalize(stmt);
      stmt = nullptr;

      std::cout << "signer_contexts\n";
      sqlite3_prepare_v2(
          db.get(),
          "SELECT value, sample_count, last_seen_at "
          "FROM process_context_observations "
          "WHERE kind = 'signer' AND subject = ?1 "
          "ORDER BY sample_count DESC, value ASC LIMIT ?2",
          -1, &stmt, nullptr);
      sqlite3_bind_text(stmt, 1, resolved_subject.c_str(), -1, SQLITE_TRANSIENT);
      sqlite3_bind_int(stmt, 2, limit);
      while (sqlite3_step(stmt) == SQLITE_ROW) {
        std::cout << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 0))
                  << " samples=" << sqlite3_column_int(stmt, 1)
                  << " last_seen=" << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 2))
                  << "\n";
      }
    } else if (command == "runs") {
      sqlite3_prepare_v2(
          db.get(),
          "SELECT id, started_at, finished_at, status, COALESCE(error, '') "
          "FROM scan_runs ORDER BY id DESC LIMIT ?1",
          -1, &stmt, nullptr);
      sqlite3_bind_int(stmt, 1, limit);
      while (sqlite3_step(stmt) == SQLITE_ROW) {
        std::cout << "#" << sqlite3_column_int(stmt, 0)
                  << " started=" << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 1))
                  << " finished=" << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 2))
                  << " status=" << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 3))
                  << " error=" << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 4))
                  << "\n";
      }
    } else if (command == "anomalies") {
      sqlite3_prepare_v2(
          db.get(),
          "SELECT ts, severity, subject, payload_json "
          "FROM findings WHERE category = 'process_anomaly' "
          "ORDER BY id DESC LIMIT ?1",
          -1, &stmt, nullptr);
      sqlite3_bind_int(stmt, 1, limit);
      while (sqlite3_step(stmt) == SQLITE_ROW) {
        std::cout << "[" << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 0)) << "] "
                  << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 1)) << " "
                  << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 2)) << "\n"
                  << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 3)) << "\n";
      }
    } else {
      sqlite3_prepare_v2(
          db.get(),
          "SELECT ts, category, severity, subject, payload_json "
          "FROM findings ORDER BY id DESC LIMIT ?1",
          -1, &stmt, nullptr);
      sqlite3_bind_int(stmt, 1, limit);
      while (sqlite3_step(stmt) == SQLITE_ROW) {
        std::cout << "[" << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 0)) << "] "
                  << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 2)) << " "
                  << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 1)) << " "
                  << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 3)) << "\n"
                  << reinterpret_cast<const char*>(sqlite3_column_text(stmt, 4)) << "\n";
      }
    }
    sqlite3_finalize(stmt);
    return 0;
  } catch (const std::exception& ex) {
    std::cerr << ex.what() << "\n";
    return 1;
  }
}
