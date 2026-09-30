use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use uuid::Uuid;

use crate::models::{now_ms, DiagnosticEntry};

pub struct Diagnostics {
    path: PathBuf,
    entries: Mutex<VecDeque<DiagnosticEntry>>,
    retention: Mutex<usize>,
}

impl Diagnostics {
    pub fn new(path: PathBuf, retention: usize) -> Self {
        let entries = fs::read_to_string(&path)
            .ok()
            .map(|content| {
                content
                    .lines()
                    .filter_map(|line| serde_json::from_str(line).ok())
                    .collect::<VecDeque<_>>()
            })
            .unwrap_or_default();
        Self {
            path,
            entries: Mutex::new(entries),
            retention: Mutex::new(retention.max(10)),
        }
    }

    pub fn set_retention(&self, retention: usize) {
        *self.retention.lock().unwrap() = retention.clamp(10, 2000);
        self.trim();
    }

    pub fn record(
        &self,
        level: &str,
        category: &str,
        message: impl Into<String>,
        command: Option<&str>,
    ) {
        let entry = DiagnosticEntry {
            id: Uuid::new_v4().to_string(),
            timestamp: now_ms(),
            level: level.to_string(),
            category: category.to_string(),
            message: sanitize(message.into()),
            command: command.map(ToOwned::to_owned),
        };
        {
            let mut entries = self.entries.lock().unwrap();
            entries.push_back(entry.clone());
        }
        self.trim();
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            if let Ok(line) = serde_json::to_string(&entry) {
                let _ = writeln!(file, "{line}");
            }
        }
    }

    pub fn list(&self) -> Vec<DiagnosticEntry> {
        self.entries.lock().unwrap().iter().cloned().rev().collect()
    }

    fn trim(&self) {
        let retention = *self.retention.lock().unwrap();
        let mut entries = self.entries.lock().unwrap();
        while entries.len() > retention {
            entries.pop_front();
        }
    }
}

fn sanitize(message: String) -> String {
    let mut message = message.replace(['\r', '\n'], " ");
    if message.len() > 500 {
        message.truncate(500);
    }
    message
}
