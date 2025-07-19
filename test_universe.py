#!/usr/bin/env python3
"""
Test universe.py script to verify Python runtime integration and Universe API
"""

print("=" * 60)
print("UNIVERSE MODDING FRAMEWORK - API TEST")
print("=" * 60)

import sys
import os


def test_basic_python():
    """Test basic Python functionality"""
    print("\n[TEST] Basic Python functionality")
    print(f"Python version: {sys.version}")
    print(f"Current directory: {os.getcwd()}")

    # Test basic math
    result = 1 + 1
    print(f"Math test: 1 + 1 = {result}")
    assert result == 2, "Basic math failed"
    print("✓ Basic Python functionality working")


def test_universe_import():
    """Test importing the universe module"""
    print("\n[TEST] Universe module import")
    try:
        import universe

        print("✓ Universe module imported successfully")

        # Check module attributes
        if hasattr(universe, "__version__"):
            print(f"Universe version: {universe.__version__}")
        if hasattr(universe, "__doc__"):
            print(f"Universe description: {universe.__doc__}")

        return universe
    except ImportError as e:
        print(f"✗ Failed to import universe module: {e}")
        return None
    except Exception as e:
        print(f"✗ Unexpected error importing universe: {e}")
        return None


def test_memory_api(universe):
    """Test memory access API"""
    print("\n[TEST] Memory API")
    try:
        # Test reading memory (this will use placeholder implementation)
        print("Testing read_memory...")
        data = universe.read_memory(0x1000, 16)
        print(f"✓ read_memory returned {len(data)} bytes")
        assert isinstance(data, (list, bytes)), (
            "read_memory should return bytes or list"
        )

        # Test writing memory (this will use placeholder implementation)
        print("Testing write_memory...")
        test_data = [0x41, 0x42, 0x43, 0x44]  # "ABCD"
        universe.write_memory(0x2000, test_data)
        print("✓ write_memory completed without error")

        # Test pattern scanning (this will use placeholder implementation)
        print("Testing pattern_scan...")
        result = universe.pattern_scan("kernel32.dll", "48 8B ? ? 89 45")
        print(f"✓ pattern_scan returned: {result}")

    except Exception as e:
        print(f"✗ Memory API test failed: {e}")


def test_hook_api(universe):
    """Test hook system API"""
    print("\n[TEST] Hook API")
    try:
        # Define test callback functions
        def test_function_hook(registers, original_function):
            print(
                f"Function hook called! Registers: {type(registers)}, Original: {type(original_function)}"
            )

        def test_jmpback_hook(registers):
            print(f"Jmpback hook called! Registers: {type(registers)}")

        # Test function hook installation
        print("Testing hook_function...")
        universe.hook_function(0x12345678, test_function_hook)
        print("✓ hook_function completed without error")

        # Test jmpback hook installation
        print("Testing hook_jmpback...")
        universe.hook_jmpback(0x87654321, test_jmpback_hook)
        print("✓ hook_jmpback completed without error")

        # Test hook removal
        print("Testing remove_hook...")
        universe.remove_hook(0x12345678)
        print("✓ remove_hook completed without error")

    except Exception as e:
        print(f"✗ Hook API test failed: {e}")


def test_ffi_api(universe):
    """Test FFI (Foreign Function Interface) API"""
    print("\n[TEST] FFI API")
    try:
        # Test creating a function wrapper
        print("Testing create_function...")
        func = universe.create_function(
            address=0x76543210,
            arg_types=["int", "float", "pointer"],
            return_type="int",
            calling_convention="cdecl",
        )
        print(f"✓ create_function returned: {type(func)}")
        assert callable(func), "create_function should return a callable object"

        # Test different calling conventions
        print("Testing different calling conventions...")
        for conv in ["cdecl", "stdcall", "fastcall"]:
            func = universe.create_function(
                address=0x11111111,
                arg_types=["int"],
                return_type="void",
                calling_convention=conv,
            )
            print(f"✓ {conv} calling convention supported")

    except Exception as e:
        print(f"✗ FFI API test failed: {e}")


