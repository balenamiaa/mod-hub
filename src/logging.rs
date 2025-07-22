use crate::core::UniverseError;
use std::fs::OpenOptions;
use std::io::{Write, Seek, SeekFrom};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Logging system for the Universe framework
pub struct Logger {
    log_file: Mutex<Option<std::fs::File>>,
}

impl Logger {
    /// Create a new logger instance
    pub fn new() -> Result<Self, UniverseError> {
        let log_path = "universe.log".to_string();
        
        // Try to create/open universe.log in the current directory
        let log_file = match OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_path)
        {
            Ok(file) => Some(file),
            Err(e) => {
                // We can't use the logger here since we're creating it
                // Fall back to stderr for this critical error
                let error_msg = format!("CRITICAL: Failed to create universe.log: {}\n", e);
                let _ = std::io::Write::write_all(&mut std::io::stderr(), error_msg.as_bytes());
                None // Fail gracefully if we can't create the log file
            }
        };

        Ok(Logger {
            log_file: Mutex::new(log_file),
        })
    }

    /// Log a message to the universe.log file
    pub fn log(&self, message: &str) {
        self.log_with_level("INFO", message);
    }

    /// Log a message with a specific level
    pub fn log_with_level(&self, level: &str, message: &str) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let log_entry = format!("[{}] [{}] {}\n", timestamp, level, message);

        // Try to write to file, but don't panic if it fails
        if let Ok(mut file_guard) = self.log_file.lock() {
            if let Some(ref mut file) = *file_guard {
                let _ = file.write_all(log_entry.as_bytes());
                let _ = file.flush();
            }
        }

        // Also output to stderr for debugging
        eprint!("{}", log_entry);
    }

    /// Clear the log file (used during hot reload)
    pub fn clear(&self) -> Result<(), UniverseError> {
        if let Ok(mut file_guard) = self.log_file.lock() {
            if let Some(ref mut file) = *file_guard {
                file.set_len(0).map_err(|e| {
                    UniverseError::SystemError(format!("Failed to clear log file: {}", e))
                })?;
                file.seek(SeekFrom::Start(0)).map_err(|e| {
                    UniverseError::SystemError(format!("Failed to seek to start of log file: {}", e))
                })?;
            }
        }
        Ok(())
    }

    /// Log an error with additional context
    pub fn log_error(&self, error: &UniverseError) {
        self.log_with_level(error.severity(), &format!("[{}] {}", error.category(), error.context()));
    }

    /// Log an error with custom context
    pub fn log_error_with_context(&self, error: &UniverseError, context: &str) {
        self.log_with_level(error.severity(), &format!("[{}] {} (Context: {})", error.category(), error.context(), context));
    }

    /// Log an info message
    pub fn log_info(&self, message: &str) {
        self.log_with_level("INFO", message);
    }

    /// Log a debug message
    pub fn log_debug(&self, message: &str) {
        self.log_with_level("DEBUG", message);
    }

    /// Log a critical error that may cause system instability
    pub fn log_critical(&self, message: &str) {
        self.log_with_level("CRITICAL", message);
    }

    /// Log a recoverable error with recovery action taken
    pub fn log_recoverable_error(&self, error: &UniverseError, recovery_action: &str) {
        self.log_with_level("RECOVER", &format!("[{}] {} (Recovery: {})", error.category(), error, recovery_action));
    }
}