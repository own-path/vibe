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
    
    # Look for the actual vibe binary (not the Python wrapper)
    vibe_path = None
    
    # First try to find vibe-bin or vibe-rs (alternative names)
    for binary_name in ['vibe-bin', 'vibe-rs']:
        vibe_path = shutil.which(binary_name)
        if vibe_path:
            break
    
    # If not found, look for 'vibe' but exclude the Python wrapper
    if not vibe_path:
        # Get all vibe executables in PATH
        for path_dir in sys.path if hasattr(sys, 'path') else []:
            continue
        
        # Check common installation locations
        import os
        possible_paths = [
            os.path.expanduser('~/.cargo/bin/vibe'),
            '/usr/local/bin/vibe-bin',
            '/opt/homebrew/bin/vibe-bin'
        ]
        
        for path in possible_paths:
            if os.path.isfile(path) and os.access(path, os.X_OK):
                # Check if this is not a Python script
                try:
                    with open(path, 'rb') as f:
                        header = f.read(50)
                        if b'python' not in header.lower():
                            vibe_path = path
                            break
                except:
                    continue
    
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