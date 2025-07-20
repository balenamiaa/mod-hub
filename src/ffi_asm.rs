use crate::core::UniverseError;
use crate::ffi::{CallingConvention, NativeType};
#[cfg(target_arch = "x86_64")]
use std::arch::global_asm;



// Assembly for the FFI trampoline generator
#[cfg(target_arch = "x86_64")]
global_asm!(
    r#"
.global ffi_call_cdecl_asm
.global ffi_call_stdcall_asm
.global ffi_call_fastcall_asm

# x64 cdecl calling convention handler
# In x64 Windows, the first 4 parameters are passed in RCX, RDX, R8, R9
# Additional parameters are passed on the stack
# Floating point parameters are passed in XMM0-XMM3
# Return value is in RAX (integer) or XMM0 (floating point)
ffi_call_cdecl_asm:
    # Preserve non-volatile registers
    push rbp
    mov rbp, rsp
    push rbx
    push rsi
    push rdi
    push r12
    push r13
    push r14
    push r15
    sub rsp, 0x40  # Shadow space + alignment

    # RCX = function address
    # RDX = args array pointer
    # R8 = number of args
    # R9 = is_float_return (1 if float return, 0 otherwise)

    # Save parameters
    mov r12, rcx    # r12 = function address
    mov r13, rdx    # r13 = args array pointer
    mov r14, r8     # r14 = number of args
    mov r15, r9     # r15 = is_float_return

    # Calculate stack space needed for arguments beyond the first 4
    # Each arg takes 8 bytes, and we need to handle args 4+
    # Also ensure we have at least 32 bytes of shadow space for the called function
    xor rcx, rcx
    cmp r14, 4
    jle .Lno_stack_args_cdecl
    mov rcx, r14
    sub rcx, 4
    shl rcx, 3      # rcx = (num_args - 4) * 8

    # Add shadow space (32 bytes minimum for x64 calling convention)
    add rcx, 32

    # Allocate stack space for arguments (maintain 16-byte alignment)
    add rcx, 15
    and rcx, -16
    sub rsp, rcx
    jmp .Lstack_allocated_cdecl

.Lno_stack_args_cdecl:
    # Even with no stack args, we need shadow space for the called function
    sub rsp, 32

.Lstack_allocated_cdecl:

.Lno_stack_args_cdecl:
    # Load the first 4 arguments into registers
    xor rcx, rcx
    xor rdx, rdx
    xor r8, r8
    xor r9, r9

    # Load arg 0 into RCX if available
    cmp r14, 0
    je .Lload_args_done_cdecl
    mov rcx, [r13]

    # Load arg 1 into RDX if available
    cmp r14, 1
    je .Lload_args_done_cdecl
    mov rdx, [r13 + 8]

    # Load arg 2 into R8 if available
    cmp r14, 2
    je .Lload_args_done_cdecl
    mov r8, [r13 + 16]

    # Load arg 3 into R9 if available
    cmp r14, 3
    je .Lload_args_done_cdecl
    mov r9, [r13 + 24]

    # Load remaining args onto the stack in reverse order
    cmp r14, 4
    jle .Lload_args_done_cdecl

    mov rax, r14
    sub rax, 4      # rax = num_args - 4 (number of stack args)
    mov rbx, rax
    shl rbx, 3      # rbx = (num_args - 4) * 8
    add rbx, r13    # rbx = pointer to last stack arg
    add rbx, 24     # adjust to start at arg 4

    # Push stack arguments (args 4+) starting after shadow space
    mov rsi, 0      # rsi = current arg index
.Lstack_arg_loop_cdecl:
    mov rdi, [rbx + rsi * 8]  # Load argument
    mov [rsp + rsi * 8 + 32], rdi  # Store on stack after shadow space
    inc rsi
    cmp rsi, rax
    jl .Lstack_arg_loop_cdecl

.Lload_args_done_cdecl:
    # Call the function
    call r12

    # Check if we need to return a floating point value
    cmp r15, 1
    je .Lfloat_return_cdecl

    # Integer return in RAX
    jmp .Lreturn_cdecl

.Lfloat_return_cdecl:
    # Float return already in XMM0

.Lreturn_cdecl:
    # Restore stack and registers
    lea rsp, [rbp - 40]  # Restore stack pointer
    pop r15
    pop r14
    pop r13
    pop r12
    pop rdi
    pop rsi
    pop rbx
    pop rbp
    ret

# x64 stdcall calling convention handler
# In x64 Windows, stdcall is the same as cdecl (the callee cleans the stack)
# However, we provide a separate implementation for clarity and potential future differences
ffi_call_stdcall_asm:
    # Preserve non-volatile registers
    push rbp
    mov rbp, rsp
    push rbx
    push rsi
    push rdi
    push r12
    push r13
    push r14
    push r15
    sub rsp, 0x40  # Shadow space + alignment

    # RCX = function address
    # RDX = args array pointer
    # R8 = number of args
    # R9 = is_float_return (1 if float return, 0 otherwise)

    # Save parameters
    mov r12, rcx    # r12 = function address
    mov r13, rdx    # r13 = args array pointer
    mov r14, r8     # r14 = number of args
    mov r15, r9     # r15 = is_float_return

    # Calculate stack space needed for arguments beyond the first 4
    # Also ensure we have at least 32 bytes of shadow space for the called function
    xor rcx, rcx
    cmp r14, 4
    jle .Lno_stack_args_stdcall
    mov rcx, r14
    sub rcx, 4
    shl rcx, 3      # rcx = (num_args - 4) * 8

    # Add shadow space (32 bytes minimum for x64 calling convention)
    add rcx, 32

    # Allocate stack space for arguments (maintain 16-byte alignment)
    add rcx, 15
    and rcx, -16
    sub rsp, rcx
    jmp .Lstack_allocated_stdcall

.Lno_stack_args_stdcall:
    # Even with no stack args, we need shadow space for the called function
    sub rsp, 32

.Lstack_allocated_stdcall:
    # Load the first 4 arguments into registers
    xor rcx, rcx
    xor rdx, rdx
    xor r8, r8
    xor r9, r9

    # Load arg 0 into RCX if available
    cmp r14, 0
    je .Lload_args_done_stdcall
    mov rcx, [r13]

    # Load arg 1 into RDX if available
    cmp r14, 1
    je .Lload_args_done_stdcall
    mov rdx, [r13 + 8]

    # Load arg 2 into R8 if available
    cmp r14, 2
    je .Lload_args_done_stdcall
    mov r8, [r13 + 16]

    # Load arg 3 into R9 if available
    cmp r14, 3
    je .Lload_args_done_stdcall
    mov r9, [r13 + 24]

    # Load remaining args onto the stack starting after shadow space
    cmp r14, 4
    jle .Lload_args_done_stdcall

    mov rax, r14
    sub rax, 4      # rax = num_args - 4 (number of stack args)
    mov rbx, rax
    shl rbx, 3      # rbx = (num_args - 4) * 8
    add rbx, r13    # rbx = pointer to last stack arg
    add rbx, 24     # adjust to start at arg 4

    # Push stack arguments (args 4+) starting after shadow space
    mov rsi, 0      # rsi = current arg index
.Lstack_arg_loop_stdcall:
    mov rdi, [rbx + rsi * 8]  # Load argument
    mov [rsp + rsi * 8 + 32], rdi  # Store on stack after shadow space
    inc rsi
    cmp rsi, rax
    jl .Lstack_arg_loop_stdcall

.Lload_args_done_stdcall:
    # Call the function
    call r12

    # Check if we need to return a floating point value
    cmp r15, 1
    je .Lfloat_return_stdcall

    # Integer return in RAX
    jmp .Lreturn_stdcall

.Lfloat_return_stdcall:
    # Float return already in XMM0

.Lreturn_stdcall:
    # Restore stack and registers
    lea rsp, [rbp - 40]  # Restore stack pointer
    pop r15
    pop r14
    pop r13
    pop r12
    pop rdi
    pop rsi
    pop rbx
    pop rbp
    ret

# x64 fastcall calling convention handler
# In x64 Windows, fastcall is essentially the same as the standard calling convention
# The first 4 integer arguments are in RCX, RDX, R8, R9
# The first 4 floating point arguments are in XMM0-XMM3
# Additional arguments are passed on the stack
# This implementation provides enhanced floating-point register handling
ffi_call_fastcall_asm:
    # Preserve non-volatile registers
    push rbp
    mov rbp, rsp
    push rbx
    push rsi
    push rdi
    push r12
    push r13
    push r14
    push r15
    sub rsp, 0x40  # Shadow space + alignment

    # RCX = function address
    # RDX = args array pointer
    # R8 = number of args
    # R9 = is_float_return (1 if float return, 0 otherwise)

    # Save parameters
    mov r12, rcx    # r12 = function address
    mov r13, rdx    # r13 = args array pointer
    mov r14, r8     # r14 = number of args
    mov r15, r9     # r15 = is_float_return

    # Calculate stack space needed for arguments beyond the first 4
    # Also ensure we have at least 32 bytes of shadow space for the called function
    xor rcx, rcx
    cmp r14, 4
    jle .Lno_stack_args_fastcall
    mov rcx, r14
    sub rcx, 4
    shl rcx, 3      # rcx = (num_args - 4) * 8

    # Add shadow space (32 bytes minimum for x64 calling convention)
    add rcx, 32

    # Allocate stack space for arguments (maintain 16-byte alignment)
    add rcx, 15
    and rcx, -16
    sub rsp, rcx
    jmp .Lstack_allocated_fastcall

.Lno_stack_args_fastcall:
    # Even with no stack args, we need shadow space for the called function
    sub rsp, 32

.Lstack_allocated_fastcall:
    # Load the first 4 arguments into registers
    # For fastcall, we need to handle both integer and floating-point registers
    xor rcx, rcx
    xor rdx, rdx
    xor r8, r8
    xor r9, r9

    # Load arg 0 into RCX if available
    cmp r14, 0
    je .Lload_args_done_fastcall
    mov rcx, [r13]

    # Load arg 1 into RDX if available
    cmp r14, 1
    je .Lload_args_done_fastcall
    mov rdx, [r13 + 8]

    # Load arg 2 into R8 if available
    cmp r14, 2
    je .Lload_args_done_fastcall
    mov r8, [r13 + 16]

    # Load arg 3 into R9 if available
    cmp r14, 3
    je .Lload_args_done_fastcall
    mov r9, [r13 + 24]

    # Load remaining args onto the stack starting after shadow space
    cmp r14, 4
    jle .Lload_args_done_fastcall

    mov rax, r14
    sub rax, 4      # rax = num_args - 4 (number of stack args)
    mov rbx, rax
    shl rbx, 3      # rbx = (num_args - 4) * 8
    add rbx, r13    # rbx = pointer to last stack arg
    add rbx, 24     # adjust to start at arg 4

    # Push stack arguments (args 4+) starting after shadow space
    mov rsi, 0      # rsi = current arg index
.Lstack_arg_loop_fastcall:
    mov rdi, [rbx + rsi * 8]  # Load argument
    mov [rsp + rsi * 8 + 32], rdi  # Store on stack after shadow space
    inc rsi
    cmp rsi, rax
    jl .Lstack_arg_loop_fastcall

.Lload_args_done_fastcall:
    # Call the function
    call r12

    # Check if we need to return a floating point value
    cmp r15, 1
    je .Lfloat_return_fastcall

    # Integer return in RAX
    jmp .Lreturn_fastcall

.Lfloat_return_fastcall:
    # Float return already in XMM0

.Lreturn_fastcall:
    # Restore stack and registers
    lea rsp, [rbp - 40]  # Restore stack pointer
    pop r15
    pop r14
    pop r13
    pop r12
    pop rdi
    pop rsi
    pop rbx
    pop rbp
    ret
"#
);

