#![allow(dead_code)] // Allow dead code during development

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub level: String, // "info", "warn", "error"
    pub tag: String,   // e.g., "emit", "stack", "memory", "governance"
    pub message: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Pretty,
    Json,
}

static LOG_FORMAT: Lazy<Mutex<LogFormat>> = Lazy::new(|| Mutex::new(LogFormat::Pretty));
static LOG_FILE: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));

impl Event {
    pub fn new<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
        level: S1,
        tag: S2,
        message: S3,
    ) -> Self {
        let now: DateTime<Utc> = Utc::now();

        Self {
            level: level.into(),
            tag: tag.into(),
            message: message.into(),
            timestamp: now.to_rfc3339(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn info<S1: Into<String>, S2: Into<String>>(tag: S1, message: S2) -> Self {
        Self::new("info", tag, message)
    }

    pub fn warn<S1: Into<String>, S2: Into<String>>(tag: S1, message: S2) -> Self {
        Self::new("warn", tag, message)
    }

    pub fn error<S1: Into<String>, S2: Into<String>>(tag: S1, message: S2) -> Self {
        Self::new("error", tag, message)
    }

    pub fn emit(&self) -> io::Result<()> {
        let format = LOG_FORMAT.lock()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to lock LOG_FORMAT: {:?}", e)))?;
        let log_file = LOG_FILE.lock()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to lock LOG_FILE: {:?}", e)))?
            .clone();

        match *format {
            LogFormat::Pretty => self.emit_pretty(log_file),
            LogFormat::Json => self.emit_json(log_file),
        }
    }

    fn emit_pretty(&self, log_file: Option<String>) -> io::Result<()> {
        let level_color = match self.level.as_str() {
            "info" => "\x1b[32m",  // Green
            "warn" => "\x1b[33m",  // Yellow
            "error" => "\x1b[31m", // Red
            _ => "\x1b[0m",        // Default
        };

        // Safely extract the time portion from the timestamp
        let time_str = self.timestamp
            .split('T')
            .nth(1)
            .unwrap_or(&self.timestamp)
            .split('.')
            .next()
            .unwrap_or("");

        let pretty_line = format!(
            "{}{} [{}] [{}] {}\x1b[0m",
            level_color,
            time_str,
            self.level.to_uppercase(),
            self.tag,
            self.message
        );

        // Always print to stdout
        println!("{}", pretty_line);

        // If log file is specified, write to it
        if let Some(file_path) = log_file {
            let plain_line = format!(
                "{} [{}] [{}] {}",
                self.timestamp,
                self.level.to_uppercase(),
                self.tag,
                self.message
            );

            append_to_file(&file_path, &plain_line)?;
        }

        Ok(())
    }

    fn emit_json(&self, log_file: Option<String>) -> io::Result<()> {
        let json = serde_json::to_string(&self)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to serialize event: {}", e)))?;

        // Always print to stdout
        println!("{}", json);

        // If log file is specified, write to it
        if let Some(file_path) = log_file {
            append_to_file(&file_path, &json)?;
        }

        Ok(())
    }
}

pub fn set_log_format(format: LogFormat) -> io::Result<()> {
    let mut log_format = LOG_FORMAT.lock()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to lock LOG_FORMAT: {:?}", e)))?;
    *log_format = format;
    Ok(())
}

pub fn set_log_file(file_path: Option<String>) -> io::Result<()> {
    let mut log_file = LOG_FILE.lock()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to lock LOG_FILE: {:?}", e)))?;
    *log_file = file_path;
    Ok(())
}

fn append_to_file(file_path: &str, content: &str) -> io::Result<()> {
    let path = Path::new(file_path);

    // Create parent directories if they don't exist
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }

    // Open file with append mode
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;

    // Write content with newline
    writeln!(file, "{}", content)?;

    Ok(())
}
