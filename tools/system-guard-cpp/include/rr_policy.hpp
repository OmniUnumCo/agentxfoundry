#pragma once

#include <filesystem>
#include <string>
#include <unordered_map>
#include <vector>

namespace rr::guard {

struct Policy {
  int listener_interval_seconds = 60;
  int launchd_interval_seconds = 300;
  int min_profile_samples = 5;
  bool enforce = false;
  double cpu_z_threshold = 3.5;
  double rss_z_threshold = 3.5;
  double thread_z_threshold = 3.0;
  double listener_z_threshold = 3.0;
  double network_z_threshold = 3.0;
  double zero_variance_spike_factor = 1.5;
  double trusted_subject_threshold_delta = 1.0;
  double apple_subject_threshold_delta = 1.5;
  double actionable_score_boost = 1.0;
  double untrusted_score_boost = 0.75;
  double cpu_spike_score = 2.0;
  double rss_spike_score = 1.5;
  double thread_spike_score = 1.25;
  double new_listener_score = 2.5;
  double listener_spike_score = 2.0;
  double new_network_activity_score = 1.75;
  double network_activity_spike_score = 1.75;
  double signer_drift_score = 3.0;
  double parent_drift_score = 1.5;
  double log_score_threshold = 2.5;
  double alert_score_threshold = 4.5;
  double contain_score_threshold = 6.5;
  double terminate_score_threshold = 8.5;
  std::vector<std::string> trusted_path_prefixes;
  std::vector<std::string> trusted_process_names;
  std::vector<std::string> protected_launchd_paths;
  std::vector<std::string> actionable_path_prefixes;
  std::vector<std::string> actionable_process_names;
};

Policy load_policy(const std::filesystem::path& path);

}  // namespace rr::guard