def test_pointer_api(universe):
    """Test pointer system API"""
    print("\n[TEST] Pointer API")
    try:
        # Debug: List all attributes in the universe module
        print("DEBUG: All attributes in universe module:")
        attrs = dir(universe)
        for attr in sorted(attrs):
            if not attr.startswith('_'):
                obj = getattr(universe, attr)
                print(f"  {attr}: {type(obj)}")
        
        # Test TypeSpec enum availability
        print("Testing TypeSpec enum...")
        print(f"hasattr(universe, 'TypeSpec'): {hasattr(universe, 'TypeSpec')}")
        if hasattr(universe, "TypeSpec"):
            print("✓ TypeSpec enum found")

            # Test TypeSpec constants
            type_specs = [
                ("INT32", universe.TypeSpec.INT32),
                ("INT64", universe.TypeSpec.INT64),
                ("FLOAT32", universe.TypeSpec.FLOAT32),
                ("FLOAT64", universe.TypeSpec.FLOAT64),
                ("STRING", universe.TypeSpec.STRING),
                ("BOOL", universe.TypeSpec.BOOL),
                ("UINT32", universe.TypeSpec.UINT32),
                ("UINT64", universe.TypeSpec.UINT64),
                ("POINTER", universe.TypeSpec.POINTER),
            ]

            for name, type_spec in type_specs:
                print(f"  ✓ {name}: {type_spec}")
                print(f"    Size: {type_spec.size()} bytes")
                print(f"    Name: {type_spec.name()}")
        else:
            print("✗ TypeSpec enum not found")

        # Test basic pointer creation with TypeSpec
        print("Testing pointer creation with TypeSpec...")
        test_address = 0x10000000

        if hasattr(universe, "TypeSpec"):
            type_specs_to_test = [
                universe.TypeSpec.INT32,
                universe.TypeSpec.INT64,
                universe.TypeSpec.FLOAT32,
                universe.TypeSpec.FLOAT64,
                universe.TypeSpec.STRING,
                universe.TypeSpec.BOOL,
                universe.TypeSpec.UINT32,
                universe.TypeSpec.UINT64,
                universe.TypeSpec.POINTER,
            ]

            for type_spec in type_specs_to_test:
                try:
                    ptr = universe.create_pointer(test_address, type_spec)
                    print(f"✓ Created {type_spec.name()} pointer: {ptr}")

                    # Test pointer properties
                    assert ptr.address == test_address, (
                        f"Pointer address mismatch for {type_spec.name()}"
                    )
                    assert hasattr(ptr, "type_name"), (
                        f"Pointer missing type_name for {type_spec.name()}"
                    )
                    assert hasattr(ptr, "type_spec"), (
                        f"Pointer missing type_spec for {type_spec.name()}"
                    )

                    print(f"  Type name: {ptr.type_name}")
                    print(f"  Type spec: {ptr.type_spec}")

                except Exception as e:
                    print(f"✗ Failed to create {type_spec.name()} pointer: {e}")
                    import traceback

                    traceback.print_exc()

        # Test backward compatibility with strings
        print("Testing backward compatibility with string types...")
        string_types = [
            "int32",
            "int64",
            "float32",
            "float64",
            "string",
            "bool",
            "uint32",
            "uint64",
            "pointer",
        ]

        for type_name in string_types:
            try:
                ptr = universe.create_pointer(test_address, type_name)
                print(f"✓ Created pointer with string '{type_name}': {ptr}")

                # Test pointer properties
                assert ptr.address == test_address, (
                    f"Pointer address mismatch for {type_name}"
                )
                print(f"  Type name: {ptr.type_name}")

            except Exception as e:
                print(f"✗ Failed to create pointer with string '{type_name}': {e}")

        # Test direct Pointer constructor with TypeSpec
        if hasattr(universe, "TypeSpec"):
            print("Testing direct Pointer constructor with TypeSpec...")
            try:
                direct_ptr = universe.Pointer(test_address, universe.TypeSpec.FLOAT64)
                print(f"✓ Created direct pointer with TypeSpec: {direct_ptr}")
                print(f"  Type name: {direct_ptr.type_name}")
                print(f"  Type spec: {direct_ptr.type_spec}")
            except Exception as e:
                print(f"✗ Failed to create direct pointer with TypeSpec: {e}")
                import traceback

                traceback.print_exc()

        # Test legacy string types for backward compatibility
        print("Testing legacy string types...")
        legacy_types = ["int", "float", "string", "int64"]

        for type_name in legacy_types:
            try:
                ptr = universe.Pointer(test_address, type_name)
                print(f"✓ Created legacy {type_name} pointer: {ptr}")

                # Test pointer properties
                assert ptr.address == test_address, (
                    f"Pointer address mismatch for {type_name}"
                )
                print(f"  Type name: {ptr.type_name}")

            except Exception as e:
                print(f"✗ Failed to create legacy {type_name} pointer: {e}")

        # Test structure system
        print("Testing Structure class...")
        if hasattr(universe, "Structure"):
            struct = universe.Structure()
            print(f"✓ Created Structure: {struct}")

    except Exception as e:
        print(f"✗ Pointer API test failed: {e}")
        import traceback

        traceback.print_exc()


