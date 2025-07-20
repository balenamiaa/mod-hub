use crate::ffi::FFIBridge;
use crate::hooks::HookManager;
use crate::logging::Logger;
use crate::memory::MemoryManager;
use crate::pointers::PointerManager;
use crate::python_runtime::PyO3Runtime;
use crate::registers::RegisterManager;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;

/// Global flag to signal hot reload requests from the F5 monitoring thread
static HOT_RELOAD_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Core error types for the Universe framework
#[derive(Debug)]
pub enum UniverseError {
    InitializationFailed(String),
    PythonError(String),
    MemoryError(String),
    HookError(String),
    SystemError(String),
    FFIError(String),
    RegisterError(String),
    PointerError(String),
    LoggingError(String),
}

impl std::fmt::Display for UniverseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UniverseError::InitializationFailed(msg) => write!(f, "Initialization failed: {}", msg),
            UniverseError::PythonError(msg) => write!(f, "Python error: {}", msg),
            UniverseError::MemoryError(msg) => write!(f, "Memory error: {}", msg),
            UniverseError::HookError(msg) => write!(f, "Hook error: {}", msg),
            UniverseError::SystemError(msg) => write!(f, "System error: {}", msg),
            UniverseError::FFIError(msg) => write!(f, "FFI error: {}", msg),
            UniverseError::RegisterError(msg) => write!(f, "Register error: {}", msg),
            UniverseError::PointerError(msg) => write!(f, "Pointer error: {}", msg),
            UniverseError::LoggingError(msg) => write!(f, "Logging error: {}", msg),
        }
    }
}

impl std::error::Error for UniverseError {}

impl UniverseError {
    /// Check if this error is recoverable (non-fatal)
    pub fn is_recoverable(&self) -> bool {
        match self {
            UniverseError::InitializationFailed(_) => false,
            UniverseError::PythonError(_) => true,
            UniverseError::MemoryError(_) => true,
            UniverseError::HookError(_) => true,
            UniverseError::SystemError(_) => false,
            UniverseError::FFIError(_) => true,
            UniverseError::RegisterError(_) => true,
            UniverseError::PointerError(_) => true,
            UniverseError::LoggingError(_) => true,
        }
    }

