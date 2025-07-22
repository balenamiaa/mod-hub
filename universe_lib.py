"""
Universe Modding Framework - Type Definitions

This module provides comprehensive type annotations for the Universe modding framework,
enabling full IDE autocomplete support and static type checking for Python scripts.

The Universe framework provides memory access, function hooking, FFI capabilities,
and pointer utilities for game modding on Windows x64 platforms.
"""

import ctypes
import os
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

# Assuming the DLL is in the same directory as the Python script
_universe_dll_path = os.path.join(os.path.dirname(__file__), "mod_hub.dll")
_UniverseDLL = ctypes.WinDLL(_universe_dll_path)

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

# Combined type for all FFI type names (only native types now)
FFITypeName = NativeTypeName

# =============================================================================
# TypeSpec for Type-Safe Pointer Operations
# =============================================================================

class TypeSpec:
    """
    Type-safe specification for all supported data types in the Universe framework.

    This class provides type safety for pointer operations and replaces
    string-based type specifications with a more robust system.

    Example:
        # Type-safe pointer creation
        ptr = universe.create_pointer(address, TypeSpec.INT32)

        # Instead of error-prone string-based approach
        ptr = universe.create_pointer(address, "int32")  # Deprecated
    """

    def __init__(self, ctypes_type: Optional[Type[ctypes._CData]], name: str):
        self._ctypes_type = ctypes_type
        self._name = name

    @property
    def ctypes_type(self) -> Optional[Type[ctypes._CData]]:
        return self._ctypes_type

    def size(self) -> int:
        """
        Get the size in bytes for this type.

        Returns:
            Size in bytes, or 0 for variable-size types like strings (CSTRING)
            or void.
        """
        if self._ctypes_type is None: # VOID
            return 0
        if self._ctypes_type == ctypes.c_char_p: # CSTRING
            return 0 # Variable size, handled by Python
        return ctypes.sizeof(self._ctypes_type)

    def name(self) -> str:
        """
        Get the name of this type as a string.

        Returns:
            Type name (e.g., "int32", "float64", "cstring")
        """
        return self._name

    def __repr__(self) -> str:
        return f"TypeSpec.{self._name.upper()}"

    def __str__(self) -> str:
        return self._name

# Instantiate TypeSpec objects
TypeSpec.INT8 = TypeSpec(ctypes.c_int8, "int8")
TypeSpec.INT16 = TypeSpec(ctypes.c_int16, "int16")
TypeSpec.INT32 = TypeSpec(ctypes.c_int32, "int32")
TypeSpec.INT64 = TypeSpec(ctypes.c_int64, "int64")
TypeSpec.UINT8 = TypeSpec(ctypes.c_uint8, "uint8")
TypeSpec.UINT16 = TypeSpec(ctypes.c_uint16, "uint16")
TypeSpec.UINT32 = TypeSpec(ctypes.c_uint32, "uint32")
TypeSpec.UINT64 = TypeSpec(ctypes.c_uint64, "uint64")
TypeSpec.FLOAT32 = TypeSpec(ctypes.c_float, "float32")
TypeSpec.FLOAT64 = TypeSpec(ctypes.c_double, "float64")
TypeSpec.POINTER = TypeSpec(ctypes.c_void_p, "pointer")
TypeSpec.CSTRING = TypeSpec(ctypes.c_char_p, "cstring")
TypeSpec.VOID = TypeSpec(None, "void")

