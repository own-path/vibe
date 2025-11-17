#!/usr/bin/env python3

import os
import sys
import platform
import subprocess
from setuptools import setup, find_packages
from setuptools.command.install import install

class PostInstallCommand(install):
    """Post-installation for installing the Vibe binary"""
    
    def run(self):
        install.run(self)
        self.install_vibe_binary()
    
    def install_vibe_binary(self):
        """Download and install the appropriate Vibe binary"""
        system = platform.system().lower()
        machine = platform.machine().lower()
        
        # Map platform to binary names
        if system == "darwin":
            if "arm" in machine or "aarch64" in machine:
                target = "aarch64-apple-darwin"
            else:
                target = "x86_64-apple-darwin"
        elif system == "linux":
            if "arm" in machine or "aarch64" in machine:
                target = "aarch64-unknown-linux-gnu"
            else:
                target = "x86_64-unknown-linux-gnu"
        elif system == "windows":
            target = "x86_64-pc-windows-msvc"
        else:
            print(f"Unsupported platform: {system} {machine}")
            print("Please install Vibe manually with: cargo install vibe")
            return
        
        # Install via cargo as it's the most reliable method
        try:
            print("Installing Vibe via cargo...")
            subprocess.check_call([sys.executable, "-c", "import subprocess; subprocess.check_call(['cargo', 'install', 'vibe'])"])
            print("✅ Vibe installed successfully!")
            print("\nQuick start:")
            print("  vibe start    # Start the daemon")
            print("  vibe status   # Check status")
            print("  vibe session start  # Begin tracking")
        except subprocess.CalledProcessError:
            print("❌ Failed to install via cargo.")
            print("Please ensure Rust is installed: https://rustup.rs/")
            print("Then run: cargo install vibe")
        except FileNotFoundError:
            print("❌ Cargo not found. Please install Rust first: https://rustup.rs/")
            print("Then run: cargo install vibe")

with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

setup(
    name="vibe-cli",
    version="0.1.0",
    author="Own Path",
    author_email="brandy.daryl@gmail.com",
    description="Automatic project time tracking CLI tool with beautiful terminal interface",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/own-path/vibe",
    project_urls={
        "Bug Tracker": "https://github.com/own-path/vibe/issues",
        "Documentation": "https://github.com/own-path/vibe/blob/main/README.md",
        "Source Code": "https://github.com/own-path/vibe",
    },
    classifiers=[
        "Development Status :: 4 - Beta",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.7",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
        "Topic :: Software Development :: Tools",
        "Topic :: Utilities",
        "Environment :: Console",
    ],
    packages=find_packages(),
    python_requires=">=3.7",
    cmdclass={
        'install': PostInstallCommand,
    },
    entry_points={
        'console_scripts': [
            'vibe=vibe_cli.main:main',
        ],
    },
    keywords=["time-tracking", "productivity", "cli", "terminal", "rust"],
    install_requires=[
        # No Python dependencies needed - we're just a wrapper
    ],
)