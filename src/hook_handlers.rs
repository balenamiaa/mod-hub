use crate::core::UniverseError;
use crate::hooks::HookManager;
use crate::logging::Logger;
use crate::registers::{RegisterState, WinContext};
use lazy_static::lazy_static;
use pyo3::prelude::*;
#[cfg(target_arch = "x86_64")]
use std::arch::global_asm;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};

// Global registry for hook callbacks
lazy_static! {
    static ref HOOK_REGISTRY: RwLock<HashMap<usize, PyObject>> = RwLock::new(HashMap::new());
    static ref JMPBACK_REGISTRY: RwLock<HashMap<usize, PyObject>> = RwLock::new(HashMap::new());
    static ref HOOK_MANAGER: Mutex<Option<Arc<Mutex<HookManager>>>> = Mutex::new(None);
    static ref LOGGER: Mutex<Option<Arc<Logger>>> = Mutex::new(None);
    static ref CURRENT_HOOK_ADDRESS: AtomicUsize = AtomicUsize::new(0);
}

/// Initialize the global hook manager reference
pub fn initialize_hook_manager(hook_manager: Arc<Mutex<HookManager>>) -> Result<(), UniverseError> {
    let mut manager = HOOK_MANAGER.lock().map_err(|_| {
        UniverseError::SystemError("Failed to acquire hook manager lock".to_string())
    })?;
    *manager = Some(hook_manager);
    Ok(())
}

/// Initialize the global logger reference
pub fn initialize_logger(logger: Arc<Logger>) -> Result<(), UniverseError> {
    let mut logger_ref = LOGGER.lock().map_err(|_| {
        UniverseError::SystemError("Failed to acquire logger lock".to_string())
    })?;
    *logger_ref = Some(logger);
    Ok(())
}

/// Get the logger for error reporting
fn get_logger() -> Option<Arc<Logger>> {
    if let Ok(logger_guard) = LOGGER.lock() {
        logger_guard.clone()
    } else {
        None
    }
}

/// Register a function hook callback
pub fn register_hook_callback(address: usize, callback: PyObject) -> Result<(), UniverseError> {
    let mut registry = HOOK_REGISTRY.write().map_err(|_| {
        UniverseError::HookError("Failed to acquire hook registry write lock".to_string())
    })?;

    registry.insert(address, callback);
    Ok(())
}

/// Register a jmpback hook callback
pub fn register_jmpback_callback(address: usize, callback: PyObject) -> Result<(), UniverseError> {
    let mut registry = JMPBACK_REGISTRY.write().map_err(|_| {
        UniverseError::HookError("Failed to acquire jmpback registry write lock".to_string())
    })?;

    registry.insert(address, callback);
    Ok(())
}

/// Remove a hook callback
pub fn remove_hook_callback(address: usize) -> Result<(), UniverseError> {
    let mut registry = HOOK_REGISTRY.write().map_err(|_| {
        UniverseError::HookError("Failed to acquire hook registry write lock".to_string())
    })?;

    registry.remove(&address);
    Ok(())
}

/// Remove a jmpback hook callback
pub fn remove_jmpback_callback(address: usize) -> Result<(), UniverseError> {
    let mut registry = JMPBACK_REGISTRY.write().map_err(|_| {
        UniverseError::HookError("Failed to acquire jmpback registry write lock".to_string())
    })?;

    registry.remove(&address);
    Ok(())
}

/// Clear all hook callbacks
pub fn clear_all_hook_callbacks() -> Result<(), UniverseError> {
    let mut registry = HOOK_REGISTRY.write().map_err(|_| {
        UniverseError::HookError("Failed to acquire hook registry write lock".to_string())
    })?;

    registry.clear();
    Ok(())
}

/// Clear all jmpback hook callbacks
pub fn clear_all_jmpback_callbacks() -> Result<(), UniverseError> {
    let mut registry = JMPBACK_REGISTRY.write().map_err(|_| {
        UniverseError::HookError("Failed to acquire jmpback registry write lock".to_string())
    })?;

    registry.clear();
    Ok(())
}

