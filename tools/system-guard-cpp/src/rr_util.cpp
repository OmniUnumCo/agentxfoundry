#include "rr_util.hpp"

#include <CommonCrypto/CommonDigest.h>

#include <chrono>
#include <fstream>
#include <iomanip>
#include <sstream>
#include <stdexcept>
#include <vector>

namespace rr::guard {

std::string utc_now() {
  const auto now = std::chrono::system_clock::now();
  const auto time = std::chrono::system_clock::to_time_t(now);
  std::tm tm{};
  gmtime_r(&time, &tm);
  std::ostringstream out;
  out << std::put_time(&tm, "%Y-%m-%dT%H:%M:%SZ");
  return out.str();
}

std::string json_escape(const std::string& input) {
  std::ostringstream out;
  out << '"';
  for (const char ch : input) {
    switch (ch) {
      case '\\': out << "\\\\"; break;
      case '"': out << "\\\""; break;
      case '\n': out << "\\n"; break;
      case '\r': out << "\\r"; break;
      case '\t': out << "\\t"; break;
      default: out << ch; break;
    }
  }
  out << '"';
  return out.str();
}

std::string sha256_file(const std::filesystem::path& path) {
  std::ifstream in(path, std::ios::binary);
  if (!in) {
    throw std::runtime_error("unable to open file for hashing: " + path.string());
  }

  CC_SHA256_CTX ctx;
  CC_SHA256_Init(&ctx);

  std::vector<char> buffer(1024 * 1024);
  while (in) {
    in.read(buffer.data(), static_cast<std::streamsize>(buffer.size()));
    const auto got = in.gcount();
    if (got > 0) {
      CC_SHA256_Update(&ctx, buffer.data(), static_cast<CC_LONG>(got));
    }
  }

  unsigned char digest[CC_SHA256_DIGEST_LENGTH];
  CC_SHA256_Final(digest, &ctx);

  std::ostringstream out;
  out << std::hex << std::setfill('0');
  for (unsigned char byte : digest) {
    out << std::setw(2) << static_cast<int>(byte);
  }
  return out.str();
}

bool starts_with(const std::string& input, const std::string& prefix) {
  return input.rfind(prefix, 0) == 0;
}

std::string read_text_file(const std::filesystem::path& path) {
  std::ifstream in(path);
  std::ostringstream out;
  out << in.rdbuf();
  return out.str();
}

}  // namespace rr::guard
