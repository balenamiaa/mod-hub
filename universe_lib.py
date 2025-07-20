"""
Universe Modding Framework - Type Definitions

This module provides comprehensive type annotations for the Universe modding framework,
enabling full IDE autocomplete support and static type checking for Python scripts.

The Universe framework provides memory access, function hooking, FFI capabilities,
and pointer utilities for game modding on Windows x64 platforms.
"""

from typing import (
    Any, Callable, Dict, List, Optional, Union, Type, TypeVar, Generic,
    Protocol, runtime_checkable, overload
)
from typing_extensions import Literal
from abc import ABC, abstractmethod
import sys

# Version information
__version__ = "1.0.0"
__author__ = "Universe Framework"

# =============================================================================
# Type Aliases and Constants
# =============================================================================

# Memory address type
Address = int

# Memory size type  
Size = int

# Byte data type
ByteData = bytes

# Pattern string for memory scanning (hex pattern with wildcards)
Pattern = str

# Module name for pattern scanning
ModuleName = str

# =============================================================================
# Calling Convention Types
# =============================================================================

CallingConvention = Literal["cdecl", "stdcall", "fastcall"]

# =============================================================================
# Native Type System
# =============================================================================

# Basic native types for FFI
NativeTypeName = Literal[
    # Integer types
    "int8", "i8", "char",
    "int16", "i16", "short",
    "int32", "i32", "int",
    "int64", "i64", "long",
    # Unsigned integer types
    "uint8", "u8", "uchar", "byte",
    "uint16", "u16", "ushort",
    "uint32", "u32", "uint",
    "uint64", "u64", "ulong",
    # Floating point types
    "float32", "f32", "float",
    "float64", "f64", "double",
    # Other types
    "pointer", "ptr", "usize",
    "cstring", "str", "string",
    "void"
]

# Structure type for FFI (format: "struct:StructName")
StructureTypeName = str  # e.g., "struct:PlayerData"

# Combined type for all FFI type names
FFITypeName = Union[NativeTypeName, StructureTypeName]

# =============================================================================
# TypeSpec Enum for Type-Safe Pointer Operations
# =============================================================================

class TypeSpec:
    """
    Type-safe specification for all supported data types in the Universe framework.
    
    This enum provides compile-time type safety for pointer operations and replaces
    string-based type specifications with proper enum-based system.
    
    Example:
        # Type-safe pointer creation
        ptr = universe.create_pointer(address, TypeSpec.INT32)
        
        # Instead of error-prone string-based approach
        ptr = universe.create_pointer(address, "int32")  # Deprecated
    """
    
    # Class attributes for type specifications
    INT32: 'TypeSpec'
    INT64: 'TypeSpec' 
    FLOAT32: 'TypeSpec'
    FLOAT64: 'TypeSpec'
    STRING: 'TypeSpec'
    BOOL: 'TypeSpec'
    UINT32: 'TypeSpec'
    UINT64: 'TypeSpec'
    POINTER: 'TypeSpec'
    
    def size(self) -> int:
        """
        Get the size in bytes for this type.
        
        Returns:
            Size in bytes, or 0 for variable-size types like strings
        """
        ...
    
    def name(self) -> str:
        """
        Get the name of this type as a string.
        
        Returns:
            Type name (e.g., "int32", "float64", "string")
        """
        ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

# =============================================================================
# Register Access System
# =============================================================================

