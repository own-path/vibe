#!/usr/bin/env python3

import os
import sys
import platform
import subprocess
from setuptools import setup, find_packages
from setuptools.command.install import install

class PostInstallCommand(install):
    """Post-installation for installing the Tempo binary"""
    
    def run(self):
        install.run(self)
        self.install_tempo_binary()
    
    def install_tempo_binary(self):
        """Download and install the appropriate Tempo binary"""
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
            print("Please install Tempo manually with: cargo install tempo-cli")
            return
        
        # Install via cargo as it's the most reliable method
        try:
            print("Installing Tempo via cargo...")
            subprocess.check_call([sys.executable, "-c", "import subprocess; subprocess.check_call(['cargo', 'install', 'tempo-cli'])"])
            print("Tempo installed successfully!")
            print("\nQuick start:")
            print("  tempo start               # Start the daemon")
            print("  tempo status              # Check status")
            print("  tempo session start      # Begin tracking")
            print("  tempo list                # View projects")
            print("  tempo dashboard           # Interactive dashboard")
        except subprocess.CalledProcessError:
            print("Failed to install via cargo.")
            print("Please ensure Rust is installed: https://rustup.rs/")
            print("Then run: cargo install tempo-cli")
        except FileNotFoundError:
            print("Cargo not found. Please install Rust first: https://rustup.rs/")
            print("Then run: cargo install tempo-cli")

with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

setup(
    name="tempo-tracker-cli",
    version="0.3.0",
    author="Own Path",
    author_email="brandy.daryl@gmail.com",
    description="The Most Advanced Automatic Project Time Tracker - Lightning-fast Rust-powered CLI",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/own-path/vibe",
    project_urls={
        "Bug Tracker": "https://github.com/own-path/vibe/issues",
        "Documentation": "https://docs.rs/tempo",
        "Source Code": "https://github.com/own-path/vibe",
        "PyPI": "https://pypi.org/project/tempo-tracker-cli/",
    },
    classifiers=[
        "Development Status :: 5 - Production/Stable",
        "Intended Audience :: Developers",
        "Intended Audience :: Information Technology",
        "License :: OSI Approved :: MIT License",
        "Operating System :: MacOS",
        "Operating System :: POSIX :: Linux", 
        "Operating System :: Microsoft :: Windows",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
        "Programming Language :: Rust",
        "Topic :: Office/Business :: Scheduling",
        "Topic :: Software Development",
        "Topic :: System :: Monitoring",
        "Topic :: Utilities",
        "Environment :: Console",
    ],
    packages=find_packages(),
    python_requires=">=3.8",
    cmdclass={
        'install': PostInstallCommand,
    },
    entry_points={
        'console_scripts': [
            'tempo=tempo_cli.main:main',
        ],
    },
    keywords=[
        "time-tracking", "productivity", "cli", "terminal", "rust", 
        "project-management", "goals", "analytics", "developer-tools",
        "workspace", "session-tracking", "automatic", "daemon"
    ],
    install_requires=[
        # No Python dependencies needed - we're just a wrapper
    ],
)