# Aliases for common types
TypeSpec.I8 = TypeSpec.INT8
TypeSpec.CHAR = TypeSpec.INT8
TypeSpec.I16 = TypeSpec.INT16
TypeSpec.SHORT = TypeSpec.INT16
TypeSpec.I32 = TypeSpec.INT32
TypeSpec.INT = TypeSpec.INT32
TypeSpec.I64 = TypeSpec.INT64
TypeSpec.LONG = TypeSpec.INT64
TypeSpec.U8 = TypeSpec.UINT8
TypeSpec.UCHAR = TypeSpec.UINT8
TypeSpec.BYTE = TypeSpec.UINT8
TypeSpec.U16 = TypeSpec.UINT16
TypeSpec.USHORT = TypeSpec.UINT16
TypeSpec.U32 = TypeSpec.UINT32
TypeSpec.UINT = TypeSpec.UINT32
TypeSpec.U64 = TypeSpec.UINT64
TypeSpec.ULONG = TypeSpec.UINT64
TypeSpec.F32 = TypeSpec.FLOAT32
TypeSpec.FLOAT = TypeSpec.FLOAT32
TypeSpec.F64 = TypeSpec.FLOAT64
TypeSpec.DOUBLE = TypeSpec.FLOAT64
TypeSpec.PTR = TypeSpec.POINTER
TypeSpec.USIZE = TypeSpec.POINTER # usize in Rust is pointer-sized
TypeSpec.STR = TypeSpec.CSTRING
TypeSpec.STRING = TypeSpec.CSTRING # Alias for cstring

# =============================================================================
# Register Access System
# =============================================================================

# Define the WinContext structure to mirror the Rust side
class WinContext(ctypes.Structure):
    _fields_ = [
        ("rax", ctypes.c_uint64),
        ("rbx", ctypes.c_uint64),
        ("rcx", ctypes.c_uint64),
        ("rdx", ctypes.c_uint64),
        ("rsi", ctypes.c_uint64),
        ("rdi", ctypes.c_uint64),
        ("rsp", ctypes.c_uint64),
        ("rbp", ctypes.c_uint64),
        ("r8", ctypes.c_uint64),
        ("r9", ctypes.c_uint64),
        ("r10", ctypes.c_uint64),
        ("r11", ctypes.c_uint64),
        ("r12", ctypes.c_uint64),
        ("r13", ctypes.c_uint64),
        ("r14", ctypes.c_uint64),
        ("r15", ctypes.c_uint64),
        ("eflags", ctypes.c_uint32),
        # XMM registers are 16 bytes each (represented as two 64-bit integers)
        ("xmm0", ctypes.c_uint64 * 2),
        ("xmm1", ctypes.c_uint64 * 2),
        ("xmm2", ctypes.c_uint64 * 2),
        ("xmm3", ctypes.c_uint64 * 2),
        ("xmm4", ctypes.c_uint64 * 2),
        ("xmm5", ctypes.c_uint64 * 2),
        ("xmm6", ctypes.c_uint64 * 2),
        ("xmm7", ctypes.c_uint64 * 2),
        ("xmm8", ctypes.c_uint64 * 2),
        ("xmm9", ctypes.c_uint64 * 2),
        ("xmm10", ctypes.c_uint64 * 2),
        ("xmm11", ctypes.c_uint64 * 2),
        ("xmm12", ctypes.c_uint64 * 2),
        ("xmm13", ctypes.c_uint64 * 2),
        ("xmm14", ctypes.c_uint64 * 2),
        ("xmm15", ctypes.c_uint64 * 2),
    ]