    /// Get error category for logging purposes
    pub fn category(&self) -> &'static str {
        match self {
            UniverseError::InitializationFailed(_) => "INIT",
            UniverseError::PythonError(_) => "PYTHON",
            UniverseError::MemoryError(_) => "MEMORY",
            UniverseError::HookError(_) => "HOOK",
            UniverseError::SystemError(_) => "SYSTEM",
            UniverseError::FFIError(_) => "FFI",
            UniverseError::RegisterError(_) => "REGISTER",
            UniverseError::PointerError(_) => "POINTER",
            UniverseError::LoggingError(_) => "LOGGING",
        }
    }

    /// Get error severity level for logging
    pub fn severity(&self) -> &'static str {
        match self {
            UniverseError::InitializationFailed(_) => "CRITICAL",
            UniverseError::PythonError(_) => "ERROR",
            UniverseError::MemoryError(_) => "ERROR",
            UniverseError::HookError(_) => "ERROR",
            UniverseError::SystemError(_) => "CRITICAL",
            UniverseError::FFIError(_) => "ERROR",
            UniverseError::RegisterError(_) => "ERROR",
            UniverseError::PointerError(_) => "ERROR",
            UniverseError::LoggingError(_) => "WARN",
        }
    }

    /// Get detailed error context for debugging
    pub fn context(&self) -> String {
        match self {
            UniverseError::InitializationFailed(msg) => format!("During framework initialization: {}", msg),
            UniverseError::PythonError(msg) => format!("In Python runtime: {}", msg),
            UniverseError::MemoryError(msg) => format!("During memory operation: {}", msg),
            UniverseError::HookError(msg) => format!("In hook system: {}", msg),
            UniverseError::SystemError(msg) => format!("System-level error: {}", msg),
            UniverseError::FFIError(msg) => format!("In FFI bridge: {}", msg),
            UniverseError::RegisterError(msg) => format!("In register system: {}", msg),
            UniverseError::PointerError(msg) => format!("In pointer system: {}", msg),
            UniverseError::LoggingError(msg) => format!("In logging system: {}", msg),
        }
    }

    /// Create a Python exception from this error
    pub fn to_python_exception(&self) -> pyo3::PyErr {
        use pyo3::exceptions::*;
        match self {
            UniverseError::InitializationFailed(msg) => PyRuntimeError::new_err(format!("Initialization failed: {}", msg)),
            UniverseError::PythonError(msg) => PyRuntimeError::new_err(msg.clone()),
            UniverseError::MemoryError(msg) => PyMemoryError::new_err(msg.clone()),
            UniverseError::HookError(msg) => PyRuntimeError::new_err(format!("Hook error: {}", msg)),
            UniverseError::SystemError(msg) => PySystemError::new_err(msg.clone()),
            UniverseError::FFIError(msg) => PyRuntimeError::new_err(format!("FFI error: {}", msg)),
            UniverseError::RegisterError(msg) => PyRuntimeError::new_err(format!("Register error: {}", msg)),
            UniverseError::PointerError(msg) => PyRuntimeError::new_err(format!("Pointer error: {}", msg)),
            UniverseError::LoggingError(msg) => PyRuntimeError::new_err(format!("Logging error: {}", msg)),
        }
    }

    /// Create an error from a PyO3 error
    pub fn from_python_error(py_err: pyo3::PyErr) -> Self {
        UniverseError::PythonError(format!("Python error: {}", py_err))
    }

    /// Create an error from a Windows system error
    pub fn from_windows_error(error_code: u32, context: &str) -> Self {
        UniverseError::SystemError(format!("{}: Windows error code {}", context, error_code))
    }

    /// Create an error with additional context
    pub fn with_context(self, context: &str) -> Self {
        match self {
            UniverseError::InitializationFailed(msg) => UniverseError::InitializationFailed(format!("{}: {}", context, msg)),
            UniverseError::PythonError(msg) => UniverseError::PythonError(format!("{}: {}", context, msg)),
            UniverseError::MemoryError(msg) => UniverseError::MemoryError(format!("{}: {}", context, msg)),
            UniverseError::HookError(msg) => UniverseError::HookError(format!("{}: {}", context, msg)),
            UniverseError::SystemError(msg) => UniverseError::SystemError(format!("{}: {}", context, msg)),
            UniverseError::FFIError(msg) => UniverseError::FFIError(format!("{}: {}", context, msg)),
            UniverseError::RegisterError(msg) => UniverseError::RegisterError(format!("{}: {}", context, msg)),
            UniverseError::PointerError(msg) => UniverseError::PointerError(format!("{}: {}", context, msg)),
            UniverseError::LoggingError(msg) => UniverseError::LoggingError(format!("{}: {}", context, msg)),
        }
    }
}

/// Main Universe framework core that coordinates all subsystems
pub struct UniverseCore {
    python_runtime: Option<PyO3Runtime>,
    hook_manager: Option<HookManager>,
    memory_manager: Option<Arc<Mutex<MemoryManager>>>,
    ffi_bridge: Option<FFIBridge>,
    register_manager: RegisterManager,
    pointer_manager: Option<PointerManager>,
    logger: Arc<Logger>,
    hot_reload_thread: Option<thread::JoinHandle<()>>,
    shutdown_flag: Arc<Mutex<bool>>,
}

impl UniverseCore {
    /// Get a shared reference to the memory manager
    pub fn memory_manager(&self) -> Option<Arc<Mutex<MemoryManager>>> {
        self.memory_manager.as_ref().cloned()
    }

    /// Get a reference to the hook manager
    pub fn hook_manager(&self) -> Option<&HookManager> {
        self.hook_manager.as_ref()
    }

    /// Get a mutable reference to the hook manager
    pub fn hook_manager_mut(&mut self) -> Option<&mut HookManager> {
        self.hook_manager.as_mut()
    }

    /// Get a reference to the FFI bridge
    pub fn ffi_bridge(&self) -> Option<&FFIBridge> {
        self.ffi_bridge.as_ref()
    }

