use std::collections::HashMap;

use crate::core::UniverseError;
use crate::hook_handlers::{
    clear_all_hook_callbacks, clear_all_jmpback_callbacks, execute_hook_callback,
    execute_jmpback_callback, register_hook_callback, register_jmpback_callback,
    remove_hook_callback, remove_jmpback_callback,
};

use ilhook::x64::HookPoint;
use ilhook::x64::{CallbackOption, HookFlags, HookType, Hooker, Registers};
use pyo3::{PyObject, Python};

/// Hook management system for function interception
pub struct HookManager {
    active_hooks: HashMap<usize, (HookPoint, usize)>, // address -> (hook, py_callback ptr)
}

impl HookManager {
    /// Create a new hook manager instance
    pub fn new() -> Result<Self, UniverseError> {
        Ok(HookManager {
            active_hooks: HashMap::new(),
        })
    }

    /// Install a function hook at the specified address with Python callback
    pub fn install_function_hook(
        &mut self,
        address: usize,
        callback: PyObject,
    ) -> Result<(), UniverseError> {
        // Check if hook already exists at this address
        if self.active_hooks.contains_key(&address) {
            return Err(UniverseError::HookError(format!(
                "Hook already exists at address 0x{:x}",
                address
            )));
        }

        // Store the Python callback in the global registry
        let callback_ref = Python::with_gil(|py| Box::leak(Box::new(callback.clone_ref(py))));
        match register_hook_callback(callback_ref) {
            Ok(_) => (),
            Err(e) => {
                unsafe {
                    let boxed_callback = Box::from_raw(callback_ref);
                    Python::with_gil(|py| boxed_callback.drop_ref(py));
                }
                return Err(e);
            }
        }

        // Install the hook using ilhook
        let hook = unsafe {
            Hooker::new(
                address,
                HookType::Retn(execute_hook_callback_c),
                CallbackOption::None,
                callback_ref as *mut _ as usize,
                HookFlags::empty(),
            )
            .hook()
            .map_err(|e| UniverseError::HookError(format!("Failed to install ilhook: {:?}", e)))?
        };

        self.active_hooks
            .insert(address, (hook, callback_ref as *mut _ as usize));

        Ok(())
    }

    /// Install a jmpback hook at the specified address with Python callback
    pub fn install_jmpback_hook(
        &mut self,
        address: usize,
        callback: PyObject,
    ) -> Result<(), UniverseError> {
        // Check if hook already exists at this address
        if self.active_hooks.contains_key(&address) {
            return Err(UniverseError::HookError(format!(
                "Hook already exists at address 0x{:x}",
                address
            )));
        }

        // Store the Python callback in the global registry
        let callback_ref = Python::with_gil(|py| Box::leak(Box::new(callback.clone_ref(py))));
        match register_jmpback_callback(callback_ref) {
            Ok(_) => (),
            Err(e) => {
                unsafe {
                    let boxed_callback = Box::from_raw(callback_ref);
                    Python::with_gil(|py| boxed_callback.drop_ref(py));
                }
                return Err(e);
            }
        }

        // Install the hook using ilhook
        let hook = unsafe {
            Hooker::new(
                address,
                HookType::JmpBack(execute_jmpback_callback_c),
                CallbackOption::None,
                callback_ref as *mut _ as usize,
                HookFlags::empty(),
            )
            .hook()
            .map_err(|e| UniverseError::HookError(format!("Failed to install ilhook: {:?}", e)))?
        };

        self.active_hooks
            .insert(address, (hook, callback_ref as *mut _ as usize));

        Ok(())
    }

    /// Remove a hook at the specified address
    pub fn remove_hook(&mut self, address: usize) -> Result<(), UniverseError> {
        if let Some((hook, py_callback_ptr)) = self.active_hooks.remove(&address) {
            unsafe {
                hook.unhook()
                    .map_err(|e| UniverseError::HookError(format!("Failed to unhook: {:?}", e)))?;
            }

            // Remove from both registries, as we don't know the type here
            remove_hook_callback(py_callback_ptr)?;
            remove_jmpback_callback(py_callback_ptr)?;
            Ok(())
        } else {
            Err(UniverseError::HookError(format!(
                "No hook found at address 0x{:x}",
                address
            )))
        }
    }

    /// Remove all active hooks
    pub fn remove_all_hooks(&mut self) -> Result<(), UniverseError> {
        let addresses: Vec<usize> = self.active_hooks.keys().cloned().collect();

        for address in addresses {
            if let Err(_e) = self.remove_hook(address) {
                // TODO: Log error if needed, but continue to try and remove other hooks
            }
        }

        // Ensure the map is cleared even if some removals failed
        self.active_hooks.clear();

        // Ensure all callback registries are cleared if any of the py_callback_ptrs failed to remove
        clear_all_hook_callbacks()?;
        clear_all_jmpback_callbacks()?;

        Ok(())
    }

    /// Get information about an active hook (simplified for ilhook)
    pub fn get_hook_info(&self, address: usize) -> Option<usize> {
        self.active_hooks.get(&address).map(|_| address) // Just return the address if hook exists
    }

    /// Cleanup hook manager resources
    pub fn cleanup(&mut self) -> Result<(), UniverseError> {
        self.remove_all_hooks()?;
        Ok(())
    }
}

// C-compatible callback for ilhook function hooks
// This function will be called by ilhook when a JmpToRet hook is triggered.
// It then calls the Python callback.
unsafe extern "win64" fn execute_hook_callback_c(
    regs: *mut Registers,
    original_function_ptr: usize,
    user_data: usize, // This will be the py_callback_ptr
) -> usize {
    execute_hook_callback(regs, original_function_ptr, user_data)
}

// C-compatible callback for ilhook jmpback hooks
// This function will be called by ilhook when a JmpBack hook is triggered.
// It then calls the Python callback.
unsafe extern "win64" fn execute_jmpback_callback_c(
    regs: *mut Registers,
    user_data: usize, // This will be the py_callback_ptr
) {
    execute_jmpback_callback(regs, user_data)
}
