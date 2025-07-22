"""
This library provides a Python interface for interacting with the Universe modding framework.
It includes low-level wrappers for the Rust API, as well as higher-level abstractions for
memory manipulation and function hooking.
"""

import ctypes
from typing import Any, Callable, Dict, List, Optional, Type, TypeVar

# The `universe` module is a native Python extension written in Rust.
# It provides the core functionality for interacting with the modding framework.
# This is a placeholder for the actual compiled Rust module.
try:
    import universe
except ImportError:
    print("Warning: 'universe' module not found. Using mock objects for type hinting.")

    # Mock objects for type hinting and development without the compiled module
    class MockUniverse:
        def __getattr__(self, name):
            def _mock_func(*args, **kwargs):
                print(f"Mock call to universe.{name}({args}, {kwargs})")
                if "read" in name:
                    return []
                if "list" in name:
                    return {}
                return None

            return _mock_func

    universe = MockUniverse()


# ==================================================================================================
# Type-level, typesafe wrappers for the Rust API
# ==================================================================================================


class ModuleInfo:
    """
    Represents information about a loaded module in the target process.
    This is a Python-friendly wrapper around the Rust `ModuleInfo` struct.
    """

    def __init__(self, base_address: int, size: int):
        self.base_address = base_address
        self.size = size

    def __repr__(self) -> str:
        return f"ModuleInfo(base_address=0x{self.base_address:x}, size={self.size})"


class Registers:
    """
    Represents the state of the CPU registers. This class provides a Python-friendly
    interface to the `PyRegisterAccess` struct in the Rust code.
    """

    def __init__(self, rust_registers: Any):
        # This is not a public constructor.
        # Instances are created by the hooking mechanism.
        self._rust_registers = rust_registers

    def __getattr__(self, name: str) -> Any:
        # Forward attribute access to the underlying Rust object
        return getattr(self._rust_registers, name)

    def __setattr__(self, name: str, value: Any) -> None:
        # Allow setting internal attributes, otherwise forward to Rust object
        if name == "_rust_registers":
            super().__setattr__(name, value)
        else:
            setattr(self._rust_registers, name, value)

    def __repr__(self) -> str:
        return repr(self._rust_registers)


def read_memory(address: int, size: int, validate: bool = True) -> bytes:
    """
    Reads a block of memory from the target process.

    Args:
        address: The memory address to read from.
        size: The number of bytes to read.
        validate: Whether to validate the memory address before reading.

    Returns:
        The bytes read from memory.
    """
    return bytes(universe.read_memory(address, size, validate))


def write_memory(address: int, data: bytes) -> None:
    """
    Writes a block of memory to the target process.

    Args:
        address: The memory address to write to.
        data: The bytes to write.
    """
    # The Rust function expects a Vec<u8>, which pyo3 can convert from a list of ints.
    universe.write_memory(address, list(data))


def list_modules() -> Dict[str, ModuleInfo]:
    """
    Lists all loaded modules in the target process.

    Returns:
        A dictionary mapping module names to `ModuleInfo` objects.
    """
    # The Rust function returns a dict with a raw PyModuleInfo object.
    # We wrap it in our Python-friendly `ModuleInfo` class.
    raw_modules = universe.list_modules()
    return {
        name: ModuleInfo(info.base_address, info.size)
        for name, info in raw_modules.items()
    }


def pattern_scan(module_name: str, pattern: str) -> Optional[int]:
    """
    Scans for a byte pattern within a specific module.

    Args:
        module_name: The name of the module to scan.
        pattern: The byte pattern to search for (e.g., "48 8B ? ? 89 45").

    Returns:
        The memory address of the first match, or `None` if the pattern is not found.
    """
    return universe.pattern_scan(module_name, pattern)


def log(message: str) -> None:
    """
    Logs a message to the Universe log file.

    Args:
        message: The message to log.
    """
    universe.log(message)


# ==================================================================================================
# High-level memory abstraction
# ==================================================================================================

T = TypeVar("T", bound="Structure")


class Structure(ctypes.Structure):
    """
    A base class for creating custom data structures backed by memory pointers.
    This class uses `ctypes` to define the structure layout and provides a simple
    way to read and write data from memory.

    Example:
        class Player(Structure):
            _fields_ = [("health", ctypes.c_int),
                        ("mana", ctypes.c_int)]

        player_ptr = 0x12345678
        player = Player.from_address(player_ptr)
        print(player.health)
        player.health = 100
        player.save()
    """

    def __init__(self, address: int, buffer: Optional[bytes] = None):
        self._address = address
        if buffer is None:
            buffer = read_memory(address, ctypes.sizeof(self))
        # ctypes.Structure can be initialized directly from a buffer
        super().__init__.from_buffer_copy(buffer)

    @classmethod
    def from_address(cls: Type[T], address: int) -> T:
        """
        Creates a new instance of the structure from a memory address.
        """
        size = ctypes.sizeof(cls)
        buffer = read_memory(address, size)
        return cls.from_buffer_copy(buffer)

    def save(self) -> None:
        """
        Writes the current state of the structure back to its memory address.
        """
        buffer = ctypes.string_at(ctypes.addressof(self), ctypes.sizeof(self))
        write_memory(self._address, buffer)

    def __repr__(self) -> str:
        return f"{self.__class__.__name__}(address=0x{self._address:x})"