    /// Get a mutable reference to the FFI bridge
    pub fn ffi_bridge_mut(&mut self) -> Option<&mut FFIBridge> {
        self.ffi_bridge.as_mut()
    }

    /// Get a reference to the register manager
    pub fn register_manager(&self) -> &RegisterManager {
        &self.register_manager
    }

    /// Get a mutable reference to the register manager
    pub fn register_manager_mut(&mut self) -> &mut RegisterManager {
        &mut self.register_manager
    }

    /// Get a reference to the pointer manager
    pub fn pointer_manager(&self) -> Option<&PointerManager> {
        self.pointer_manager.as_ref()
    }

    /// Get a mutable reference to the pointer manager
    pub fn pointer_manager_mut(&mut self) -> Option<&mut PointerManager> {
        self.pointer_manager.as_mut()
    }

    /// Get a reference to the logger
    pub fn logger(&self) -> &Logger {
        &self.logger
    }

    /// Initialize the Universe framework core
    pub fn initialize() -> Result<Self, UniverseError> {
        // Initialize logger first so we can log initialization progress
        let logger = Logger::new().map_err(|e| {
            UniverseError::InitializationFailed(format!("Failed to initialize logger: {}", e))
        })?;

        // Initialize logger reference in hook handlers
        let logger_arc = Arc::new(logger);
        crate::hook_handlers::initialize_logger(Arc::clone(&logger_arc)).map_err(|e| {
            UniverseError::InitializationFailed(format!("Failed to initialize hook handlers logger: {}", e))
        })?;

        logger_arc.log("Initializing Universe framework...");

        // Initialize memory manager
        logger_arc.log("Initializing memory manager...");
        let memory_manager = MemoryManager::new().map_err(|e| {
            logger_arc.log(&format!("Failed to initialize memory manager: {}", e));
            UniverseError::InitializationFailed(format!(
                "Memory manager initialization failed: {}",
                e
            ))
        })?;

        // Create shared reference to memory manager for pointer system
        let shared_memory_manager = Arc::new(Mutex::new(memory_manager));

        // Initialize hook manager
        logger_arc.log("Initializing hook manager...");
        let hook_manager = HookManager::new().map_err(|e| {
            logger_arc.log(&format!("Failed to initialize hook manager: {}", e));
            UniverseError::InitializationFailed(format!(
                "Hook manager initialization failed: {}",
                e
            ))
        })?;

        // Initialize register manager
        logger_arc.log("Initializing register manager...");
        let register_manager = RegisterManager::new();

        // Initialize pointer manager
        logger_arc.log("Initializing pointer manager...");
        let pointer_manager = PointerManager::new(Arc::clone(&shared_memory_manager));

        // Initialize FFI bridge
        logger_arc.log("Initializing FFI bridge...");
        let ffi_bridge = FFIBridge::new().map_err(|e| {
            logger_arc.log(&format!("Failed to initialize FFI bridge: {}", e));
            UniverseError::InitializationFailed(format!("FFI bridge initialization failed: {}", e))
        })?;

        // Initialize Python runtime
        logger_arc.log("Initializing Python runtime...");
        let python_runtime = PyO3Runtime::new().map_err(|e| {
            logger_arc.log(&format!("Failed to initialize Python runtime: {}", e));
            UniverseError::InitializationFailed(format!(
                "Python runtime initialization failed: {}",
                e
            ))
        })?;

        // Try to execute universe.py if it exists
        logger_arc.log("Looking for universe.py in game directory...");
        match python_runtime.execute_universe_py() {
            Ok(()) => {
                logger_arc.log("Successfully executed universe.py");
            }
            Err(UniverseError::PythonError(msg)) => {
                if msg.contains("universe.py not found") {
                    logger_arc.log("universe.py not found - continuing without user script");
                } else {
                    logger_arc.log(&format!("Python execution error: {}", msg));
                }
            }
            Err(e) => {
                logger_arc.log(&format!("Error executing universe.py: {}", e));
            }
        }

        // Initialize shutdown flag for hot reload thread
        let shutdown_flag = Arc::new(Mutex::new(false));

        logger_arc.log("Universe framework initialization complete");

        let mut core = UniverseCore {
            python_runtime: Some(python_runtime),
            hook_manager: Some(hook_manager),
            memory_manager: Some(shared_memory_manager),
            ffi_bridge: Some(ffi_bridge),
            register_manager,
            pointer_manager: Some(pointer_manager),
            logger: logger_arc,
            hot_reload_thread: None,
            shutdown_flag: Arc::clone(&shutdown_flag),
        };

        // Start the hot reload monitoring thread
        core.start_hot_reload_thread()?;

        Ok(core)
    }

