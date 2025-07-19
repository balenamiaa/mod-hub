use crate::ffi::FFIBridge;
use crate::hooks::HookManager;
use crate::logging::Logger;
use crate::memory::MemoryManager;
use crate::python_runtime::PyO3Runtime;
use crate::registers::RegisterManager;
use crate::pointers::PointerManager;
use std::sync::{Arc, Mutex};

/// Core error types for the Universe framework
#[derive(Debug)]
pub enum UniverseError {
    InitializationFailed(String),
    PythonError(String),
    MemoryError(String),
    HookError(String),
    SystemError(String),
}

impl std::fmt::Display for UniverseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UniverseError::InitializationFailed(msg) => write!(f, "Initialization failed: {}", msg),
            UniverseError::PythonError(msg) => write!(f, "Python error: {}", msg),
            UniverseError::MemoryError(msg) => write!(f, "Memory error: {}", msg),
            UniverseError::HookError(msg) => write!(f, "Hook error: {}", msg),
            UniverseError::SystemError(msg) => write!(f, "System error: {}", msg),
        }
    }
}

impl std::error::Error for UniverseError {}

/// Main Universe framework core that coordinates all subsystems
pub struct UniverseCore {
    python_runtime: Option<PyO3Runtime>,
    hook_manager: Option<HookManager>,
    memory_manager: Option<MemoryManager>,
    ffi_bridge: Option<FFIBridge>,
    register_manager: RegisterManager,
    pointer_manager: Option<PointerManager>,
    logger: Logger,
}

impl UniverseCore {
    /// Initialize the Universe framework core
    pub fn initialize() -> Result<Self, UniverseError> {
        // Initialize logger first so we can log initialization progress
        let logger = Logger::new().map_err(|e| {
            UniverseError::InitializationFailed(format!("Failed to initialize logger: {}", e))
        })?;
        
        logger.log("Initializing Universe framework...");

        // Initialize memory manager
        logger.log("Initializing memory manager...");
        let memory_manager = MemoryManager::new().map_err(|e| {
            logger.log(&format!("Failed to initialize memory manager: {}", e));
            UniverseError::InitializationFailed(format!("Memory manager initialization failed: {}", e))
        })?;

        // Create shared reference to memory manager for pointer system
        let shared_memory_manager = Arc::new(Mutex::new(memory_manager));

        // Initialize hook manager
        logger.log("Initializing hook manager...");
        let hook_manager = HookManager::new().map_err(|e| {
            logger.log(&format!("Failed to initialize hook manager: {}", e));
            UniverseError::InitializationFailed(format!("Hook manager initialization failed: {}", e))
        })?;

        // Initialize register manager
        logger.log("Initializing register manager...");
        let register_manager = RegisterManager::new();

        // Initialize pointer manager
        logger.log("Initializing pointer manager...");
        let pointer_manager = PointerManager::new(Arc::clone(&shared_memory_manager));

        // Initialize FFI bridge
        logger.log("Initializing FFI bridge...");
        let ffi_bridge = FFIBridge::new().map_err(|e| {
            logger.log(&format!("Failed to initialize FFI bridge: {}", e));
            UniverseError::InitializationFailed(format!("FFI bridge initialization failed: {}", e))
        })?;

        // Initialize Python runtime
        logger.log("Initializing Python runtime...");
        let python_runtime = PyO3Runtime::new().map_err(|e| {
            logger.log(&format!("Failed to initialize Python runtime: {}", e));
            UniverseError::InitializationFailed(format!("Python runtime initialization failed: {}", e))
        })?;

        // Try to execute universe.py if it exists
        logger.log("Looking for universe.py in game directory...");
        match python_runtime.execute_universe_py() {
            Ok(()) => {
                logger.log("Successfully executed universe.py");
            }
            Err(UniverseError::PythonError(msg)) => {
                if msg.contains("universe.py not found") {
                    logger.log("universe.py not found - continuing without user script");
                } else {
                    logger.log(&format!("Python execution error: {}", msg));
                }
            }
            Err(e) => {
                logger.log(&format!("Error executing universe.py: {}", e));
            }
        }

        logger.log("Universe framework initialization complete");

        let core = UniverseCore {
            python_runtime: Some(python_runtime),
            hook_manager: Some(hook_manager),
            memory_manager: None, // We're using shared_memory_manager now
            ffi_bridge: Some(ffi_bridge),
            register_manager,
            pointer_manager: Some(pointer_manager),
            logger,
        };

        Ok(core)
    }

    /// Shutdown the Universe framework and cleanup resources
    pub fn shutdown(&mut self) -> Result<(), UniverseError> {
        self.logger.log("Shutting down Universe framework...");

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

    /// Handle hot reload triggered by F5 key
    pub fn handle_hot_reload(&mut self) -> Result<(), UniverseError> {
        self.logger.log("Hot reload triggered...");
        // Implementation will be added in later tasks
        Ok(())
    }
}
