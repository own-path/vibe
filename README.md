# Tempo

**Automatic project time tracking CLI tool**

A lightweight Rust-powered time tracking application that automatically detects your work context and provides detailed insights into your productivity patterns through a beautiful terminal interface.

[![Crates.io](https://img.shields.io/crates/v/tempo.svg)](https://crates.io/crates/tempo)
[![Documentation](https://docs.rs/tempo/badge.svg)](https://docs.rs/tempo)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://github.com/yourusername/vibe/workflows/CI/badge.svg)](https://github.com/yourusername/vibe/actions)

---

## Features

✅ **Currently Available:**
- **Daemon Architecture**: Lightweight background service for continuous tracking via IPC
- **Session Management**: Start, stop, pause, and resume tracking sessions
- **Beautiful CLI Output**: Color-coded, professional terminal interface with context-aware formatting
- **Automatic Context Detection**: Tracks time across terminal, IDE, and linked project contexts
- **Shell Integration**: Automatic project detection with directory changes (bash/zsh/fish/PowerShell)
- **Report Generation**: Export detailed reports in terminal, CSV, and JSON formats
- **Database Storage**: SQLite-based persistent storage with proper schema
- **Cross-platform**: Works on macOS, Linux, and Windows

🚧 **Coming Soon:**
- Project management (create, archive, organize)
- Tag system for session categorization
- Interactive TUI dashboard
- Session editing and audit trails
- Configuration management

## Quick Start

### Installation

**Prerequisites**: All installation methods require Rust and Cargo to be installed on your system. Install from [rustup.rs](https://rustup.rs/) first.

#### 🐍 pip (Python Wrapper)

```bash
# Install Rust first: https://rustup.rs/
pip install vibe-cli
vibe --version  # Should show: vibe 0.1.0
```

*Note: The Python package is a lightweight wrapper that calls the Rust binary.*

#### 🍺 Homebrew (Recommended)

```bash
brew tap own-path/tap
brew install vibe
vibe --version  # Should show: vibe 0.1.0
```

#### 📦 Cargo (Direct)

```bash
cargo install vibe
vibe --version  # Should show: vibe 0.1.0
```

#### 🔧 From Source

```bash
git clone https://github.com/own-path/vibe.git
cd vibe
cargo install --path .
vibe --version  # Should show: vibe 0.1.0
```

### Getting Started

Follow these step-by-step instructions:

#### 1. Start the Daemon
```bash
# Start the background tracking service
vibe start
# ✓ Daemon started successfully

# Verify it's running
vibe status
```

#### 2. Begin Time Tracking
```bash
# Start tracking in your current directory
vibe session start
# ✓ Started tracking session for [project-name]

# Check what's being tracked
vibe session current
# Shows: Active session details with duration and context
```

#### 3. View Real-time Status
```bash
vibe status
```
Output example:
```
┌─────────────────────────────────────────┐
│               Daemon Status             │
├─────────────────────────────────────────┤
│ Status:   ● Online                      │
│ Uptime:   2h 15m 30s                    │
│                                         │
│ Active Session:                         │
│   Project: my-awesome-project           │
│   Duration: 45m 12s                     │
│   Context: terminal                     │
└─────────────────────────────────────────┘
```

#### 4. Generate Reports
```bash
# View terminal report
vibe report

# Export to CSV
vibe report --format csv --from 2024-01-01

# Export to JSON  
vibe report --format json --group week
```

#### 5. Control Sessions
```bash
# Pause current session
vibe session pause

# Resume tracking
vibe session resume

# Stop session
vibe session stop
```

## Context-Aware Tracking

Vibe automatically detects your work environment and color-codes contexts:

- **Terminal** - Bright Cyan: Command-line development
- **IDE** - Bright Magenta: Integrated development environments  
- **Linked** - Bright Yellow: Multi-project workflows
- **Manual** - Bright Blue: Explicitly started sessions

## Project Structure

```
vibe/
├── src/
│   ├── cli/           # Complete CLI framework with commands and reports
│   ├── daemon/        # Background tracking service with IPC
│   ├── db/            # SQLite database with migrations and queries
│   ├── models/        # Data models (Project, Session, Tag, Config)
│   ├── ui/            # Terminal UI components and formatting
│   └── utils/         # IPC communication, config, and path utilities
├── migrations/        # SQLite database schema files
├── shell-hooks/       # Bash/Zsh/Fish/PowerShell integration scripts
├── python-package/    # Python wrapper package for pip distribution
│   └── python-pkg/
│       └── vibe_cli/  # Python entry point that calls Rust binary
└── Formula/           # Homebrew package formula
```

## Available Commands

### ✅ Daemon Management (Working)
```bash
vibe start              # Start tracking daemon
vibe stop               # Stop daemon
vibe restart            # Restart daemon
vibe status             # Show daemon and session status
```

### ✅ Session Control (Working)
```bash
vibe session start      # Start tracking current project
vibe session stop       # Stop current session
vibe session pause      # Pause tracking
vibe session resume     # Resume tracking
vibe session current    # Show active session details
```

### ✅ Reporting (Working)
```bash
vibe report                           # Terminal report with color formatting
vibe report --format csv              # Export to CSV
vibe report --format json             # Export to JSON
vibe report --from 2024-01-01         # Date range filtering
vibe report --group week              # Group by day/week/month/project
```

### ✅ Shell Integration (Working)
```bash
vibe completions bash               # Generate bash completions
vibe completions zsh                # Generate zsh completions
vibe completions fish               # Generate fish completions
vibe completions powershell         # Generate PowerShell completions
```

### 🚧 Coming Soon
```bash
vibe init [name]        # Initialize project tracking
vibe list               # List all projects
vibe project archive    # Archive a project
vibe project add-tag    # Tag projects for organization
vibe dashboard          # Real-time dashboard (TUI)
vibe tui               # Interactive project viewer
```

## Shell Integration

Vibe includes comprehensive shell hooks for automatic project detection and switching:

### Setup Instructions

#### Bash/Zsh
```bash
# Generate and install completions
vibe completions bash > ~/.config/vibe/completions.bash
echo 'source ~/.config/vibe/completions.bash' >> ~/.bashrc

# Add shell hooks (if available)
# source /usr/local/share/vibe/shell-hooks/vibe-hook.sh
```

#### Fish
```fish
# Generate completions
vibe completions fish > ~/.config/fish/completions/vibe.fish

# Add shell hooks (if available)  
# source /usr/local/share/vibe/shell-hooks/vibe-hook.fish
```

#### PowerShell
```powershell
# Generate completions
vibe completions powershell | Out-File -FilePath $PROFILE -Append
```

### Automatic Features
When shell integration is active:
- **Project Detection**: Automatically detects Git repos, package.json, Cargo.toml, etc.
- **Context Switching**: Changes tracking context when you `cd` between projects
- **Background Communication**: Seamlessly communicates with daemon via IPC

## Data Storage

Vibe stores all data locally using SQLite:

### Database Location
- **Database**: `~/.vibe/data.db` (SQLite with full schema)
- **Socket**: `~/.vibe/daemon.sock` (IPC communication)
- **PID File**: `~/.vibe/daemon.pid` (Daemon process tracking)

### Database Schema
The SQLite database includes tables for:
- **Projects**: Project metadata and paths
- **Sessions**: Time tracking sessions with context
- **Tags**: Project categorization (planned)
- **Config**: Application settings (planned)
- **Session Edits**: Audit trail for modifications (planned)

## Building from Source

```bash
git clone https://github.com/yourusername/vibe.git
cd vibe
cargo build --release

# Install locally
cargo install --path .

# Run tests
cargo test
```

### Development Setup

```bash
# Install development dependencies
cargo install cargo-watch

# Run with hot reload during development
cargo watch -x 'run -- status'

# Check code formatting
cargo fmt --check
cargo clippy
```

## Architecture

Vibe uses a daemon architecture for continuous, lightweight tracking:

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   CLI Client    │◄──►│  Daemon Process  │◄──►│ SQLite Database │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │                       │
         │              ┌─────────────────┐              │
         └─────────────►│ Shell Hooks     │◄─────────────┘
                        └─────────────────┘
```

- **CLI**: Beautiful terminal interface for all user interactions
- **Daemon**: Background service for automatic tracking and session management
- **Database**: SQLite for reliable, local data storage
- **Shell Hooks**: Automatic project detection via directory changes

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Quick Contribution Setup

```bash
# Fork and clone the repository
git clone https://github.com/yourusername/vibe.git
cd vibe

# Create a feature branch
git checkout -b feature/awesome-improvement

# Make changes and test
cargo test
cargo fmt
cargo clippy

# Commit and push
git commit -m "Add awesome improvement"
git push origin feature/awesome-improvement
```

## Examples

Check out the [`examples/`](examples/) directory for:

- Basic time tracking workflows
- Advanced reporting configurations
- Custom shell integrations
- Multi-project setups

## Roadmap

- [ ] Web dashboard for team analytics
- [ ] Git integration for commit-based tracking  
- [ ] Plugin system for IDE integrations
- [ ] Team collaboration features
- [ ] Mobile companion app
- [ ] AI-powered productivity insights

## Community

- **Discussions**: [GitHub Discussions](https://github.com/yourusername/vibe/discussions)
- **Issues**: [Bug Reports & Feature Requests](https://github.com/yourusername/vibe/issues)
- **Discord**: [Join our community](https://discord.gg/vibe-community)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Ratatui](https://github.com/ratatui/ratatui) for beautiful terminal interfaces
- Inspired by the simplicity of time tracking tools like Toggl and RescueTime
- Special thanks to the Rust community for excellent tooling and libraries

---

**Made with ❤️ by developers, for developers**