    /// Shutdown the Universe framework and cleanup resources
    pub fn shutdown(&mut self) -> Result<(), UniverseError> {
        self.logger.log("Shutting down Universe framework...");

        // Signal hot reload thread to shutdown
        if let Ok(mut flag) = self.shutdown_flag.lock() {
            *flag = true;
        }

        // Wait for hot reload thread to finish
        if let Some(thread_handle) = self.hot_reload_thread.take() {
            let _ = thread_handle.join(); // Ignore join errors during shutdown
        }

        // Cleanup in reverse order of initialization
        if let Some(mut hook_manager) = self.hook_manager.take() {
            hook_manager.cleanup()?;
        }

        if let Some(mut python_runtime) = self.python_runtime.take() {
            python_runtime.shutdown()?;
        }

        self.logger.log("Universe framework shutdown complete");
        Ok(())
    }

    /// Tick the Universe framework. Runs every 100ms.
    pub fn tick(&mut self) -> Result<(), UniverseError> {
        match self.check_and_handle_hot_reload() {
            Ok(()) => Ok(()),
            Err(e) if e.is_recoverable() => {
                // Try to recover from the error
                match self.handle_recoverable_error(&e) {
                    Ok(()) => Ok(()),
                    Err(recovery_error) => {
                        self.logger.log_critical(&format!("Failed to recover from error: {}", recovery_error));
                        Err(recovery_error)
                    }
                }
            }
            Err(e) => {
                self.logger.log_critical(&format!("Non-recoverable error during tick: {}", e));
                Err(e)
            }
        }
    }

    /// Start the hot reload monitoring thread
    fn start_hot_reload_thread(&mut self) -> Result<(), UniverseError> {
        let shutdown_flag = Arc::clone(&self.shutdown_flag);

        let thread_handle = thread::spawn(move || {
            let mut f5_pressed = false;

            loop {
                // Check if we should shutdown
                if let Ok(flag) = shutdown_flag.lock() {
                    if *flag {
                        break;
                    }
                }

                // Check F5 key state (VK_F5 = 0x74)
                unsafe {
                    let f5_state = GetAsyncKeyState(0x74);
                    let f5_currently_pressed = ((f5_state as u16) & 0x8000) != 0;

                    // Detect F5 key press (transition from not pressed to pressed)
                    if f5_currently_pressed && !f5_pressed {
                        // F5 was just pressed - trigger hot reload
                        // We need to communicate back to the main thread
                        // For now, we'll use a simple approach with a global flag
                        HOT_RELOAD_REQUESTED.store(true, std::sync::atomic::Ordering::Relaxed);
                    }

                    f5_pressed = f5_currently_pressed;
                }

                // Sleep for a short time to avoid busy waiting
                thread::sleep(Duration::from_millis(100));
            }
        });

        self.hot_reload_thread = Some(thread_handle);
        Ok(())
    }