class Registers:
    """
    Provides access to CPU register state during hook execution.
    
    This class allows reading and modifying all x64 CPU registers within
    hook callbacks, enabling full control over function parameters and
    execution state.
    """
    def __init__(self, context_ptr: ctypes.POINTER(WinContext)):
        self._context_ptr = context_ptr
        self._context = context_ptr.contents

    @property
    def rax(self) -> int: return self._context.rax
    @rax.setter
    def rax(self, value: int): self._context.rax = value

    @property
    def rbx(self) -> int: return self._context.rbx
    @rbx.setter
    def rbx(self, value: int): self._context.rbx = value

    @property
    def rcx(self) -> int: return self._context.rcx
    @rcx.setter
    def rcx(self, value: int): self._context.rcx = value

    @property
    def rdx(self) -> int: return self._context.rdx
    @rdx.setter
    def rdx(self, value: int): self._context.rdx = value

    @property
    def rsi(self) -> int: return self._context.rsi
    @rsi.setter
    def rsi(self, value: int): self._context.rsi = value

    @property
    def rdi(self) -> int: return self._context.rdi
    @rdi.setter
    def rdi(self, value: int): self._context.rdi = value

    @property
    def rsp(self) -> int: return self._context.rsp
    @rsp.setter
    def rsp(self, value: int): self._context.rsp = value

    @property
    def rbp(self) -> int: return self._context.rbp
    @rbp.setter
    def rbp(self, value: int): self._context.rbp = value

    @property
    def r8(self) -> int: return self._context.r8
    @r8.setter
    def r8(self, value: int): self._context.r8 = value

    @property
    def r9(self) -> int: return self._context.r9
    @r9.setter
    def r9(self, value: int): self._context.r9 = value

    @property
    def r10(self) -> int: return self._context.r10
    @r10.setter
    def r10(self, value: int): self._context.r10 = value

    @property
    def r11(self) -> int: return self._context.r11
    @r11.setter
    def r11(self, value: int): self._context.r11 = value

    @property
    def r12(self) -> int: return self._context.r12
    @r12.setter
    def r12(self, value: int): self._context.r12 = value

    @property
    def r13(self) -> int: return self._context.r13
    @r13.setter
    def r13(self, value: int): self._context.r13 = value

    @property
    def r14(self) -> int: return self._context.r14
    @r14.setter
    def r14(self, value: int): self._context.r14 = value

    @property
    def r15(self) -> int: return self._context.r15
    @r15.setter
    def r15(self, value: int): self._context.r15 = value

    @property
    def rflags(self) -> int: return self._context.eflags # eflags is 32-bit, rflags is 64-bit
    @rflags.setter
    def rflags(self, value: int): self._context.eflags = value & 0xFFFFFFFF # Mask to 32-bit

    def get_xmm(self, index: int) -> int:
        """
        Get the value of an XMM register as a 128-bit integer.
        """
        if not 0 <= index <= 15:
            raise IndexError("XMM register index out of range (0-15)")
        xmm_array = getattr(self._context, f"xmm{index}")
        # Combine two 64-bit integers into a 128-bit integer
        return (xmm_array[1] << 64) | xmm_array[0]

    def set_xmm(self, index: int, value: int) -> None:
        """
        Set the value of an XMM register from a 128-bit integer.
        """
        if not 0 <= index <= 15:
            raise IndexError("XMM register index out of range (0-15)")
        xmm_array = getattr(self._context, f"xmm{index}")
        xmm_array[0] = value & 0xFFFFFFFFFFFFFFFF
        xmm_array[1] = (value >> 64) & 0xFFFFFFFFFFFFFFFF

    def get_xmm_bytes(self, index: int) -> bytes:
        """
        Get the value of an XMM register as 16 bytes.
        """
        if not 0 <= index <= 15:
            raise IndexError("XMM register index out of range (0-15)")
        xmm_array = getattr(self._context, f"xmm{index}")
        # Convert two uint64s to bytes
        return xmm_array[0].to_bytes(8, 'little') + xmm_array[1].to_bytes(8, 'little')

    def set_xmm_bytes(self, index: int, data: bytes) -> None:
        """
        Set the value of an XMM register from 16 bytes.
        """
        if not 0 <= index <= 15:
            raise IndexError("XMM register index out of range (0-15)")
        if len(data) != 16:
            raise ValueError("Data must be exactly 16 bytes for XMM register")
        xmm_array = getattr(self._context, f"xmm{index}")
        xmm_array[0] = int.from_bytes(data[0:8], 'little')
        xmm_array[1] = int.from_bytes(data[8:16], 'little')

    def __repr__(self) -> str:
        return f"<Registers rax=0x{self.rax:X} rcx=0x{self.rcx:X} ...>"

# =============================================================================
# Hook System Types
# =============================================================================

