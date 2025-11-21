#!/bin/bash
set -e

echo "🚀 Installing Tempo CLI..."

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo is not installed. Please install Rust and Cargo first."
    echo "   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Build and install
echo "📦 Building and installing..."
cargo install --path . --bins --force

echo "✅ Tempo installed successfully!"
echo "   Run 'tempo help' to get started."