class Registers:
    """
    Provides access to CPU register state during hook execution.
    
    This class allows reading and modifying all x64 CPU registers within
    hook callbacks, enabling full control over function parameters and
    execution state.
    
    Example:
        def my_hook(registers: Registers, original: OriginalFunction) -> None:
            # Read register values
            param1 = registers.rcx  # First parameter in Windows x64 calling convention
            param2 = registers.rdx  # Second parameter
            
            # Modify register values
            registers.rax = 0x12345678  # Change return value
            
            # Access floating-point registers
            xmm_data = registers.get_xmm(0)  # Get XMM0 register
            registers.set_xmm(1, 0x123456789abcdef0)  # Set XMM1 register
    """
    
    # General-purpose registers (64-bit)
    rax: int
    rbx: int
    rcx: int
    rdx: int
    rsi: int
    rdi: int
    rsp: int
    rbp: int
    r8: int
    r9: int
    r10: int
    r11: int
    r12: int
    r13: int
    r14: int
    r15: int
    
    # Flags register
    rflags: int
    
    def get_xmm(self, index: int) -> int:
        """
        Get the value of an XMM register as a 128-bit integer.
        
        Args:
            index: XMM register index (0-15)
            
        Returns:
            128-bit integer value of the XMM register
            
        Raises:
            IndexError: If index is not in range 0-15
        """
        ...
    
    def set_xmm(self, index: int, value: int) -> None:
        """
        Set the value of an XMM register from a 128-bit integer.
        
        Args:
            index: XMM register index (0-15)
            value: 128-bit integer value to set
            
        Raises:
            IndexError: If index is not in range 0-15
        """
        ...
    
    def get_xmm_bytes(self, index: int) -> bytes:
        """
        Get the value of an XMM register as 16 bytes.
        
        Args:
            index: XMM register index (0-15)
            
        Returns:
            16-byte representation of the XMM register
            
        Raises:
            IndexError: If index is not in range 0-15
        """
        ...
    
    def set_xmm_bytes(self, index: int, data: bytes) -> None:
        """
        Set the value of an XMM register from 16 bytes.
        
        Args:
            index: XMM register index (0-15)
            data: Exactly 16 bytes of data
            
        Raises:
            IndexError: If index is not in range 0-15
            ValueError: If data is not exactly 16 bytes
        """
        ...
    
    def __repr__(self) -> str: ...

# =============================================================================
# Hook System Types
# =============================================================================

class OriginalFunction:
    """
    Wrapper for calling the original function from within a function hook callback.
    
    This object is passed to function hook callbacks and allows calling the
    original function that was hooked, with the current register state.
    
    Example:
        def my_function_hook(registers: Registers, original: OriginalFunction) -> None:
            # Modify parameters before calling original
            registers.rcx = modified_param1
            
            # Call the original function with modified parameters
            original.call(registers)
            
            # Modify return value after original function returns
            registers.rax = modified_return_value
    """
    
    @property
    def address(self) -> Address:
        """Get the memory address of the original function."""
        ...
    
    def call(self, registers: Registers) -> None:
        """
        Call the original function with the provided register state.
        
        Args:
            registers: Register state to use when calling the original function.
                      The register state will be updated with the function's results.
        """
        ...

# Hook callback type definitions
FunctionHookCallback = Callable[[Registers, OriginalFunction], None]
JmpBackHookCallback = Callable[[Registers], None]

# =============================================================================
# FFI (Foreign Function Interface) System
# =============================================================================

class CallableFunction:
    """
    A callable Python object that wraps a native function with type information.
    
    Created by universe.create_function() to provide type-safe calling of
    arbitrary native functions from Python with proper argument and return
    value marshalling.
    
    Example:
        # Create a callable for a native function
        native_func = universe.create_function(
            address=0x12345678,
            arg_types=["int32", "float32"],
            return_type="int64",
            calling_convention="stdcall"
        )
        
        # Call the native function
        result = native_func(42, 3.14)  # Returns int64
    """
    
    def __call__(self, *args: Any) -> Any:
        """
        Call the native function with the provided arguments.
        
        Args:
            *args: Arguments to pass to the native function, must match
                  the types specified when creating this callable
                  
        Returns:
            Return value from the native function, converted to appropriate Python type
            
        Raises:
            ValueError: If argument count or types don't match the function signature
            RuntimeError: If the native function call fails
        """
        ...

# =============================================================================
# Pointer System
# =============================================================================

# Type variable for generic pointer operations
T = TypeVar('T')