class OriginalFunction:
    """
    Wrapper for calling the original function from within a function hook callback.
    
    This object is passed to function hook callbacks and allows calling the
    original function that was hooked, with the current register state.
    """
    def __init__(self, original_address: Address, original_bytes: bytes):
        self._original_address = original_address
        self._original_bytes = original_bytes # Not directly used by Python, but kept for context
        self._called = False

    @property
    def address(self) -> Address:
        """Get the memory address of the original function."""
        return self._original_address

    def call(self, registers: Registers) -> None:
        """
        Call the original function with the provided register state.
        
        Args:
            registers: Register state to use when calling the original function.
                      The register state will be updated with the function's results.
        """
        self._called = True

    # This method is for internal use by the Rust side to check if original was called
    def _was_called(self) -> bool:
        return self._called

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
    """
    def __init__(self, address: Address, arg_types: List[Union[TypeSpec, Type["Structure"]]],
                 return_type: Union[TypeSpec, Type["Structure"]], calling_convention: CallingConvention):
        self._address = address
        self._arg_types = arg_types
        self._return_type = return_type
        self._calling_convention = calling_convention

        self._c_arg_types = []
        for arg_type in arg_types:
            if isinstance(arg_type, TypeSpec):
                self._c_arg_types.append(arg_type.ctypes_type)
            elif issubclass(arg_type, Structure):
                # For structures, we pass a pointer to the structure
                self._c_arg_types.append(ctypes.POINTER(arg_type))
            else:
                raise ValueError(f"Invalid argument type: {arg_type}")

        if isinstance(return_type, TypeSpec):
            self._c_return_type = return_type.ctypes_type
        elif issubclass(return_type, Structure):
            # For structures, we expect a pointer to the structure to be returned
            self._c_return_type = ctypes.POINTER(return_type)
        else:
            raise ValueError(f"Invalid return type: {return_type}")

        # Determine the ctypes calling convention
        if calling_convention == "cdecl":
            self._c_function_type = ctypes.CFUNCTYPE(self._c_return_type, *self._c_arg_types)
        elif calling_convention == "stdcall":
            self._c_function_type = ctypes.WINFUNCTYPE(self._c_return_type, *self._c_arg_types)
        elif calling_convention == "fastcall":
            # fastcall is not directly supported by ctypes, it's usually handled by the compiler.
            # For simplicity, we'll treat it as cdecl for now, but this might need
            # more advanced assembly if true fastcall behavior is required.
            self._c_function_type = ctypes.CFUNCTYPE(self._c_return_type, *self._c_arg_types)
        else:
            raise ValueError(f"Unsupported calling convention: {calling_convention}")

        self._native_function = self._c_function_type(self._address)

    def __call__(self, *args: Any) -> Any:
        """
        Call the native function with the provided arguments.
        """
        if len(args) != len(self._arg_types):
            raise ValueError(f"Expected {len(self._arg_types)} arguments, got {len(args)}")

        processed_args = []
        for i, arg in enumerate(args):
            expected_type = self._arg_types[i]
            if isinstance(expected_type, TypeSpec):
                if expected_type == TypeSpec.CSTRING:
                    processed_args.append(arg.encode('utf-8')) # Convert Python str to bytes for cstring
                elif expected_type == TypeSpec.POINTER:
                    if isinstance(arg, Pointer):
                        processed_args.append(arg.address)
                    elif isinstance(arg, int):
                        processed_args.append(arg)
                    else:
                        raise TypeError(f"Expected int or Pointer for pointer type, got {type(arg)}")
                else:
                    processed_args.append(arg)
            elif issubclass(expected_type, Structure):
                if isinstance(arg, StructurePointer) and arg.structure_class == expected_type:
                    processed_args.append(arg.address) # Pass the address of the structure
                else:
                    raise TypeError(f"Expected StructurePointer of type {expected_type.__name__}, got {type(arg)}")
            else:
                raise ValueError(f"Unexpected argument type specification: {expected_type}")

        result = self._native_function(*processed_args)

        if isinstance(self._return_type, TypeSpec):
            if self._return_type == TypeSpec.CSTRING:
                return result.decode('utf-8') if result else None
            return result
        elif issubclass(self._return_type, Structure):
            # If a structure pointer is returned, wrap it in StructurePointer
            if result:
                return StructurePointer(result, self._return_type)
            return None
        return result

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
    """
    def __init__(self, address: Address, type_spec: TypeSpec):
        self._address = address
        self._type_spec = type_spec
        self._ctypes_ptr = ctypes.cast(address, ctypes.POINTER(type_spec.ctypes_type))

    @property
    def address(self) -> Address:
        """Get the memory address this pointer points to."""
        return self._address

    @property
    def type_name(self) -> str:
        """Get the type name of this pointer (e.g., "int32", "float64")."""
        return self._type_spec.name()

    @property
    def type_spec(self) -> TypeSpec:
        """Get the TypeSpec for this pointer."""
        return self._type_spec

    def read(self) -> T:
        """
        Read the value from the memory address.
        """
        if self._type_spec == TypeSpec.CSTRING:
            max_len = 256 # Arbitrary max length for CSTRING
            buffer = (ctypes.c_char * max_len)()
            result = _UniverseDLL.read_memory(self._address, ctypes.addressof(buffer), max_len)
            if result != 0:
                raise MemoryError(f"Failed to read CSTRING from 0x{self._address:X}")
            try:
                return buffer.value.decode('utf-8')
            except UnicodeDecodeError:
                raise ValueError(f"Invalid UTF-8 sequence in CSTRING at 0x{self._address:X}")
        try:
            return self._ctypes_ptr.contents.value
        except Exception as e:
            raise RuntimeError(f"Failed to read memory at 0x{self._address:X}: {e}")

    def write(self, value: T) -> None:
        """
        Write a value to the memory address.
        """
        if self._type_spec == TypeSpec.CSTRING:
            encoded_value = value.encode('utf-8') + b'\0' # Null-terminate
            buffer = ctypes.create_string_buffer(encoded_value)
            result = _UniverseDLL.write_memory(self._address, ctypes.addressof(buffer), len(encoded_value))
            if result != 0:
                raise MemoryError(f"Failed to write CSTRING to 0x{self._address:X}")
            return

        try:
            self._ctypes_ptr.contents.value = value
        except Exception as e:
            raise RuntimeError(f"Failed to write memory at 0x{self._address:X}: {e}")

    def __repr__(self) -> str:
        return f"<Pointer 0x{self.address:X}, type={self.type_name}>"

    def __str__(self) -> str:
        return self.__repr__()