    /// Handle hot reload triggered by F5 key
    pub fn handle_hot_reload(&mut self) -> Result<(), UniverseError> {
        self.logger
            .log("Hot reload triggered - starting hot reload process...");

        // Step 1: Clear universe.log file
        self.logger
            .clear()
            .map_err(|e| UniverseError::SystemError(format!("Failed to clear log file: {}", e)))?;

        self.logger.log("Hot reload: Log file cleared");

        // Step 2: Remove all active hooks
        if let Some(ref mut hook_manager) = self.hook_manager {
            hook_manager.remove_all_hooks().map_err(|e| {
                self.logger
                    .log(&format!("Failed to remove hooks during hot reload: {}", e));
                e
            })?;
            self.logger.log("Hot reload: All hooks removed");
        }

        // Step 3: Clear Python module cache and reload modules
        if let Some(ref mut python_runtime) = self.python_runtime {
            python_runtime.reload_modules().map_err(|e| {
                self.logger
                    .log(&format!("Failed to reload Python modules: {}", e));
                e
            })?;
            self.logger.log("Hot reload: Python modules reloaded");

            // Step 4: Re-execute universe.py
            match python_runtime.execute_universe_py() {
                Ok(()) => {
                    self.logger
                        .log("Hot reload: universe.py re-executed successfully");
                }
                Err(UniverseError::PythonError(msg)) => {
                    if msg.contains("universe.py not found") {
                        self.logger.log(
                            "Hot reload: universe.py not found - continuing without user script",
                        );
                    } else {
                        self.logger
                            .log(&format!("Hot reload: Python execution error: {}", msg));
                        return Err(UniverseError::PythonError(msg));
                    }
                }
                Err(e) => {
                    self.logger
                        .log(&format!("Hot reload: Error executing universe.py: {}", e));
                    return Err(e);
                }
            }
        }

        self.logger.log("Hot reload completed successfully");
        Ok(())
    }

    /// Check if hot reload was requested and handle it
    pub fn check_and_handle_hot_reload(&mut self) -> Result<(), UniverseError> {
        if HOT_RELOAD_REQUESTED.load(std::sync::atomic::Ordering::Relaxed) {
            HOT_RELOAD_REQUESTED.store(false, std::sync::atomic::Ordering::Relaxed);
            self.handle_hot_reload()?;
        }
        Ok(())
    }

    /// Handle a recoverable error with appropriate recovery action
    pub fn handle_recoverable_error(&mut self, error: &UniverseError) -> Result<(), UniverseError> {
        match error {
            UniverseError::PythonError(_) => {
                self.logger.log_recoverable_error(error, "Attempting to reinitialize Python runtime");
                // Try to reinitialize Python runtime
                match PyO3Runtime::new() {
                    Ok(new_runtime) => {
                        self.python_runtime = Some(new_runtime);
                        self.logger.log_info("Python runtime successfully reinitialized");
                        Ok(())
                    }
                    Err(e) => {
                        self.logger.log_error_with_context(&e, "Failed to reinitialize Python runtime");
                        Err(e)
                    }
                }
            }
            UniverseError::MemoryError(_) => {
                self.logger.log_recoverable_error(error, "Refreshing memory manager module list");
                // Try to refresh memory manager modules
                if let Some(ref memory_manager) = self.memory_manager {
                    if let Ok(mut manager) = memory_manager.lock() {
                        match manager.refresh_modules() {
                            Ok(()) => {
                                self.logger.log_info("Memory manager modules refreshed successfully");
                                Ok(())
                            }
                            Err(e) => {
                                self.logger.log_error_with_context(&e, "Failed to refresh memory manager modules");
                                Err(e)
                            }
                        }
                    } else {
                        let err = UniverseError::SystemError("Failed to acquire memory manager lock".to_string());
                        self.logger.log_error(&err);
                        Err(err)
                    }
                } else {
                    let err = UniverseError::SystemError("Memory manager not available".to_string());
                    self.logger.log_error(&err);
                    Err(err)
                }
            }
            UniverseError::HookError(_) => {
                self.logger.log_recoverable_error(error, "Clearing all hooks and resetting hook manager");
                // Try to clear all hooks and reset
                if let Some(ref mut hook_manager) = self.hook_manager {
                    match hook_manager.remove_all_hooks() {
                        Ok(()) => {
                            self.logger.log_info("All hooks cleared successfully");
                            Ok(())
                        }
                        Err(e) => {
                            self.logger.log_error_with_context(&e, "Failed to clear hooks during recovery");
                            Err(e)
                        }
                    }
                } else {
                    let err = UniverseError::SystemError("Hook manager not available".to_string());
                    self.logger.log_error(&err);
                    Err(err)
                }
            }
            _ => {
                // For other error types, just log and continue
                self.logger.log_recoverable_error(error, "No specific recovery action available");
                Ok(())
            }
        }
    }
}