/// Get the hook manager
fn get_hook_manager() -> Result<Arc<Mutex<HookManager>>, UniverseError> {
    let manager = HOOK_MANAGER.lock().map_err(|_| {
        UniverseError::SystemError("Failed to acquire hook manager lock".to_string())
    })?;

    manager
        .clone()
        .ok_or_else(|| UniverseError::SystemError("Hook manager not initialized".to_string()))
}

/// Convert Windows CONTEXT to RegisterState
fn context_to_register_state(context: &WinContext) -> RegisterState {
    RegisterState::from_win_context(context)
}

/// Convert RegisterState to Windows CONTEXT
fn register_state_to_context(state: &RegisterState, context: &mut WinContext) {
    context.rax = state.rax;
    context.rbx = state.rbx;
    context.rcx = state.rcx;
    context.rdx = state.rdx;
    context.rsi = state.rsi;
    context.rdi = state.rdi;
    context.rsp = state.rsp;
    context.rbp = state.rbp;
    context.r8 = state.r8;
    context.r9 = state.r9;
    context.r10 = state.r10;
    context.r11 = state.r11;
    context.r12 = state.r12;
    context.r13 = state.r13;
    context.r14 = state.r14;
    context.r15 = state.r15;
    context.eflags = state.rflags as u32;

    // Copy XMM registers
    context.xmm0 = state.xmm[0];
    context.xmm1 = state.xmm[1];
    context.xmm2 = state.xmm[2];
    context.xmm3 = state.xmm[3];
    context.xmm4 = state.xmm[4];
    context.xmm5 = state.xmm[5];
    context.xmm6 = state.xmm[6];
    context.xmm7 = state.xmm[7];
    context.xmm8 = state.xmm[8];
    context.xmm9 = state.xmm[9];
    context.xmm10 = state.xmm[10];
    context.xmm11 = state.xmm[11];
    context.xmm12 = state.xmm[12];
    context.xmm13 = state.xmm[13];
    context.xmm14 = state.xmm[14];
    context.xmm15 = state.xmm[15];
}

