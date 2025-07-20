use crate::core::UniverseError;
use pyo3::types::PyTracebackMethods;
use std::fs::OpenOptions;
use std::io::{Write, Seek, SeekFrom};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

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
                // We can't use the logger here since we're creating it
                // Fall back to stderr for this critical error
                let error_msg = format!("CRITICAL: Failed to create universe.log: {}\n", e);
                let _ = std::io::Write::write_all(&mut std::io::stderr(), error_msg.as_bytes());
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
        self.log_with_level(error.severity(), &format!("[{}] {}", error.category(), error.context()));
    }

    /// Log an error with custom context
    pub fn log_error_with_context(&self, error: &UniverseError, context: &str) {
        self.log_with_level(error.severity(), &format!("[{}] {} (Context: {})", error.category(), error.context(), context));
    }

    /// Log a Python exception with full traceback
    pub fn log_python_exception(&self, py_err: &pyo3::PyErr) {
        pyo3::Python::with_gil(|py| {
            let traceback = py_err.traceback(py);
            let exception_str = py_err.to_string();
            
            if let Some(tb) = traceback {
                if let Ok(tb_str) = tb.format() {
                    self.log_with_level("ERROR", &format!("[PYTHON] Exception with traceback:\n{}\n{}", exception_str, tb_str));
                } else {
                    self.log_with_level("ERROR", &format!("[PYTHON] Exception: {}", exception_str));
                }
            } else {
                self.log_with_level("ERROR", &format!("[PYTHON] Exception: {}", exception_str));
            }
        });
    }

    /// Log a system operation with timing
    pub fn log_operation<T, F>(&self, operation_name: &str, operation: F) -> T
    where
        F: FnOnce() -> T,
    {
        let start_time = std::time::Instant::now();
        self.log_debug(&format!("Starting operation: {}", operation_name));
        
        let result = operation();
        
        let duration = start_time.elapsed();
        self.log_debug(&format!("Completed operation: {} (took {:?})", operation_name, duration));
        
        result
    }

    /// Log a system operation with error handling
    pub fn log_operation_with_error<T, E, F>(&self, operation_name: &str, operation: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::fmt::Display,
    {
        let start_time = std::time::Instant::now();
        self.log_debug(&format!("Starting operation: {}", operation_name));
        
        let result = operation();
        
        let duration = start_time.elapsed();
        match &result {
            Ok(_) => {
                self.log_debug(&format!("Completed operation: {} (took {:?})", operation_name, duration));
            }
            Err(e) => {
                self.log_with_level("ERROR", &format!("Failed operation: {} (took {:?}): {}", operation_name, duration, e));
            }
        }
        
        result
    }

    /// Log a warning message
    pub fn log_warning(&self, message: &str) {
        self.log_with_level("WARN", message);
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