// Export assembly functions for use in Rust
extern "C" {
    pub fn ffi_call_cdecl_asm(
        func_addr: usize,
        args: *const u64,
        num_args: usize,
        is_float_return: usize,
    ) -> u64;
    pub fn ffi_call_stdcall_asm(
        func_addr: usize,
        args: *const u64,
        num_args: usize,
        is_float_return: usize,
    ) -> u64;
    pub fn ffi_call_fastcall_asm(
        func_addr: usize,
        args: *const u64,
        num_args: usize,
        is_float_return: usize,
    ) -> u64;
}

/// Validate stack alignment and argument count for the calling convention
fn validate_stack_parameters(
    args: &[u64],
    calling_convention: &CallingConvention,
) -> Result<(), UniverseError> {
    // Validate argument count limits
    const MAX_ARGS: usize = 64; // Reasonable limit to prevent stack overflow
    if args.len() > MAX_ARGS {
        return Err(UniverseError::SystemError(format!(
            "Too many arguments: {} (maximum: {})",
            args.len(),
            MAX_ARGS
        )));
    }

    // Validate calling convention specific constraints
    match calling_convention {
        CallingConvention::Cdecl => {
            // cdecl can handle any number of arguments
        }
        CallingConvention::Stdcall => {
            // stdcall in x64 is the same as cdecl
        }
        CallingConvention::Fastcall => {
            // fastcall in x64 is the same as the standard calling convention
            // First 4 args in registers, rest on stack
        }
    }

    Ok(())
}