/// Execute a function hook callback
///
/// This function is called from the assembly hook handler
#[no_mangle]
pub extern "C" fn execute_hook_callback(
    context: &mut WinContext,
    hook_address: usize,
    original_function: usize,
) -> i32 {
    // Store the current hook address for potential error handling
    CURRENT_HOOK_ADDRESS.store(hook_address, Ordering::SeqCst);

    // Convert Windows CONTEXT to RegisterState
    let register_state = context_to_register_state(context);

    // Get the callback from the registry - we need to clone it to avoid lifetime issues
    let callback = {
        // Scope for the registry guard to ensure it's dropped after we're done with it
        let registry_guard = match HOOK_REGISTRY.read() {
            Ok(guard) => guard,
            Err(_) => {
                if let Some(logger) = get_logger() {
                    logger.log_error(&UniverseError::HookError("Failed to acquire hook registry read lock".to_string()));
                }
                return -1;
            }
        };
        
        // Check if we have a callback for this address
        match registry_guard.get(&hook_address) {
            Some(cb) => {
                // Clone the PyObject to avoid lifetime issues
                Python::with_gil(|py| cb.clone_ref(py))
            },
            None => {
                if let Some(logger) = get_logger() {
                    logger.log_error(&UniverseError::HookError(format!("No callback found for hook at 0x{:x}", hook_address)));
                }
                return -1;
            }
        }
    };

    // Execute the callback with Python GIL
    let (result, should_call_original) = Python::with_gil(|py| {
        // Create Python register object
        let py_registers = match register_state.to_python_object(py) {
            Ok(regs) => regs,
            Err(e) => {
                if let Some(logger) = get_logger() {
                    let error = UniverseError::PythonError(format!("Failed to create Python registers: {}", e));
                    logger.log_error_with_context(&error, "Hook callback execution");
                }
                return (-1, false);
            }
        };

        // Get the hook manager to create the original function object
        let hook_manager = match get_hook_manager() {
            Ok(manager) => manager,
            Err(e) => {
                if let Some(logger) = get_logger() {
                    logger.log_error_with_context(&e, "Failed to get hook manager in hook callback");
                }
                return (-1, false);
            }
        };

        // Lock the hook manager
        let hook_manager = match hook_manager.lock() {
            Ok(manager) => manager,
            Err(_) => {
                if let Some(logger) = get_logger() {
                    logger.log_error(&UniverseError::HookError("Failed to acquire hook manager lock".to_string()));
                }
                return (-1, false);
            }
        };

        // Get hook info
        let hook_info = match hook_manager.get_hook_info(hook_address) {
            Some(info) => info,
            None => {
                if let Some(logger) = get_logger() {
                    logger.log_error(&UniverseError::HookError(format!("No hook info found for address 0x{:x}", hook_address)));
                }
                return (-1, false);
            }
        };

        // Create original function object
        let original_function_obj = match &hook_info.hook_type {
            crate::hooks::HookType::Function { original_bytes, .. } => {
                let original = crate::hooks::PyOriginalFunction::new(
                    original_function,
                    original_bytes.clone(),
                );
                match Py::new(py, original) {
                    Ok(obj) => obj,
                    Err(e) => {
                        if let Some(logger) = get_logger() {
                            let error = UniverseError::PythonError(format!("Failed to create original function object: {}", e));
                            logger.log_error_with_context(&error, "Hook callback execution");
                        }
                        return (-1, false);
                    }
                }
            }
            _ => {
                if let Some(logger) = get_logger() {
                    logger.log_error(&UniverseError::HookError("Invalid hook type for function callback".to_string()));
                }
                return (-1, false);
            }
        };

        // Call the Python callback with (registers, original_function)
        let args = (
            py_registers.clone_ref(py),
            original_function_obj.clone_ref(py),
        );
        match callback.call1(py, args) {
            Ok(_) => {
                // Extract potentially modified register state
                let register_manager = crate::registers::RegisterManager::new();
                match register_manager.extract_register_state(&py_registers, py) {
                    Ok(modified_registers) => {
                        // Update the context with modified register values
                        register_state_to_context(&modified_registers, context);

                        // Check if the original function was called
                        let original_func = original_function_obj.borrow(py);
                        let should_call_original = original_func.was_called();

                        (0, should_call_original) // Success
                    }
                    Err(e) => {
                        if let Some(logger) = get_logger() {
                            let error = UniverseError::PythonError(format!("Failed to extract register state: {}", e));
                            logger.log_error_with_context(&error, "Hook callback execution");
                        }
                        (-1, false)
                    }
                }
            }
            Err(e) => {
                if let Some(logger) = get_logger() {
                    logger.log_error(&UniverseError::PythonError(format!("Hook callback failed: {}", e)));
                }
                (-1, false)
            }
        }
    });

    // If there was an error, return the error code
    if result != 0 {
        return result;
    }

    // Return a special code to indicate whether to call the original function
    if should_call_original {
        0 // Call original function
    } else {
        1 // Skip original function
    }
}

/// Execute a jmpback hook callback
///
/// This function is called from the assembly jmpback hook handler
#[no_mangle]
pub extern "C" fn execute_jmpback_callback(context: &mut WinContext, hook_address: usize) -> i32 {
    // Store the current hook address for potential error handling
    CURRENT_HOOK_ADDRESS.store(hook_address, Ordering::SeqCst);

    // Convert Windows CONTEXT to RegisterState
    let register_state = context_to_register_state(context);

    // Get the callback from the registry - we need to clone it to avoid lifetime issues
    let callback = {
        // Scope for the registry guard to ensure it's dropped after we're done with it
        let registry_guard = match JMPBACK_REGISTRY.read() {
            Ok(guard) => guard,
            Err(_) => {
                if let Some(logger) = get_logger() {
                    logger.log_error(&UniverseError::HookError("Failed to acquire jmpback registry read lock".to_string()));
                }
                return -1;
            }
        };
        
        // Check if we have a callback for this address
        match registry_guard.get(&hook_address) {
            Some(cb) => {
                // Clone the PyObject to avoid lifetime issues
                Python::with_gil(|py| cb.clone_ref(py))
            },
            None => {
                if let Some(logger) = get_logger() {
                    logger.log_error(&UniverseError::HookError(format!("No callback found for jmpback hook at 0x{:x}", hook_address)));
                }
                return -1;
            }
        }
    };

    // Execute the callback with Python GIL
    let result = Python::with_gil(|py| {
        // Create Python register object
        let py_registers = match register_state.to_python_object(py) {
            Ok(regs) => regs,
            Err(e) => {
                if let Some(logger) = get_logger() {
                    let error = UniverseError::PythonError(format!("Failed to create Python registers: {}", e));
                    logger.log_error_with_context(&error, "Jmpback hook callback execution");
                }
                return -1;
            }
        };

        // Call the Python callback with only (registers) parameter
        match callback.call1(py, (py_registers.clone_ref(py),)) {
            Ok(_) => {
                // Extract potentially modified register state
                let register_manager = crate::registers::RegisterManager::new();
                match register_manager.extract_register_state(&py_registers, py) {
                    Ok(modified_registers) => {
                        // Update the context with modified register values
                        register_state_to_context(&modified_registers, context);
                        0 // Success
                    }
                    Err(e) => {
                        if let Some(logger) = get_logger() {
                            let error = UniverseError::PythonError(format!("Failed to extract register state: {}", e));
                            logger.log_error_with_context(&error, "Jmpback hook callback execution");
                        }
                        -1
                    }
                }
            }
            Err(e) => {
                if let Some(logger) = get_logger() {
                    logger.log_error(&UniverseError::PythonError(format!("Jmpback hook callback failed: {}", e)));
                }
                -1
            }
        }
    });

    result
}

