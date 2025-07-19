use crate::core::UniverseError;
use pyo3::prelude::*;
// Removed unused import
use std::env;
use std::fs;
use std::path::Path;

/// Python runtime management using PyO3
pub struct PyO3Runtime {
    initialized: bool,
}

impl PyO3Runtime {
    /// Initialize the Python runtime
    pub fn new() -> Result<Self, UniverseError> {
        // Initialize Python with proper threading configuration
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            // Set up Python path to include the current directory (game directory)
            let sys = py.import("sys").map_err(|e| {
                UniverseError::PythonError(format!("Failed to import sys module: {}", e))
            })?;

            let path = sys.getattr("path").map_err(|e| {
                UniverseError::PythonError(format!("Failed to get sys.path: {}", e))
            })?;

            // Add current directory to Python path for module discovery
            let current_dir = env::current_dir().map_err(|e| {
                UniverseError::PythonError(format!("Failed to get current directory: {}", e))
            })?;

            let current_dir_str = current_dir.to_string_lossy();
            path.call_method1("insert", (0, current_dir_str.as_ref()))
                .map_err(|e| {
                    UniverseError::PythonError(format!(
                        "Failed to add current directory to sys.path: {}",
                        e
                    ))
                })?;

            // Register the universe module using the python_interface
            crate::python_interface::register_universe_module(py).map_err(|e| {
                UniverseError::PythonError(format!(
                    "Failed to register universe module: {}",
                    e
                ))
            })?;

            Ok::<(), UniverseError>(())
        })?;

        Ok(PyO3Runtime { initialized: true })
    }

    /// Execute the universe.py script from the game directory
    pub fn execute_universe_py(&self) -> Result<(), UniverseError> {
        if !self.initialized {
            return Err(UniverseError::PythonError(
                "Python runtime not initialized".to_string(),
            ));
        }

        // Check if universe.py exists in the current directory
        let universe_py_path = Path::new("universe.py");
        if !universe_py_path.exists() {
            return Err(UniverseError::PythonError(
                "universe.py not found in game directory".to_string(),
            ));
        }

        // Read the universe.py file
        let universe_py_content = fs::read_to_string(universe_py_path).map_err(|e| {
            UniverseError::PythonError(format!("Failed to read universe.py: {}", e))
        })?;

        // Execute the universe.py script
        Python::with_gil(|py| {
            // Execute the script directly with proper exception handling
            let code = std::ffi::CString::new(universe_py_content).map_err(|e| {
                UniverseError::PythonError(format!(
                    "Failed to convert Python code to CString: {}",
                    e
                ))
            })?;

            match py.run(&code, None, None) {
                Ok(_) => Ok(()),
                Err(py_err) => {
                    // Handle Python exception with detailed traceback
                    let error_details = self.handle_exception(py_err);
                    Err(UniverseError::PythonError(error_details))
                }
            }
        })?;

        Ok(())
    }

    /// Reload Python modules for hot reload functionality
    pub fn reload_modules(&mut self) -> Result<(), UniverseError> {
        if !self.initialized {
            return Err(UniverseError::PythonError(
                "Python runtime not initialized".to_string(),
            ));
        }

        Python::with_gil(|py| {
            // Clear the module cache to force reloading
            let sys = py.import("sys").map_err(|e| {
                UniverseError::PythonError(format!("Failed to import sys module: {}", e))
            })?;

            let modules = sys.getattr("modules").map_err(|e| {
                UniverseError::PythonError(format!("Failed to get sys.modules: {}", e))
            })?;

            // Get list of modules to remove (avoid modifying dict while iterating)
            let modules_to_remove: Vec<String> = modules
                .try_iter()
                .map_err(|e| {
                    UniverseError::PythonError(format!("Failed to iterate sys.modules: {}", e))
                })?
                .filter_map(|item| {
                    if let Ok(val) = item {
                        if let Ok(key_str) = val.extract::<String>() {
                            // Remove user modules but keep built-in ones
                            if !key_str.starts_with("__")
                                && !key_str.starts_with("sys")
                                && !key_str.starts_with("os")
                                && !key_str.starts_with("builtins")
                                && key_str != "universe"
                            {
                                Some(key_str)
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();

            // Remove the modules
            for module_name in modules_to_remove {
                let _ = modules.del_item(module_name); // Ignore errors
            }

            Ok::<(), UniverseError>(())
        })?;

        Ok(())
    }

    /// Handle Python exceptions and log them
    pub fn handle_exception(&self, error: PyErr) -> String {
        Python::with_gil(|py| {
            // Get the exception traceback
            let traceback = if let Some(traceback) = error.traceback(py) {
                match traceback.format() {
                    Ok(tb_str) => tb_str,
                    Err(_) => "Failed to format traceback".to_string(),
                }
            } else {
                "No traceback available".to_string()
            };

            // Format the error message
            let error_msg = format!("Python Exception: {}\nTraceback:\n{}", error, traceback);
            error_msg
        })
    }

    /// Shutdown the Python runtime
    pub fn shutdown(&mut self) -> Result<(), UniverseError> {
        if self.initialized {
            // PyO3 handles Python finalization automatically
            // We just need to mark as not initialized
            self.initialized = false;
        }
        Ok(())
    }
}


