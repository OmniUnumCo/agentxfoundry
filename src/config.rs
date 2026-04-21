//! Load the user-global Rational Reserve configuration.
//!
//! The config file lives at `~/.rational-reserve/config.toml` and is optional.

use serde::Deserialize;
use std::path::PathBuf;

const DEFAULT_HOME: &str = ".rational-reserve";

/// Filesystem locations for the RR runtime.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Paths {
    pub home: PathBuf,
    pub state_dir: PathBuf,
    pub run_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub doctrine_dir: PathBuf,
}

impl Default for Paths {
    fn default() -> Self {
        let home = home_dir().join(DEFAULT_HOME);
        Self {
            state_dir: home.join("state"),
            run_dir: home.join("run"),
            logs_dir: home.join("logs"),
            doctrine_dir: home.join("doctrine"),
            home,
        }
    }
}

impl Paths {
    pub fn db_path(&self) -> PathBuf {
        self.state_dir.join("rr.db")
    }

    pub fn pid_file(&self) -> PathBuf {
        self.run_dir.join("rr.pid")
    }

    pub fn socket_path(&self) -> PathBuf {
        self.run_dir.join("rr.sock")
    }

    pub fn log_file(&self) -> PathBuf {
        self.logs_dir.join("c2.jsonl")
    }

    /// Create all runtime directories if missing.
    pub fn ensure(&self) -> std::io::Result<()> {
        for d in [&self.state_dir, &self.run_dir, &self.logs_dir] {
            std::fs::create_dir_all(d)?;
        }
        Ok(())
    }
}

/// Top-level runtime config.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub paths: Paths,
}

/// Load config from the given path or the default location.
///
/// Returns the default `Config` when the file is missing.
pub fn load_config(config_path: Option<&PathBuf>) -> Config {
    let candidate = config_path.cloned().unwrap_or_else(default_config_path);
    if !candidate.is_file() {
        return Config::default();
    }

    let content = match std::fs::read_to_string(&candidate) {
        Ok(c) => c,
        Err(_) => return Config::default(),
    };

    let parsed: toml::Value = match toml::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Config::default(),
    };

    let Some(paths_table) = parsed.get("paths").and_then(|v| v.as_table()) else {
        return Config::default();
    };

    let home = paths_table
        .get("home")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(DEFAULT_HOME));

    let paths = Paths {
        state_dir: paths_table
            .get("state_dir")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("state")),
        run_dir: paths_table
            .get("run_dir")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("run")),
        logs_dir: paths_table
            .get("logs_dir")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("logs")),
        doctrine_dir: paths_table
            .get("doctrine_dir")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("doctrine")),
        home: home.clone(),
    };

    Config { paths }
}

fn default_config_path() -> PathBuf {
    home_dir().join(DEFAULT_HOME).join("config.toml")
}

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}