# =============================================================================
# Structure System
# =============================================================================

class Structure(ctypes.Structure):
    """
    Base class for defining custom memory structure layouts.

    Inherit from this class to define Python classes that mirror game memory
    structures, enabling natural field access with automatic offset calculation
    and type conversion.

    Users should define `_fields_` in their subclasses, mapping field names
    to ctypes types.

    Example:
        class PlayerData(Structure):
            _fields_ = [
                ("health", ctypes.c_int32),
                ("mana", ctypes.c_int32),
                ("position_x", ctypes.c_float),
                ("position_y", ctypes.c_float),
                ("name_ptr", ctypes.c_void_p), # For string, usually a pointer
            ]
            # Add properties for easier access to complex types like strings
            @property
            def name(self) -> str:
                if self.name_ptr:
                    return Pointer(self.name_ptr, TypeSpec.CSTRING).read()
                return ""
            @name.setter
            def name(self, value: str):
                if self.name_ptr:
                    Pointer(self.name_ptr, TypeSpec.CSTRING).write(value)
                else:
                    log("Warning: Cannot write string to null name_ptr. Allocate memory first.")
    """
    pass # Users will define _fields_ in their subclasses

class StructurePointer(Generic[T]):
    """
    Pointer to a custom structure with dynamic field access.

    Provides automatic field access for custom structure types with offset
    calculation, type conversion, and memory read/write operations.
    """
    def __init__(self, address: Address, structure_class: Type[Structure]):
        if not issubclass(structure_class, Structure):
            raise TypeError("structure_class must be a subclass of Structure")
        if not hasattr(structure_class, '_fields_'):
            raise ValueError(f"Structure class {structure_class.__name__} must define _fields_")

        self._address = address
        self._structure_class = structure_class
        self._ctypes_struct_ptr = ctypes.cast(address, ctypes.POINTER(structure_class))

    @property
    def address(self) -> Address:
        """Get the base memory address of this structure."""
        return self._address

    @property
    def structure_class(self) -> Type[Structure]:
        """Get the Python Structure class associated with this pointer."""
        return self._structure_class

    def __getattr__(self, name: str) -> Any:
        """
        Get a field value from the structure.
        """
        # Direct access to ctypes structure members
        try:
            return getattr(self._ctypes_struct_ptr.contents, name)
        except AttributeError:
            # If not a direct field, check for properties in the structure_class
            if hasattr(self._structure_class, name):
                attr = getattr(self._structure_class, name)
                if isinstance(attr, property):
                    return attr.__get__(self._ctypes_struct_ptr.contents, self._structure_class)
            raise AttributeError(f"Field or property '{name}' not found in structure {self._structure_class.__name__}")
        except Exception as e:
            raise RuntimeError(f"Failed to read field '{name}' from 0x{self._address:X}: {e}")

    def __setattr__(self, name: str, value: Any) -> None:
        """
        Set a field value in the structure.
        """
        # Allow setting internal attributes directly
        if name.startswith('_'):
            super().__setattr__(name, value)
            return

        # Direct access to ctypes structure members
        try:
            setattr(self._ctypes_struct_ptr.contents, name, value)
        except AttributeError:
            # If not a direct field, check for properties with setters in the structure_class
            if hasattr(self._structure_class, name):
                attr = getattr(self._structure_class, name)
                if isinstance(attr, property) and attr.fset is not None:
                    attr.fset(self._ctypes_struct_ptr.contents, value)
                    return
            raise AttributeError(f"Field or property '{name}' not found or not settable in structure {self._structure_class.__name__}")
        except Exception as e:
            raise RuntimeError(f"Failed to write field '{name}' to 0x{self._address:X}: {e}")