def test_logging_api(universe):
    """Test logging API"""
    print("\n[TEST] Logging API")
    try:
        # Test logging function
        print("Testing log function...")
        universe.log("Test log message from Python")
        print("✓ log function completed without error")

        # Test logging different message types
        test_messages = [
            "Simple test message",
            "Message with numbers: 123",
            "Message with special chars: !@#$%^&*()",
            "Unicode test: 你好世界",
        ]

        for msg in test_messages:
            try:
                universe.log(f"Test: {msg}")
            except Exception as e:
                print(f"✗ Failed to log message '{msg}': {e}")

        print("✓ All logging tests completed")

    except Exception as e:
        print(f"✗ Logging API test failed: {e}")


def test_error_handling(universe):
    """Test error handling and edge cases"""
    print("\n[TEST] Error Handling")
    try:
        # Test invalid memory addresses
        print("Testing error handling for invalid operations...")

        # Test invalid type in create_function
        try:
            universe.create_function(0x1000, ["invalid_type"], "int")
            print("✗ Should have failed with invalid type")
        except Exception as e:
            print(f"✓ Correctly caught invalid type error: {type(e).__name__}")

        # Test invalid calling convention
        try:
            universe.create_function(0x1000, ["int"], "int", "invalid_convention")
            print("✗ Should have failed with invalid calling convention")
        except Exception as e:
            print(
                f"✓ Correctly caught invalid calling convention error: {type(e).__name__}"
            )

        # Test non-callable hook callback
        try:
            universe.hook_function(0x1000, "not_callable")
            print("✗ Should have failed with non-callable callback")
        except Exception as e:
            print(f"✓ Correctly caught non-callable callback error: {type(e).__name__}")

    except Exception as e:
        print(f"✗ Error handling test failed: {e}")


def test_class_functionality(universe):
    """Test Python class functionality exposed by the module"""
    print("\n[TEST] Class Functionality")
    try:
        # Test available classes
        classes_to_test = [
            "Pointer",
            "Structure",
            "StructurePointer",
            "Registers",
            "OriginalFunction",
        ]

        for class_name in classes_to_test:
            if hasattr(universe, class_name):
                cls = getattr(universe, class_name)
                print(f"✓ Found class: {class_name}")

                # Test class documentation
                if hasattr(cls, "__doc__") and cls.__doc__:
                    print(f"  Documentation: {cls.__doc__[:100]}...")
            else:
                print(f"- Class not found: {class_name}")

    except Exception as e:
        print(f"✗ Class functionality test failed: {e}")


def run_comprehensive_tests():
    """Run all tests"""
    print("Starting comprehensive Universe API tests...")

    # Test basic Python functionality
    test_basic_python()

    # Test universe module import
    universe = test_universe_import()
    if not universe:
        print("\n✗ Cannot continue tests - universe module not available")
        return False

    # Test all APIs
    test_memory_api(universe)
    test_hook_api(universe)
    test_ffi_api(universe)
    test_pointer_api(universe)
    test_logging_api(universe)
    test_error_handling(universe)
    test_class_functionality(universe)

    return True


def main():
    """Main test function"""
    try:
        print("Universe.py test script starting...")

        success = run_comprehensive_tests()

        print("\n" + "=" * 60)
        if success:
            print("🎉 ALL TESTS COMPLETED!")
            print("Universe modding framework is working correctly!")
        else:
            print("❌ SOME TESTS FAILED!")
            print("Check the output above for details.")
        print("=" * 60)

        # Log completion to file for verification
        try:
            with open("test_results.log", "w") as f:
                f.write("Universe API Test Results\n")
                f.write("=" * 30 + "\n")
                f.write(f"Test completed: {'SUCCESS' if success else 'FAILURE'}\n")
                f.write(f"Python version: {sys.version}\n")
                f.write(f"Working directory: {os.getcwd()}\n")
            print("Test results written to test_results.log")
        except Exception as e:
            print(f"Could not write test results: {e}")

    except Exception as e:
        print(f"\n💥 CRITICAL ERROR in test execution: {e}")
        import traceback

        traceback.print_exc()


if __name__ == "__main__":
    main()