class Pointer(Generic[T]):
    """
    Type-safe pointer for accessing basic data types in memory.
    
    Provides read and write access to primitive data types at specific memory
    addresses with automatic type conversion and validation.
    
    Example:
        # Create pointers for different types
        int_ptr = universe.create_pointer(address, TypeSpec.INT32)
        float_ptr = universe.create_pointer(address, TypeSpec.FLOAT64)
        string_ptr = universe.create_pointer(address, TypeSpec.STRING)
        
        # Read values
        int_value = int_ptr.read()      # Returns int
        float_value = float_ptr.read()  # Returns float
        string_value = string_ptr.read() # Returns str
        
        # Write values
        int_ptr.write(42)
        float_ptr.write(3.14159)
        string_ptr.write("Hello World")
    """
    
    @property
    def address(self) -> Address:
        """Get the memory address this pointer points to."""
        ...
    
    @property
    def type_name(self) -> str:
        """Get the type name of this pointer (e.g., "int32", "float64")."""
        ...
    
    @property
    def type_spec(self) -> TypeSpec:
        """Get the TypeSpec for this pointer."""
        ...
    
    def read(self) -> T:
        """
        Read the value from the memory address.
        
        Returns:
            Value at the memory address, converted to appropriate Python type
            
        Raises:
            RuntimeError: If memory read fails or address is invalid
        """
        ...
    
    def write(self, value: T) -> None:
        """
        Write a value to the memory address.
        
        Args:
            value: Value to write, must be compatible with the pointer's type
            
        Raises:
            RuntimeError: If memory write fails or address is invalid
            TypeError: If value type is incompatible with pointer type
        """
        ...
    
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

# =============================================================================
# Structure System
# =============================================================================

class Structure:
    """
    Base class for defining custom memory structure layouts.
    
    Inherit from this class to define Python classes that mirror game memory
    structures, enabling natural field access with automatic offset calculation
    and type conversion.
    
    Example:
        class PlayerData(Structure):
            health: int = 0      # Will be treated as int32 at offset 0
            mana: int = 4        # Will be treated as int32 at offset 4  
            position_x: float = 8    # Will be treated as float32 at offset 8
            position_y: float = 12   # Will be treated as float32 at offset 12
            name: str = 16       # Will be treated as string at offset 16
        
        # Create a pointer to the structure
        player_ptr = universe.create_pointer(player_address, PlayerData)
        
        # Access fields naturally
        current_health = player_ptr.health
        player_ptr.health = 100
        player_name = player_ptr.name
    """
    
    name: str
    
    def __init__(self) -> None: ...

class StructurePointer(Generic[T]):
    """
    Pointer to a custom structure with dynamic field access.
    
    Provides automatic field access for custom structure types with offset
    calculation, type conversion, and memory read/write operations.
    
    Example:
        class GameEntity(Structure):
            entity_id: int = 0
            health: int = 4
            position_x: float = 8
            position_y: float = 12
        
        # Create structure pointer
        entity_ptr = universe.create_pointer(entity_address, GameEntity)
        
        # Access fields with automatic type conversion
        entity_id = entity_ptr.entity_id    # Reads int32 from offset 0
        entity_ptr.health = 100             # Writes int32 to offset 4
        pos_x = entity_ptr.position_x       # Reads float32 from offset 8
    """
    
    @property
    def address(self) -> Address:
        """Get the base memory address of this structure."""
        ...
    
    def __getattr__(self, name: str) -> Any:
        """
        Get a field value from the structure.
        
        Args:
            name: Field name as defined in the structure class
            
        Returns:
            Field value converted to appropriate Python type
            
        Raises:
            AttributeError: If field name is not defined in the structure
            RuntimeError: If memory read fails
        """
        ...
    
    def __setattr__(self, name: str, value: Any) -> None:
        """
        Set a field value in the structure.
        
        Args:
            name: Field name as defined in the structure class
            value: Value to write, must be compatible with field type
            
        Raises:
            AttributeError: If field name is not defined in the structure
            RuntimeError: If memory write fails
            TypeError: If value type is incompatible with field type
        """
        ...

# Union type for all pointer types
AnyPointer = Union[Pointer[Any], StructurePointer[Any]]

# =============================================================================
# Core Universe Module Interface
# =============================================================================