// Assembly hook handler that captures all CPU registers
// This is the entry point for function hooks
#[cfg(target_arch = "x86_64")]
global_asm!(
    r#"
.global hook_handler_asm
.global jmpback_handler_asm

# Function hook handler
hook_handler_asm:
    # Allocate stack space for WinContext structure
    sub rsp, 0x200
    
    # Save all general-purpose registers
    mov [rsp + 0x00], rax
    mov [rsp + 0x08], rbx
    mov [rsp + 0x10], rcx
    mov [rsp + 0x18], rdx
    mov [rsp + 0x20], rsi
    mov [rsp + 0x28], rdi
    mov [rsp + 0x30], rsp
    add qword ptr [rsp + 0x30], 0x200  # Adjust RSP to original value
    mov [rsp + 0x38], rbp
    mov [rsp + 0x40], r8
    mov [rsp + 0x48], r9
    mov [rsp + 0x50], r10
    mov [rsp + 0x58], r11
    mov [rsp + 0x60], r12
    mov [rsp + 0x68], r13
    mov [rsp + 0x70], r14
    mov [rsp + 0x78], r15
    
    # Save EFLAGS
    pushfq
    pop rax
    mov [rsp + 0x80], rax
    
    # Save XMM registers
    movdqu [rsp + 0x88], xmm0
    movdqu [rsp + 0x98], xmm1
    movdqu [rsp + 0xA8], xmm2
    movdqu [rsp + 0xB8], xmm3
    movdqu [rsp + 0xC8], xmm4
    movdqu [rsp + 0xD8], xmm5
    movdqu [rsp + 0xE8], xmm6
    movdqu [rsp + 0xF8], xmm7
    movdqu [rsp + 0x108], xmm8
    movdqu [rsp + 0x118], xmm9
    movdqu [rsp + 0x128], xmm10
    movdqu [rsp + 0x138], xmm11
    movdqu [rsp + 0x148], xmm12
    movdqu [rsp + 0x158], xmm13
    movdqu [rsp + 0x168], xmm14
    movdqu [rsp + 0x178], xmm15
    
    # Call the Rust hook callback handler
    # First parameter (RCX): pointer to context structure
    # Second parameter (RDX): hook address
    # Third parameter (R8): original function address
    mov rcx, rsp
    mov rdx, [rsp + 0x188]  # Hook address (passed by trampoline)
    mov r8, [rsp + 0x190]   # Original function address (passed by trampoline)
    
    # Align stack to 16 bytes (required by Windows x64 calling convention)
    sub rsp, 0x20
    call execute_hook_callback
    add rsp, 0x20
    
    # Check return value (RAX)
    test eax, eax
    js hook_error     # Jump if negative (error)
    cmp eax, 1
    je skip_original  # Jump if 1 (skip original function)
    
    # Restore XMM registers
    movdqu xmm0, [rsp + 0x88]
    movdqu xmm1, [rsp + 0x98]
    movdqu xmm2, [rsp + 0xA8]
    movdqu xmm3, [rsp + 0xB8]
    movdqu xmm4, [rsp + 0xC8]
    movdqu xmm5, [rsp + 0xD8]
    movdqu xmm6, [rsp + 0xE8]
    movdqu xmm7, [rsp + 0xF8]
    movdqu xmm8, [rsp + 0x108]
    movdqu xmm9, [rsp + 0x118]
    movdqu xmm10, [rsp + 0x128]
    movdqu xmm11, [rsp + 0x138]
    movdqu xmm12, [rsp + 0x148]
    movdqu xmm13, [rsp + 0x158]
    movdqu xmm14, [rsp + 0x168]
    movdqu xmm15, [rsp + 0x178]
    
    # Restore EFLAGS
    mov rax, [rsp + 0x80]
    push rax
    popfq
    
    # Restore general-purpose registers
    mov rax, [rsp + 0x00]
    mov rbx, [rsp + 0x08]
    mov rcx, [rsp + 0x10]
    mov rdx, [rsp + 0x18]
    mov rsi, [rsp + 0x20]
    mov rdi, [rsp + 0x28]
    # RSP is restored last
    mov rbp, [rsp + 0x38]
    mov r8, [rsp + 0x40]
    mov r9, [rsp + 0x48]
    mov r10, [rsp + 0x50]
    mov r11, [rsp + 0x58]
    mov r12, [rsp + 0x60]
    mov r13, [rsp + 0x68]
    mov r14, [rsp + 0x70]
    mov r15, [rsp + 0x78]
    
    # Get original function address (to call it)
    mov r10, [rsp + 0x190]  # Original function address (passed by trampoline)
    
    # Restore stack and jump to original function
    add rsp, 0x200
    jmp r10
    
skip_original:
    # Restore XMM registers
    movdqu xmm0, [rsp + 0x88]
    movdqu xmm1, [rsp + 0x98]
    movdqu xmm2, [rsp + 0xA8]
    movdqu xmm3, [rsp + 0xB8]
    movdqu xmm4, [rsp + 0xC8]
    movdqu xmm5, [rsp + 0xD8]
    movdqu xmm6, [rsp + 0xE8]
    movdqu xmm7, [rsp + 0xF8]
    movdqu xmm8, [rsp + 0x108]
    movdqu xmm9, [rsp + 0x118]
    movdqu xmm10, [rsp + 0x128]
    movdqu xmm11, [rsp + 0x138]
    movdqu xmm12, [rsp + 0x148]
    movdqu xmm13, [rsp + 0x158]
    movdqu xmm14, [rsp + 0x168]
    movdqu xmm15, [rsp + 0x178]
    
    # Restore EFLAGS
    mov rax, [rsp + 0x80]
    push rax
    popfq
    
    # Restore general-purpose registers
    mov rax, [rsp + 0x00]
    mov rbx, [rsp + 0x08]
    mov rcx, [rsp + 0x10]
    mov rdx, [rsp + 0x18]
    mov rsi, [rsp + 0x20]
    mov rdi, [rsp + 0x28]
    # RSP is restored last
    mov rbp, [rsp + 0x38]
    mov r8, [rsp + 0x40]
    mov r9, [rsp + 0x48]
    mov r10, [rsp + 0x50]
    mov r11, [rsp + 0x58]
    mov r12, [rsp + 0x60]
    mov r13, [rsp + 0x68]
    mov r14, [rsp + 0x70]
    mov r15, [rsp + 0x78]
    
    # Get return address (to skip the original function)
    mov r10, [rsp + 0x198]  # Return address (passed by trampoline)
    
    # Restore stack and jump to return address (skipping original function)
    add rsp, 0x200
    jmp r10

hook_error:
    # Handle error case - log the error through the logger
    # The error has already been logged in the Rust function
    
    # Restore XMM registers
    movdqu xmm0, [rsp + 0x88]
    movdqu xmm1, [rsp + 0x98]
    movdqu xmm2, [rsp + 0xA8]
    movdqu xmm3, [rsp + 0xB8]
    movdqu xmm4, [rsp + 0xC8]
    movdqu xmm5, [rsp + 0xD8]
    movdqu xmm6, [rsp + 0xE8]
    movdqu xmm7, [rsp + 0xF8]
    movdqu xmm8, [rsp + 0x108]
    movdqu xmm9, [rsp + 0x118]
    movdqu xmm10, [rsp + 0x128]
    movdqu xmm11, [rsp + 0x138]
    movdqu xmm12, [rsp + 0x148]
    movdqu xmm13, [rsp + 0x158]
    movdqu xmm14, [rsp + 0x168]
    movdqu xmm15, [rsp + 0x178]
    
    # Restore EFLAGS
    mov rax, [rsp + 0x80]
    push rax
    popfq
    
    # Restore general-purpose registers
    mov rax, [rsp + 0x00]
    mov rbx, [rsp + 0x08]
    mov rcx, [rsp + 0x10]
    mov rdx, [rsp + 0x18]
    mov rsi, [rsp + 0x20]
    mov rdi, [rsp + 0x28]
    # RSP is restored last
    mov rbp, [rsp + 0x38]
    mov r8, [rsp + 0x40]
    mov r9, [rsp + 0x48]
    mov r10, [rsp + 0x50]
    mov r11, [rsp + 0x58]
    mov r12, [rsp + 0x60]
    mov r13, [rsp + 0x68]
    mov r14, [rsp + 0x70]
    mov r15, [rsp + 0x78]
    
    # Get return address and original function address
    mov r10, [rsp + 0x198]  # Return address (passed by trampoline)
    
    # Restore stack and jump to original function
    add rsp, 0x200
    jmp r10

# JmpBack hook handler
jmpback_handler_asm:
    # Allocate stack space for WinContext structure
    sub rsp, 0x200
    
    # Save all general-purpose registers
    mov [rsp + 0x00], rax
    mov [rsp + 0x08], rbx
    mov [rsp + 0x10], rcx
    mov [rsp + 0x18], rdx
    mov [rsp + 0x20], rsi
    mov [rsp + 0x28], rdi
    mov [rsp + 0x30], rsp
    add qword ptr [rsp + 0x30], 0x200  # Adjust RSP to original value
    mov [rsp + 0x38], rbp
    mov [rsp + 0x40], r8
    mov [rsp + 0x48], r9
    mov [rsp + 0x50], r10
    mov [rsp + 0x58], r11
    mov [rsp + 0x60], r12
    mov [rsp + 0x68], r13
    mov [rsp + 0x70], r14
    mov [rsp + 0x78], r15
    
    # Save EFLAGS
    pushfq
    pop rax
    mov [rsp + 0x80], rax
    
    # Save XMM registers
    movdqu [rsp + 0x88], xmm0
    movdqu [rsp + 0x98], xmm1
    movdqu [rsp + 0xA8], xmm2
    movdqu [rsp + 0xB8], xmm3
    movdqu [rsp + 0xC8], xmm4
    movdqu [rsp + 0xD8], xmm5
    movdqu [rsp + 0xE8], xmm6
    movdqu [rsp + 0xF8], xmm7
    movdqu [rsp + 0x108], xmm8
    movdqu [rsp + 0x118], xmm9
    movdqu [rsp + 0x128], xmm10
    movdqu [rsp + 0x138], xmm11
    movdqu [rsp + 0x148], xmm12
    movdqu [rsp + 0x158], xmm13
    movdqu [rsp + 0x168], xmm14
    movdqu [rsp + 0x178], xmm15
    
    # Call the Rust jmpback hook callback handler
    # First parameter (RCX): pointer to context structure
    # Second parameter (RDX): hook address
    mov rcx, rsp
    mov rdx, [rsp + 0x188]  # Hook address (passed by trampoline)
    
    # Align stack to 16 bytes (required by Windows x64 calling convention)
    sub rsp, 0x20
    call execute_jmpback_callback
    add rsp, 0x20
    
    # Check return value (RAX)
    test eax, eax
    jnz jmpback_error
    
    # Restore XMM registers
    movdqu xmm0, [rsp + 0x88]
    movdqu xmm1, [rsp + 0x98]
    movdqu xmm2, [rsp + 0xA8]
    movdqu xmm3, [rsp + 0xB8]
    movdqu xmm4, [rsp + 0xC8]
    movdqu xmm5, [rsp + 0xD8]
    movdqu xmm6, [rsp + 0xE8]
    movdqu xmm7, [rsp + 0xF8]
    movdqu xmm8, [rsp + 0x108]
    movdqu xmm9, [rsp + 0x118]
    movdqu xmm10, [rsp + 0x128]
    movdqu xmm11, [rsp + 0x138]
    movdqu xmm12, [rsp + 0x148]
    movdqu xmm13, [rsp + 0x158]
    movdqu xmm14, [rsp + 0x168]
    movdqu xmm15, [rsp + 0x178]
    
    # Restore EFLAGS
    mov rax, [rsp + 0x80]
    push rax
    popfq
    
    # Restore general-purpose registers
    mov rax, [rsp + 0x00]
    mov rbx, [rsp + 0x08]
    mov rcx, [rsp + 0x10]
    mov rdx, [rsp + 0x18]
    mov rsi, [rsp + 0x20]
    mov rdi, [rsp + 0x28]
    # RSP is restored last
    mov rbp, [rsp + 0x38]
    mov r8, [rsp + 0x40]
    mov r9, [rsp + 0x48]
    mov r10, [rsp + 0x50]
    mov r11, [rsp + 0x58]
    mov r12, [rsp + 0x60]
    mov r13, [rsp + 0x68]
    mov r14, [rsp + 0x70]
    mov r15, [rsp + 0x78]
    
    # Get trampoline address (contains original bytes + jmp back)
    mov r10, [rsp + 0x190]  # Trampoline address (passed by hook)
    
    # Restore stack and jump to trampoline
    add rsp, 0x200
    jmp r10

jmpback_error:
    # Handle error case - log the error through the logger
    # The error has already been logged in the Rust function
    
    # Restore XMM registers
    movdqu xmm0, [rsp + 0x88]
    movdqu xmm1, [rsp + 0x98]
    movdqu xmm2, [rsp + 0xA8]
    movdqu xmm3, [rsp + 0xB8]
    movdqu xmm4, [rsp + 0xC8]
    movdqu xmm5, [rsp + 0xD8]
    movdqu xmm6, [rsp + 0xE8]
    movdqu xmm7, [rsp + 0xF8]
    movdqu xmm8, [rsp + 0x108]
    movdqu xmm9, [rsp + 0x118]
    movdqu xmm10, [rsp + 0x128]
    movdqu xmm11, [rsp + 0x138]
    movdqu xmm12, [rsp + 0x148]
    movdqu xmm13, [rsp + 0x158]
    movdqu xmm14, [rsp + 0x168]
    movdqu xmm15, [rsp + 0x178]
    
    # Restore EFLAGS
    mov rax, [rsp + 0x80]
    push rax
    popfq
    
    # Restore general-purpose registers
    mov rax, [rsp + 0x00]
    mov rbx, [rsp + 0x08]
    mov rcx, [rsp + 0x10]
    mov rdx, [rsp + 0x18]
    mov rsi, [rsp + 0x20]
    mov rdi, [rsp + 0x28]
    # RSP is restored last
    mov rbp, [rsp + 0x38]
    mov r8, [rsp + 0x40]
    mov r9, [rsp + 0x48]
    mov r10, [rsp + 0x50]
    mov r11, [rsp + 0x58]
    mov r12, [rsp + 0x60]
    mov r13, [rsp + 0x68]
    mov r14, [rsp + 0x70]
    mov r15, [rsp + 0x78]
    
    # Get trampoline address (contains original bytes + jmp back)
    mov r10, [rsp + 0x190]  # Trampoline address (passed by hook)
    
    # Restore stack and jump to trampoline
    add rsp, 0x200
    jmp r10
"#
);

// Export assembly functions for use in Rust
extern "C" {
    pub fn hook_handler_asm();
    pub fn jmpback_handler_asm();
}

// Get the address of the hook handler assembly function
pub fn get_hook_handler_address() -> usize {
    hook_handler_asm as usize
}

// Get the address of the jmpback hook handler assembly function
pub fn get_jmpback_handler_address() -> usize {
    jmpback_handler_asm as usize
}
