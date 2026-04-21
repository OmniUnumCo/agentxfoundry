#pragma once

#include <filesystem>
#include <string>

namespace rr::guard {

std::string utc_now();
std::string json_escape(const std::string& input);
std::string sha256_file(const std::filesystem::path& path);
bool starts_with(const std::string& input, const std::string& prefix);
std::string read_text_file(const std::filesystem::path& path);

}  // namespace rr::guard