# Union type for all pointer types
AnyPointer = Union[Pointer[Any], StructurePointer[Any]]

# =============================================================================
# Core Universe Module Interface
# =============================================================================

# Configure the Rust DLL functions
_UniverseDLL.read_memory.argtypes = [ctypes.c_uint64, ctypes.c_void_p, ctypes.c_size_t]
_UniverseDLL.read_memory.restype = ctypes.c_int # Returns 0 on success, -1 on failure

_UniverseDLL.write_memory.argtypes = [ctypes.c_uint64, ctypes.c_void_p, ctypes.c_size_t]
_UniverseDLL.write_memory.restype = ctypes.c_int # Returns 0 on success, -1 on failure

_UniverseDLL.pattern_scan.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
_UniverseDLL.pattern_scan.restype = ctypes.c_uint64 # Returns address or 0

# Define the C-compatible callback types
# For FunctionHookCallback: (context_ptr: *mut WinContext, hook_address: usize, original_function_address: usize) -> i32
_FunctionHookCallbackC = ctypes.CFUNCTYPE(
    ctypes.c_int, # Return value: 0 to call original, 1 to skip, -1 for error
    ctypes.POINTER(WinContext), # context_ptr
    ctypes.c_uint64, # hook_address
    ctypes.c_uint64 # original_function_address
)

# For JmpBackHookCallback: (context_ptr: *mut WinContext, hook_address: usize) -> i32
_JmpBackHookCallbackC = ctypes.CFUNCTYPE(
    ctypes.c_int, # Return value: 0 on success, -1 on failure
    ctypes.POINTER(WinContext), # context_ptr
    ctypes.c_uint64 # hook_address
)

