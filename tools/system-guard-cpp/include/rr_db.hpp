#pragma once

#include <sqlite3.h>

#include <filesystem>
#include <stdexcept>
#include <string>

namespace rr::guard {

class SqliteDb {
 public:
  explicit SqliteDb(const std::filesystem::path& path);
  ~SqliteDb();

  SqliteDb(const SqliteDb&) = delete;
  SqliteDb& operator=(const SqliteDb&) = delete;

  sqlite3* get() const { return db_; }
  void exec(const std::string& sql) const;

 private:
  sqlite3* db_ = nullptr;
};

void init_schema(const SqliteDb& db);

}  // namespace rr::guard