# Memory Operations
def read_memory(address: Address, size: Size) -> ByteData:
    """
    Read raw bytes from a memory address.
    
    Args:
        address: Memory address to read from
        size: Number of bytes to read
        
    Returns:
        Raw bytes read from memory
        
    Raises:
        MemoryError: If memory access fails or address is invalid
        RuntimeError: If Universe core is not initialized
        
    Example:
        # Read 16 bytes from a specific address
        data = universe.read_memory(0x12345678, 16)
        print(f"Read {len(data)} bytes: {data.hex()}")
    """
    ...

def write_memory(address: Address, data: ByteData) -> None:
    """
    Write raw bytes to a memory address.
    
    Args:
        address: Memory address to write to
        data: Bytes to write to memory
        
    Raises:
        MemoryError: If memory access fails or address is invalid
        RuntimeError: If Universe core is not initialized
        
    Example:
        # Write specific bytes to memory
        data = b'\x90\x90\x90\x90'  # NOP instructions
        universe.write_memory(0x12345678, data)
    """
    ...

def pattern_scan(module_name: ModuleName, pattern: Pattern) -> Optional[Address]:
    """
    Search for a byte pattern within a specific loaded module.
    
    Args:
        module_name: Name of the module to search (e.g., "game.exe", "engine.dll")
        pattern: Hex pattern string with wildcards (e.g., "48 8B ? ? 89 45")
                Use ? for wildcard bytes that can match any value
                
    Returns:
        Memory address of the first match, or None if pattern not found
        
    Raises:
        RuntimeError: If module is not loaded or scan fails
        
    Example:
        # Find a specific function pattern
        addr = universe.pattern_scan("game.exe", "48 8B 05 ? ? ? ? 48 85 C0")
        if addr:
            print(f"Pattern found at: 0x{addr:X}")
        else:
            print("Pattern not found")
    """
    ...

# Hook System
def hook_function(address: Address, callback: FunctionHookCallback) -> None:
    """
    Install a function hook at the specified address.
    
    The callback will be executed whenever the hooked function is called,
    receiving the CPU register state and an object to call the original function.
    
    Args:
        address: Memory address of the function to hook
        callback: Python function to call when hook is triggered.
                 Must accept (registers: Registers, original: OriginalFunction)
                 
    Raises:
        RuntimeError: If hook installation fails or address is invalid
        
    Example:
        def my_function_hook(registers: Registers, original: OriginalFunction) -> None:
            print(f"Function called with RCX={registers.rcx:X}")
            
            # Optionally call original function
            original.call(registers)
            
            # Modify return value
            registers.rax = 42
        
        universe.hook_function(0x12345678, my_function_hook)
    """
    ...

def hook_jmpback(address: Address, callback: JmpBackHookCallback) -> None:
    """
    Install a jmpback hook at the specified address.
    
    The callback will be executed at the specified location, then execution
    continues normally. Unlike function hooks, jmpback hooks don't provide
    access to the original function.
    
    Args:
        address: Memory address to hook (can be mid-function)
        callback: Python function to call when hook is triggered.
                 Must accept (registers: Registers) only
                 
    Raises:
        RuntimeError: If hook installation fails or address is invalid
        
    Example:
        def my_jmpback_hook(registers: Registers) -> None:
            print(f"Execution reached 0x{registers.rip:X}")
            # Modify registers if needed
            registers.rax = 0
        
        universe.hook_jmpback(0x12345678, my_jmpback_hook)
    """
    ...

def remove_hook(address: Address) -> None:
    """
    Remove a hook at the specified address.
    
    Args:
        address: Memory address of the hook to remove
        
    Raises:
        RuntimeError: If no hook exists at the address or removal fails
        
    Example:
        # Remove a previously installed hook
        universe.remove_hook(0x12345678)
    """
    ...

# FFI System
@overload
def create_function(
    address: Address,
    arg_types: List[FFITypeName],
    return_type: FFITypeName
) -> CallableFunction: ...

@overload  
def create_function(
    address: Address,
    arg_types: List[FFITypeName], 
    return_type: FFITypeName,
    calling_convention: CallingConvention
) -> CallableFunction: ...

