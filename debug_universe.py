#!/usr/bin/env python3
"""
Debug script to check what's available in the universe module
"""

print("=" * 60)
print("DEBUG UNIVERSE MODULE CONTENTS")
print("=" * 60)


def debug_universe():
    """Debug what's available in the universe module"""
    try:
        import universe

        print("✓ Universe module imported successfully")

        # List all attributes in the universe module
        print("\nAll attributes in universe module:")
        attrs = dir(universe)
        for attr in sorted(attrs):
            if not attr.startswith("_"):
                obj = getattr(universe, attr)
                print(f"  {attr}: {type(obj)} - {obj}")

        # Specifically check for TypeSpec
        print(f"\nhasattr(universe, 'TypeSpec'): {hasattr(universe, 'TypeSpec')}")

        if hasattr(universe, "TypeSpec"):
            print("TypeSpec found!")
            print(f"TypeSpec type: {type(universe.TypeSpec)}")
            print(f"TypeSpec: {universe.TypeSpec}")
        else:
            print("TypeSpec NOT found!")

    except Exception as e:
        print(f"✗ Error: {e}")
        import traceback

        traceback.print_exc()


if __name__ == "__main__":
    debug_universe()
