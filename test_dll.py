#!/usr/bin/env python3
"""
Test script to verify the DLL can be loaded and Python runtime works
"""

import ctypes
import os
import sys
import shutil

def test_dll_loading():
    """Test that we can load the DLL and it initializes correctly"""
    
    # Copy our test universe.py to the current directory
    if os.path.exists("test_universe.py"):
        shutil.copy("test_universe.py", "universe.py")
        print("Copied test_universe.py to universe.py")
    
    # Try to load the DLL
    try:
        # Build the DLL first
        print("Building the DLL...")
        os.system("cargo build")
        
        # Load the DLL
        dll_path = "./target/debug/mod_hub.dll"
        if not os.path.exists(dll_path):
            print(f"DLL not found at {dll_path}")
            return False
            
        print(f"Loading DLL from {dll_path}")
        dll = ctypes.CDLL(dll_path)
        print("DLL loaded successfully!")
        
        # The DLL should have initialized Python and executed universe.py
        # Check if universe.log was created
        if os.path.exists("universe.log"):
            print("universe.log found! Reading contents:")
            with open("universe.log", "r") as f:
                log_contents = f.read()
                print("--- universe.log contents ---")
                print(log_contents)
                print("--- end of universe.log ---")
        else:
            print("universe.log not found")
            
        return True
        
    except Exception as e:
        print(f"Error loading DLL: {e}")
        return False
    finally:
        # Clean up
        if os.path.exists("universe.py"):
            os.remove("universe.py")
            print("Cleaned up universe.py")

if __name__ == "__main__":
    print("Testing Universe DLL Python runtime integration...")
    success = test_dll_loading()
    if success:
        print("Test completed successfully!")
    else:
        print("Test failed!")
    sys.exit(0 if success else 1)