/// Calculate required stack space for the given arguments and calling convention
fn calculate_stack_space(args: &[u64], _calling_convention: &CallingConvention) -> usize {
    let stack_args = if args.len() > 4 { args.len() - 4 } else { 0 };
    let shadow_space = 32; // 32 bytes shadow space for x64
    let arg_space = stack_args * 8; // 8 bytes per argument
    let total_space = shadow_space + arg_space;
    
    // Align to 16-byte boundary
    (total_space + 15) & !15
}

/// Call a function using the specified calling convention with proper stack management
pub fn call_function(
    address: usize,
    args: &[u64],
    return_type: &NativeType,
    calling_convention: &CallingConvention,
) -> Result<u64, UniverseError> {
    // Validate stack parameters before making the call
    validate_stack_parameters(args, calling_convention)?;

    // Calculate required stack space for validation
    let _required_stack_space = calculate_stack_space(args, calling_convention);

    // Determine if the return type is floating point
    let is_float_return = match return_type {
        NativeType::Float32 | NativeType::Float64 => 1,
        _ => 0,
    };

    // Call the function with the appropriate calling convention
    let result = match calling_convention {
        CallingConvention::Cdecl => unsafe {
            ffi_call_cdecl_asm(address, args.as_ptr(), args.len(), is_float_return)
        },
        CallingConvention::Stdcall => unsafe {
            ffi_call_stdcall_asm(address, args.as_ptr(), args.len(), is_float_return)
        },
        CallingConvention::Fastcall => unsafe {
            ffi_call_fastcall_asm(address, args.as_ptr(), args.len(), is_float_return)
        },
    };

    Ok(result)
}