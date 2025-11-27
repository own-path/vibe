#!/usr/bin/env python3
"""
Python wrapper for the Tempo CLI tool.

This script acts as a Python entry point that calls the actual Tempo binary.
"""

import sys
import subprocess
import shutil

def main():
    """Main entry point that forwards all arguments to the tempo binary"""
    
    # Look for the actual tempo binary (not the Python wrapper)
    tempo_path = None
    
    # First try to find tempo-bin or tempo-rs (alternative names)
    for binary_name in ['tempo-bin', 'tempo-rs']:
        tempo_path = shutil.which(binary_name)
        if tempo_path:
            break
    
    # If not found, look for 'tempo' but exclude the Python wrapper
    if not tempo_path:
        # Get all tempo executables in PATH
        for path_dir in sys.path if hasattr(sys, 'path') else []:
            continue
        
        # Check common installation locations
        import os
        possible_paths = [
            os.path.expanduser('~/.cargo/bin/tempo'),
            '/usr/local/bin/tempo-bin',
            '/opt/homebrew/bin/tempo-bin'
        ]
        
        for path in possible_paths:
            if os.path.isfile(path) and os.access(path, os.X_OK):
                # Check if this is not a Python script
                try:
                    with open(path, 'rb') as f:
                        header = f.read(50)
                        if b'python' not in header.lower():
                            tempo_path = path
                            break
                except:
                    continue
    
    if not tempo_path:
        print("❌ Tempo binary not found in PATH.")
        print("\nThis usually means the installation didn't complete successfully.")
        print("Please try one of these alternatives:")
        print("  1. Install via cargo: cargo install tempo")
        print("  2. Install via homebrew: brew install tempo")
        print("  3. Reinstall this package: pip install --force-reinstall tempo-cli")
        sys.exit(1)
    
    # Forward all arguments to the actual tempo binary
    try:
        # Use subprocess to call the tempo binary with all arguments
        result = subprocess.run([tempo_path] + sys.argv[1:], 
                              check=False,
                              text=True)
        sys.exit(result.returncode)
    except KeyboardInterrupt:
        sys.exit(130)  # Standard exit code for SIGINT
    except Exception as e:
        print(f"❌ Error running tempo: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()