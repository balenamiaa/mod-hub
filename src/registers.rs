use pyo3::prelude::*;
use std::arch::asm;

// Define our own CONTEXT structure for x64 since windows-sys CONTEXT is architecture-specific
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WinContext {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rsp: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub eflags: u32,
    // Simplified XMM representation
    pub xmm0: u128,
    pub xmm1: u128,
    pub xmm2: u128,
    pub xmm3: u128,
    pub xmm4: u128,
    pub xmm5: u128,
    pub xmm6: u128,
    pub xmm7: u128,
    pub xmm8: u128,
    pub xmm9: u128,
    pub xmm10: u128,
    pub xmm11: u128,
    pub xmm12: u128,
    pub xmm13: u128,
    pub xmm14: u128,
    pub xmm15: u128,
}

/// Register state structure containing all x64 CPU registers
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RegisterState {
    // General-purpose registers
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rsp: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    
    // Floating-point registers (XMM0-XMM15)
    pub xmm: [u128; 16],
    
    // Flags register
    pub rflags: u64,
}

impl RegisterState {
    /// Create a new empty register state
    pub fn new() -> Self {
        RegisterState {
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rsp: 0, rbp: 0,
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
            xmm: [0; 16],
            rflags: 0,
        }
    }

    /// Capture current register state (unsafe - requires inline assembly)
    /// Note: This is a simplified version for demonstration
    /// In a real hook scenario, registers would be captured by the hook trampoline
    pub unsafe fn capture() -> Self {
        let mut state = RegisterState::new();
        
        // Capture general-purpose registers using inline assembly
        // Note: This is simplified - in practice, this would be done by hook trampolines
        asm!(
            "mov {}, rax",
            "mov {}, rbx", 
            "mov {}, rcx",
            "mov {}, rdx",
            "mov {}, rsi",
            "mov {}, rdi",
            "mov {}, rsp",
            "mov {}, rbp",
            "mov {}, r8",
            "mov {}, r9",
            "mov {}, r10",
            "mov {}, r11",
            "mov {}, r12",
            "mov {}, r13",
            "mov {}, r14",
            "mov {}, r15",
            out(reg) state.rax,
            out(reg) state.rbx,
            out(reg) state.rcx,
            out(reg) state.rdx,
            out(reg) state.rsi,
            out(reg) state.rdi,
            out(reg) state.rsp,
            out(reg) state.rbp,
            out(reg) state.r8,
            out(reg) state.r9,
            out(reg) state.r10,
            out(reg) state.r11,
            out(reg) state.r12,
            out(reg) state.r13,
            out(reg) state.r14,
            out(reg) state.r15,
        );

        // Capture flags register
        asm!("pushfq; pop {}", out(reg) state.rflags);

        state
    }

    /// Restore register state (unsafe - requires inline assembly)
    /// Note: This would typically be done by hook trampolines when returning to original code
    pub unsafe fn restore(&self) {
        // Restore general-purpose registers
        asm!(
            "mov rax, {}",
            "mov rbx, {}",
            "mov rcx, {}",
            "mov rdx, {}",
            "mov rsi, {}",
            "mov rdi, {}",
            "mov r8, {}",
            "mov r9, {}",
            "mov r10, {}",
            "mov r11, {}",
            "mov r12, {}",
            "mov r13, {}",
            "mov r14, {}",
            "mov r15, {}",
            in(reg) self.rax,
            in(reg) self.rbx,
            in(reg) self.rcx,
            in(reg) self.rdx,
            in(reg) self.rsi,
            in(reg) self.rdi,
            in(reg) self.r8,
            in(reg) self.r9,
            in(reg) self.r10,
            in(reg) self.r11,
            in(reg) self.r12,
            in(reg) self.r13,
            in(reg) self.r14,
            in(reg) self.r15,
        );

        // Note: RSP and RBP are not restored here as they would break the stack
        // In practice, these would be handled carefully by the hook system

        // Restore flags register
        asm!("push {}; popfq", in(reg) self.rflags);
    }

    /// Convert register state to Python object
    pub fn to_python_object(&self, py: Python) -> PyResult<PyObject> {
        let registers = PyRegisterAccess::new(*self);
        Ok(Py::new(py, registers)?.into())
    }

