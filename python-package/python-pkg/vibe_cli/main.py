#!/usr/bin/env python3
"""
Python wrapper for the Vibe CLI tool.

This script acts as a Python entry point that calls the actual Vibe binary.
"""

import sys
import subprocess
import shutil

def main():
    """Main entry point that forwards all arguments to the vibe binary"""
    
    # Check if vibe is installed
    vibe_path = shutil.which('vibe')
    if not vibe_path:
        print("❌ Vibe binary not found in PATH.")
        print("\nThis usually means the installation didn't complete successfully.")
        print("Please try one of these alternatives:")
        print("  1. Install via cargo: cargo install vibe")
        print("  2. Install via homebrew: brew install own-path/tap/vibe")
        print("  3. Reinstall this package: pip install --force-reinstall vibe-cli")
        sys.exit(1)
    
    # Forward all arguments to the actual vibe binary
    try:
        # Use subprocess to call the vibe binary with all arguments
        result = subprocess.run([vibe_path] + sys.argv[1:], 
                              check=False,
                              text=True)
        sys.exit(result.returncode)
    except KeyboardInterrupt:
        sys.exit(130)  # Standard exit code for SIGINT
    except Exception as e:
        print(f"❌ Error running vibe: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()