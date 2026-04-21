//! Structured JSONL logger — append-only, thread-safe.
//!
//! Every daemon event is appended as a single JSON object per line. This
//! format is grep-able, jq-able, and trivial to replay.

use chrono::Utc;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

/// Append-only JSONL writer with a mutex for thread safety.
pub struct JsonlLogger {
    path: PathBuf,
    lock: Mutex<()>,
}

impl JsonlLogger {
    pub fn new(path: PathBuf) -> std::io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(Self {
            path,
            lock: Mutex::new(()),
        })
    }

    pub fn append(&self, event: &HashMap<String, serde_json::Value>) {
        let _guard = self.lock.lock().unwrap();
        let mut payload = event.clone();
        payload.insert(
            "ts".to_string(),
            serde_json::Value::String(Utc::now().to_rfc3339()),
        );

        let line = serde_json::to_string(&payload).unwrap_or_default();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .expect("failed to open log file");

        let _ = writeln!(file, "{}", line);
    }
}

impl crate::traits::EventLogger for JsonlLogger {
    fn log(&self, event: &HashMap<String, serde_json::Value>) {
        self.append(event);
    }
}