    /// Create register state from Windows CONTEXT structure
    pub fn from_win_context(context: &WinContext) -> Self {
        RegisterState {
            rax: context.rax,
            rbx: context.rbx,
            rcx: context.rcx,
            rdx: context.rdx,
            rsi: context.rsi,
            rdi: context.rdi,
            rsp: context.rsp,
            rbp: context.rbp,
            r8: context.r8,
            r9: context.r9,
            r10: context.r10,
            r11: context.r11,
            r12: context.r12,
            r13: context.r13,
            r14: context.r14,
            r15: context.r15,
            xmm: [
                context.xmm0, context.xmm1, context.xmm2, context.xmm3,
                context.xmm4, context.xmm5, context.xmm6, context.xmm7,
                context.xmm8, context.xmm9, context.xmm10, context.xmm11,
                context.xmm12, context.xmm13, context.xmm14, context.xmm15,
            ],
            rflags: context.eflags as u64,
        }
    }
}

/// Python wrapper for register access with read/write capabilities
#[pyclass(name = "Registers")]
pub struct PyRegisterAccess {
    state: RegisterState,
    modified: bool,
}

impl PyRegisterAccess {
    pub fn new(state: RegisterState) -> Self {
        PyRegisterAccess {
            state,
            modified: false,
        }
    }

    pub fn get_state(&self) -> RegisterState {
        self.state
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }
}

#[pymethods]
impl PyRegisterAccess {
    // General-purpose register getters
    #[getter]
    fn rax(&self) -> u64 { self.state.rax }
    
    #[getter]
    fn rbx(&self) -> u64 { self.state.rbx }
    
    #[getter]
    fn rcx(&self) -> u64 { self.state.rcx }
    
    #[getter]
    fn rdx(&self) -> u64 { self.state.rdx }
    
    #[getter]
    fn rsi(&self) -> u64 { self.state.rsi }
    
    #[getter]
    fn rdi(&self) -> u64 { self.state.rdi }
    
    #[getter]
    fn rsp(&self) -> u64 { self.state.rsp }
    
    #[getter]
    fn rbp(&self) -> u64 { self.state.rbp }
    
    #[getter]
    fn r8(&self) -> u64 { self.state.r8 }
    
    #[getter]
    fn r9(&self) -> u64 { self.state.r9 }
    
    #[getter]
    fn r10(&self) -> u64 { self.state.r10 }
    
    #[getter]
    fn r11(&self) -> u64 { self.state.r11 }
    
    #[getter]
    fn r12(&self) -> u64 { self.state.r12 }
    
    #[getter]
    fn r13(&self) -> u64 { self.state.r13 }
    
    #[getter]
    fn r14(&self) -> u64 { self.state.r14 }
    
    #[getter]
    fn r15(&self) -> u64 { self.state.r15 }
    
    #[getter]
    fn rflags(&self) -> u64 { self.state.rflags }

    // General-purpose register setters
    #[setter]
    fn set_rax(&mut self, value: u64) {
        self.state.rax = value;
        self.modified = true;
    }
    
    #[setter]
    fn set_rbx(&mut self, value: u64) {
        self.state.rbx = value;
        self.modified = true;
    }
    
    #[setter]
    fn set_rcx(&mut self, value: u64) {
        self.state.rcx = value;
        self.modified = true;
    }
    
    #[setter]
    fn set_rdx(&mut self, value: u64) {
        self.state.rdx = value;
        self.modified = true;
    }
    
    #[setter]
    fn set_rsi(&mut self, value: u64) {
        self.state.rsi = value;
        self.modified = true;
    }
    
    #[setter]
    fn set_rdi(&mut self, value: u64) {
        self.state.rdi = value;
        self.modified = true;
    }
    
    #[setter]
    fn set_rsp(&mut self, value: u64) {
        self.state.rsp = value;
        self.modified = true;
    }
    
    #[setter]
    fn set_rbp(&mut self, value: u64) {
        self.state.rbp = value;
        self.modified = true;
    }
    
    #[setter]
    fn set_r8(&mut self, value: u64) {
        self.state.r8 = value;
        self.modified = true;
    }
    
    #[setter]
    fn set_r9(&mut self, value: u64) {
        self.state.r9 = value;
        self.modified = true;
    }
    
    #[setter]
    fn set_r10(&mut self, value: u64) {
        self.state.r10 = value;
        self.modified = true;
    }
    
    #[setter]
    fn set_r11(&mut self, value: u64) {
        self.state.r11 = value;
        self.modified = true;
    }
    
    #[setter]
    fn set_r12(&mut self, value: u64) {
        self.state.r12 = value;
        self.modified = true;
    }
    
