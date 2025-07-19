use crate::core::UniverseError;
use std::fs::OpenOptions;
use std::io::{Write, Seek, SeekFrom};
use std::sync::Mutex;

/// Logging system for the Universe framework
pub struct Logger {
    log_file: Mutex<Option<std::fs::File>>,
    log_path: String,
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
                eprintln!("Warning: Failed to create universe.log: {}", e);
                None // Fail gracefully if we can't create the log file
            }
        };

        Ok(Logger {
            log_file: Mutex::new(log_file),
            log_path,
        })
    }

    /// Log a message to the universe.log file
    pub fn log(&self, message: &str) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let log_entry = format!("[{}] {}\n", timestamp, message);

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

    /// Recreate the log file if it was lost or corrupted
    pub fn recreate_log_file(&self) -> Result<(), UniverseError> {
        if let Ok(mut file_guard) = self.log_file.lock() {
            match OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&self.log_path)
            {
                Ok(file) => {
                    *file_guard = Some(file);
                    Ok(())
                }
                Err(e) => Err(UniverseError::SystemError(format!(
                    "Failed to recreate log file: {}", e
                )))
            }
        } else {
            Err(UniverseError::SystemError(
                "Failed to acquire log file mutex".to_string()
            ))
        }
    }

    /// Log an error with additional context
    pub fn log_error(&self, error: &UniverseError) {
        self.log(&format!("ERROR: {}", error));
    }

    /// Log a warning message
    pub fn log_warning(&self, message: &str) {
        self.log(&format!("WARNING: {}", message));
    }

    /// Log an info message
    pub fn log_info(&self, message: &str) {
        self.log(&format!("INFO: {}", message));
    }
}