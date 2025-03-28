use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::Mutex;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub level: String,     // "info", "warn", "error"
    pub tag: String,       // e.g., "emit", "stack", "memory", "governance"
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
    pub fn new<S: Into<String>>(level: S, tag: S, message: S) -> Self {
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
    
    pub fn info<S: Into<String>>(tag: S, message: S) -> Self {
        Self::new("info", tag, message)
    }
    
    pub fn warn<S: Into<String>>(tag: S, message: S) -> Self {
        Self::new("warn", tag, message)
    }
    
    pub fn error<S: Into<String>>(tag: S, message: S) -> Self {
        Self::new("error", tag, message)
    }
    
    pub fn emit(&self) -> io::Result<()> {
        let format = LOG_FORMAT.lock().unwrap();
        let log_file = LOG_FILE.lock().unwrap().clone();
        
        match *format {
            LogFormat::Pretty => self.emit_pretty(log_file),
            LogFormat::Json => self.emit_json(log_file),
        }
    }
    
    fn emit_pretty(&self, log_file: Option<String>) -> io::Result<()> {
        let level_color = match self.level.as_str() {
            "info" => "\x1b[32m", // Green
            "warn" => "\x1b[33m", // Yellow
            "error" => "\x1b[31m", // Red
            _ => "\x1b[0m",       // Default
        };
        
        let pretty_line = format!(
            "{}{} [{}] [{}] {}\x1b[0m", 
            level_color,
            self.timestamp.split('T').nth(1).unwrap_or(&self.timestamp).split('.').next().unwrap_or(""),
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
        let json = serde_json::to_string(&self).unwrap();
        
        // Always print to stdout
        println!("{}", json);
        
        // If log file is specified, write to it
        if let Some(file_path) = log_file {
            append_to_file(&file_path, &json)?;
        }
        
        Ok(())
    }
}

pub fn set_log_format(format: LogFormat) {
    let mut log_format = LOG_FORMAT.lock().unwrap();
    *log_format = format;
}

pub fn set_log_file(file_path: Option<String>) {
    let mut log_file = LOG_FILE.lock().unwrap();
    *log_file = file_path;
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
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    
    // Write content with newline
    writeln!(file, "{}", content)?;
    
    Ok(())
} 