_UniverseDLL.hook_function.argtypes = [ctypes.c_uint64, _FunctionHookCallbackC]
_UniverseDLL.hook_function.restype = ctypes.c_int # Returns 0 on success, -1 on failure

_UniverseDLL.hook_jmpback.argtypes = [ctypes.c_uint64, _JmpBackHookCallbackC]
_UniverseDLL.hook_jmpback.restype = ctypes.c_int # Returns 0 on success, -1 on failure

_UniverseDLL.remove_hook.argtypes = [ctypes.c_uint64]
_UniverseDLL.remove_hook.restype = ctypes.c_int # Returns 0 on success, -1 on failure

_UniverseDLL.log.argtypes = [ctypes.c_char_p]
_UniverseDLL.log.restype = None

# Store references to Python callbacks to prevent them from being garbage collected
_HOOK_CALLBACKS: Dict[Address, Any] = {}
_JMPBACK_CALLBACKS: Dict[Address, Any] = {}

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
    buffer = ctypes.create_string_buffer(size)
    # Pass the address of the buffer to the C function
    result = _UniverseDLL.read_memory(address, ctypes.addressof(buffer), size)
    if result != 0:
        raise MemoryError(f"Failed to read {size} bytes from 0x{address:X}")
    return buffer.raw

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
    buffer = ctypes.create_string_buffer(data)
    # Pass the address of the buffer to the C function
    result = _UniverseDLL.write_memory(address, ctypes.addressof(buffer), len(data))
    if result != 0:
        raise MemoryError(f"Failed to write {len(data)} bytes to 0x{address:X}")

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
    module_name_bytes = module_name.encode('utf-8')
    pattern_bytes = pattern.encode('utf-8')
    address = _UniverseDLL.pattern_scan(module_name_bytes, pattern_bytes)
    return address if address != 0 else None

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
    @_FunctionHookCallbackC
    def _c_callback(context_ptr: ctypes.POINTER(WinContext), hook_address: ctypes.c_uint64, original_function_address: ctypes.c_uint64) -> int:
        registers = Registers(context_ptr)
        original_func = OriginalFunction(original_function_address, b'') # Original bytes are handled by Rust
        try:
            callback(registers, original_func)
            return 0 if original_func._was_called() else 1 # 0 to call original, 1 to skip
        except Exception as e:
            log(f"Error in function hook callback at 0x{address:X}: {e}")
            return -1 # Indicate error to Rust

    _HOOK_CALLBACKS[address] = _c_callback # Store reference to prevent GC
    result = _UniverseDLL.hook_function(address, _c_callback)
    if result != 0:
        del _HOOK_CALLBACKS[address] # Remove if hook failed
        raise RuntimeError(f"Failed to install function hook at 0x{address:X}")

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
    @_JmpBackHookCallbackC
    def _c_callback(context_ptr: ctypes.POINTER(WinContext), hook_address: ctypes.c_uint64) -> int:
        registers = Registers(context_ptr)
        try:
            callback(registers)
            return 0 # Success
        except Exception as e:
            log(f"Error in jmpback hook callback at 0x{address:X}: {e}")
            return -1 # Indicate error to Rust

    _JMPBACK_CALLBACKS[address] = _c_callback # Store reference to prevent GC
    result = _UniverseDLL.hook_jmpback(address, _c_callback)
    if result != 0:
        del _JMPBACK_CALLBACKS[address] # Remove if hook failed
        raise RuntimeError(f"Failed to install jmpback hook at 0x{address:X}")

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
    result = _UniverseDLL.remove_hook(address)
    if result != 0:
        raise RuntimeError(f"Failed to remove hook at 0x{address:X}")
    _HOOK_CALLBACKS.pop(address, None)
    _JMPBACK_CALLBACKS.pop(address, None)

# FFI System
@overload
def create_function(
    address: Address,
    arg_types: List[Union[TypeSpec, Type["Structure"]]],
    return_type: Union[TypeSpec, Type["Structure"]]
) -> CallableFunction: ...