def create_function(
    address: Address,
    arg_types: List[FFITypeName],
    return_type: FFITypeName,
    calling_convention: Optional[CallingConvention] = None
) -> CallableFunction:
    """
    Create a callable Python object from a native function address.
    
    Args:
        address: Memory address of the native function
        arg_types: List of argument type names (e.g., ["int32", "float32"])
                  For structure types, use "struct:StructName" format
        return_type: Return type name (e.g., "int64", "struct:PlayerData")
        calling_convention: Calling convention ("cdecl", "stdcall", "fastcall").
                           Defaults to "cdecl" if not specified.
                           
    Returns:
        Callable object that can be invoked from Python
        
    Raises:
        ValueError: If type names or calling convention are invalid
        RuntimeError: If function creation fails
        
    Example:
        # Create a callable for a native function with basic types
        native_func = universe.create_function(
            address=0x12345678,
            arg_types=["int32", "float32", "pointer"],
            return_type="int64",
            calling_convention="stdcall"
        )
        
        # Call the function
        result = native_func(42, 3.14, 0x87654321)
        print(f"Function returned: {result}")
        
        # Create a callable for a function with structure parameters
        class PlayerData(Structure):
            health: int = 0
            mana: int = 4
            
        player_ptr = universe.create_pointer(0x12345690, PlayerData)
        
        get_player_name = universe.create_function(
            address=0x12345700,
            arg_types=["struct:PlayerData"],  # Pass structure by reference
            return_type="cstring",
            calling_convention="cdecl"
        )
        
        # Call with structure pointer
        player_name = get_player_name(player_ptr)
    """
    ...

# Pointer System
@overload
def create_pointer(address: Address, type_spec: TypeSpec) -> Pointer[Any]: ...

@overload
def create_pointer(address: Address, type_spec: str) -> Pointer[Any]: ...

@overload  
def create_pointer(address: Address, structure_class: Type[T]) -> StructurePointer[T]: ...

def create_pointer(
    address: Address, 
    type_spec: Union[TypeSpec, str, Type[Structure]]
) -> AnyPointer:
    """
    Create a typed pointer for accessing memory with automatic type conversion.
    
    Args:
        address: Memory address to point to
        type_spec: Type specification - either:
                  - TypeSpec enum for basic types (recommended)
                  - String for basic types (deprecated, use TypeSpec)
                  - Structure class for complex types
                  
    Returns:
        Appropriate pointer object for the specified type
        
    Raises:
        ValueError: If type specification is invalid
        RuntimeError: If pointer creation fails
        
    Example:
        # Basic type pointers (recommended approach)
        int_ptr = universe.create_pointer(0x12345678, TypeSpec.INT32)
        float_ptr = universe.create_pointer(0x12345680, TypeSpec.FLOAT64)
        
        # String-based types (deprecated but supported)
        old_ptr = universe.create_pointer(0x12345688, "int32")
        
        # Structure pointers
        class PlayerData(Structure):
            health: int = 0
            mana: int = 4
            
        player_ptr = universe.create_pointer(0x12345690, PlayerData)
    """
    ...

# Alias for backward compatibility
Pointer = create_pointer

# Logging
def log(message: str) -> None:
    """
    Log a message to the universe.log file.
    
    Args:
        message: Message to log
        
    Example:
        universe.log("Hook installed successfully")
        universe.log(f"Player health: {player_ptr.health}")
    """
    ...

# =============================================================================
# Module Metadata
# =============================================================================

# Module version
__version__: str

# All exported symbols
__all__ = [
    # Type system
    "TypeSpec",
    "CallingConvention", 
    "NativeTypeName",
    "StructureTypeName",
    "FFITypeName",
    "Address",
    "Size", 
    "ByteData",
    "Pattern",
    "ModuleName",
    
    # Register system
    "Registers",
    
    # Hook system
    "OriginalFunction",
    "FunctionHookCallback",
    "JmpBackHookCallback",
    
    # FFI system
    "CallableFunction",
    
    # Pointer system
    "Pointer",
    "Structure", 
    "StructurePointer",
    "AnyPointer",
    
    # Core functions
    "read_memory",
    "write_memory", 
    "pattern_scan",
    "hook_function",
    "hook_jmpback", 
    "remove_hook",
    "create_function",
    "create_pointer",
    "log",
]