    #[setter]
    fn set_r13(&mut self, value: u64) {
        self.state.r13 = value;
        self.modified = true;
    }
    
    #[setter]
    fn set_r14(&mut self, value: u64) {
        self.state.r14 = value;
        self.modified = true;
    }
    
    #[setter]
    fn set_r15(&mut self, value: u64) {
        self.state.r15 = value;
        self.modified = true;
    }
    
    #[setter]
    fn set_rflags(&mut self, value: u64) {
        self.state.rflags = value;
        self.modified = true;
    }

    // XMM register access methods
    fn get_xmm(&self, index: usize) -> PyResult<u128> {
        if index >= 16 {
            return Err(pyo3::exceptions::PyIndexError::new_err("XMM register index must be 0-15"));
        }
        Ok(self.state.xmm[index])
    }
    
    fn set_xmm(&mut self, index: usize, value: u128) -> PyResult<()> {
        if index >= 16 {
            return Err(pyo3::exceptions::PyIndexError::new_err("XMM register index must be 0-15"));
        }
        self.state.xmm[index] = value;
        self.modified = true;
        Ok(())
    }

    // Convenience methods for XMM registers as bytes
    fn get_xmm_bytes(&self, index: usize) -> PyResult<Vec<u8>> {
        if index >= 16 {
            return Err(pyo3::exceptions::PyIndexError::new_err("XMM register index must be 0-15"));
        }
        Ok(self.state.xmm[index].to_le_bytes().to_vec())
    }
    
    fn set_xmm_bytes(&mut self, index: usize, bytes: Vec<u8>) -> PyResult<()> {
        if index >= 16 {
            return Err(pyo3::exceptions::PyIndexError::new_err("XMM register index must be 0-15"));
        }
        if bytes.len() != 16 {
            return Err(pyo3::exceptions::PyValueError::new_err("XMM register requires exactly 16 bytes"));
        }
        
        let mut array = [0u8; 16];
        array.copy_from_slice(&bytes);
        self.state.xmm[index] = u128::from_le_bytes(array);
        self.modified = true;
        Ok(())
    }

    // String representation for debugging
    fn __repr__(&self) -> String {
        format!(
            "Registers(rax=0x{:016x}, rbx=0x{:016x}, rcx=0x{:016x}, rdx=0x{:016x}, rsi=0x{:016x}, rdi=0x{:016x}, rsp=0x{:016x}, rbp=0x{:016x}, r8=0x{:016x}, r9=0x{:016x}, r10=0x{:016x}, r11=0x{:016x}, r12=0x{:016x}, r13=0x{:016x}, r14=0x{:016x}, r15=0x{:016x}, rflags=0x{:016x})",
            self.state.rax, self.state.rbx, self.state.rcx, self.state.rdx,
            self.state.rsi, self.state.rdi, self.state.rsp, self.state.rbp,
            self.state.r8, self.state.r9, self.state.r10, self.state.r11,
            self.state.r12, self.state.r13, self.state.r14, self.state.r15,
            self.state.rflags
        )
    }
}

/// Register manager for handling register state operations
pub struct RegisterManager;

impl RegisterManager {
    pub fn new() -> Self {
        RegisterManager
    }

    /// Create a Python register object from register state
    pub fn create_python_registers(&self, state: RegisterState, py: Python) -> PyResult<PyObject> {
        state.to_python_object(py)
    }

    /// Extract register state from Python register object
    pub fn extract_register_state(&self, py_registers: &PyObject, py: Python) -> PyResult<RegisterState> {
        let registers: PyRef<PyRegisterAccess> = py_registers.extract(py)?;
        Ok(registers.get_state())
    }

    /// Check if register state has been modified
    pub fn is_modified(&self, py_registers: &PyObject, py: Python) -> PyResult<bool> {
        let registers: PyRef<PyRegisterAccess> = py_registers.extract(py)?;
        Ok(registers.is_modified())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_state_creation() {
        let state = RegisterState::new();
        assert_eq!(state.rax, 0);
        assert_eq!(state.rbx, 0);
        assert_eq!(state.xmm[0], 0);
        assert_eq!(state.xmm[15], 0);
    }

    #[test]
    fn test_register_state_modification() {
        let mut state = RegisterState::new();
        state.rax = 0x1234567890abcdef;
        state.xmm[0] = 0x123456789abcdef0fedcba0987654321;
        
        assert_eq!(state.rax, 0x1234567890abcdef);
        assert_eq!(state.xmm[0], 0x123456789abcdef0fedcba0987654321);
    }
}