@overload
def create_function(
    address: Address,
    arg_types: List[Union[TypeSpec, Type["Structure"]]],
    return_type: Union[TypeSpec, Type["Structure"]],
    calling_convention: CallingConvention
) -> CallableFunction: ...

def create_function(
    address: Address,
    arg_types: List[Union[TypeSpec, Type["Structure"]]],
    return_type: Union[TypeSpec, Type["Structure"]],
    calling_convention: Optional[CallingConvention] = None
) -> CallableFunction:
    """
    Create a callable Python object from a native function address.
    
    Args:
        address: Memory address of the native function
        arg_types: List of argument type names (e.g., ["int32", "float32"])
                  For structure types, use the Structure class directly
        return_type: Return type name (e.g., "int64") or Structure class
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
            arg_types=[TypeSpec.INT32, TypeSpec.FLOAT32, TypeSpec.POINTER],
            return_type=TypeSpec.INT64,
            calling_convention="stdcall"
        )
        
        # Call the function
        result = native_func(42, 3.14, 0x87654321)
        print(f"Function returned: {result}")
        
        # Create a callable for a function with structure parameters
        class PlayerData(Structure):
            _fields_ = [
                ("health", ctypes.c_int32),
                ("mana", ctypes.c_int32),
            ]
            
        player_ptr = universe.create_pointer(0x12345690, PlayerData)
        
        get_player_name = universe.create_function(
            address=0x12345700,
            arg_types=[PlayerData],  # Pass structure by reference
            return_type=TypeSpec.CSTRING,
            calling_convention="cdecl"
        )
        
        # Call with structure pointer
        player_name = get_player_name(player_ptr)
    """
    if calling_convention is None:
        calling_convention = "cdecl" # Default calling convention
    return CallableFunction(address, arg_types, return_type, calling_convention)

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
            _fields_ = [
                ("health", ctypes.c_int32),
                ("mana", ctypes.c_int32),
            ]
            
        player_ptr = universe.create_pointer(0x12345690, PlayerData)
    """
    if isinstance(type_spec, TypeSpec):
        return Pointer(address, type_spec)
    elif isinstance(type_spec, str):
        # Deprecated string-based type specification
        for ts in [
            TypeSpec.INT8, TypeSpec.INT16, TypeSpec.INT32, TypeSpec.INT64,
            TypeSpec.UINT8, TypeSpec.UINT16, TypeSpec.UINT32, TypeSpec.UINT64,
            TypeSpec.FLOAT32, TypeSpec.FLOAT64, TypeSpec.POINTER, TypeSpec.CSTRING, TypeSpec.VOID,
            # Aliases
            TypeSpec.I8, TypeSpec.CHAR, TypeSpec.I16, TypeSpec.SHORT, TypeSpec.I32, TypeSpec.INT,
            TypeSpec.I64, TypeSpec.LONG, TypeSpec.U8, TypeSpec.UCHAR, TypeSpec.BYTE, TypeSpec.U16,
            TypeSpec.USHORT, TypeSpec.U32, TypeSpec.UINT, TypeSpec.U64, TypeSpec.ULONG,
            TypeSpec.F32, TypeSpec.FLOAT, TypeSpec.F64, TypeSpec.DOUBLE, TypeSpec.PTR,
            TypeSpec.USIZE, TypeSpec.STR, TypeSpec.STRING
        ]:
            if ts.name() == type_spec.lower():
                log(f"Warning: Using deprecated string-based type '{type_spec}'. Use TypeSpec.{ts.name().upper()} instead.")
                return Pointer(address, ts)
        raise ValueError(f"Invalid type string: {type_spec}")
    elif issubclass(type_spec, Structure):
        return StructurePointer(address, type_spec)
    else:
        raise ValueError(f"Invalid type specification: {type_spec}")

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
    _UniverseDLL.log(message.encode('utf-8'))

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