# ==================================================================================================
# High-level hooking abstraction
# ==================================================================================================


def _create_callable_for_original(address: int, func_sig: Any) -> Callable:
    """Internal helper to create a callable ctypes function."""
    return func_sig(address)


class Hook:
    """
    A high-level abstraction for creating and managing function hooks.

    This class simplifies the process of hooking a function, providing a
    performant way to call the original function from the Python callback.

    Example:
        # Define the signature of the function to hook (e.g., a Windows API)
        MyFunctionType = ctypes.WINFUNCTYPE(ctypes.c_int, ctypes.c_int, ctypes.c_int)

        def my_callback(registers: Registers, original_function: Callable, a: int, b: int) -> int:
            print(f"Hooked! Args: {a}, {b}")
            # Call the original function with modified arguments
            result = original_function(a * 2, b * 2)
            print(f"Original function returned: {result}")
            return result

        # Address found via pattern scanning or other means
        target_address = 0x12345678
        hook = Hook(target_address, MyFunctionType, my_callback)

        # ... later
        hook.remove()
    """

    def __init__(self, address: int, func_sig: Any, callback: Callable):
        if not address or address == 0:
            raise ValueError("Hook address cannot be null or zero.")
        if not callable(callback):
            raise TypeError("Callback must be a callable function.")

        self.address = address
        self.func_sig = func_sig
        self.callback = callback
        self._is_removed = False

        # The core of the hooking mechanism. The Rust backend calls _hook_handler.
        universe.hook_function(self.address, self._hook_handler)

    def _hook_handler(self, rust_registers: Any, original_function_ptr: int) -> int:
        """
        This is the internal handler that the Rust backend calls.
        It wraps the raw pointers and calls the user's Python callback.
        """
        if self._is_removed:
            # This should ideally not be called if removed, but as a safeguard:
            return 0  # Default return value

        # 1. Create a performant, callable wrapper for the original function
        original_function = _create_callable_for_original(
            original_function_ptr, self.func_sig
        )

        # 2. Wrap the raw rust_registers in our Python-friendly class
        py_registers = Registers(rust_registers)

        # 3. Call the user's Python callback with the wrapped objects
        # The user's callback is responsible for calling original_function with the correct arguments.
        try:
            return self.callback(py_registers, original_function)
        except Exception as e:
            log(f"Error in Python hook callback for address 0x{self.address:x}: {e}")
            # Return a default value to prevent crashing the target application
            return 0

    def remove(self) -> None:
        """
        Removes the hook and cleans up resources.
        """
        if not self._is_removed:
            universe.remove_hook(self.address)
            self._is_removed = True
            log(f"Hook removed for address 0x{self.address:x}")

    def __del__(self):
        # Ensure the hook is removed when the Hook object is garbage collected
        self.remove()

    def __repr__(self) -> str:
        status = "Removed" if self._is_removed else "Active"
        return f"Hook(address=0x{self.address:x}, status={status})"


# Jmpback hooks are simpler as they don't typically call the original function.
class JmpbackHook:
    """
    A high-level abstraction for creating jmp-back hooks.
    These hooks are for code caves and don't return to the original function.
    """

    def __init__(self, address: int, callback: Callable):
        if not address or address == 0:
            raise ValueError("Hook address cannot be null or zero.")
        if not callable(callback):
            raise TypeError("Callback must be a callable function.")

        self.address = address
        self.callback = callback
        self._is_removed = False

        universe.hook_jmpback(self.address, self._jmpback_handler)

    def _jmpback_handler(self, rust_registers: Any) -> None:
        """Internal handler that calls the user's Python callback."""
        if self._is_removed:
            return

        py_registers = Registers(rust_registers)
        try:
            self.callback(py_registers)
        except Exception as e:
            log(
                f"Error in Python jmpback hook callback for address 0x{self.address:x}: {e}"
            )

    def remove(self) -> None:
        """Removes the hook."""
        if not self._is_removed:
            universe.remove_hook(self.address)
            self._is_removed = True
            log(f"Jmpback hook removed for address 0x{self.address:x}")

    def __del__(self):
        self.remove()

    def __repr__(self) -> str:
        status = "Removed" if self._is_removed else "Active"
        return f"JmpbackHook(address=0x{self.address:x}, status={status})"
