# Tempo ⏱️

**Simple, Fast Project Time Tracking for Developers**

A lightweight Rust-powered time tracking CLI that automatically detects your project context and tracks time across multiple projects. Built for developers who want accurate time tracking without complexity.

[![PyPI](https://img.shields.io/pypi/v/tempo-cli)](https://pypi.org/project/tempo-cli/)
[![Crates.io](https://img.shields.io/crates/v/tempo-cli.svg)](https://crates.io/crates/tempo-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Downloads](https://img.shields.io/pypi/dm/tempo-cli)](https://pypi.org/project/tempo-cli/)

---

## Why Tempo?

**Simple & Effective**: Start tracking time in seconds - no complex setup or configuration required.

**Developer-Focused**: Automatically detects Git repositories and project structures. Understands your workflow.

**Fast & Lightweight**: Rust-powered daemon uses minimal system resources. Commands respond instantly.

**Privacy-First**: All data stored locally in SQLite. No cloud services, no data collection.

**Cross-Platform**: Works on macOS, Linux, and Windows from any terminal.

---

## Installation

### Python/UV (Recommended)
```bash
# Install with uv (fastest)
uv install tempo-cli

# Or with pip
pip install tempo-cli

# Start using immediately
tempo start
tempo session start
```

### Rust
```bash
# Install from crates.io
cargo install tempo-cli

# Or build from source
git clone https://github.com/own-path/vibe.git
cd vibe && cargo install --path .
```

---

## Core Features

### ⚡ Time Tracking
- **Automatic Project Detection** - Recognizes Git repositories, package.json, Cargo.toml files
- **Session Management** - Start, stop, pause, and resume tracking sessions
- **Background Daemon** - Lightweight service runs automatically in background
- **Multi-Project Support** - Track multiple projects without switching configurations

### 📊 Project Management  
- **Project Organization** - Initialize and manage project tracking
- **Session History** - Browse and edit past tracking sessions
- **Time Reports** - Generate reports with CSV/JSON export
- **Project Archiving** - Archive completed projects while preserving data

### 🎨 User Interface
- **Interactive Dashboard** - Real-time tracking status and project overview
- **Terminal UI** - Browse projects and sessions with keyboard navigation
- **Timer Interface** - Visual timer with progress tracking
- **Configurable Settings** - Customize behavior through configuration files

---

## Quick Start

```bash
# Start the daemon
tempo start

# Initialize a project (in your project directory)
tempo init "My Project"

# Start tracking
tempo session start

# Check status
tempo status

# View dashboard
tempo dashboard

# Stop tracking
tempo session stop

# Generate report
tempo report --format csv
```

---

## Available Commands

### Session Management
```bash
tempo session start        # Begin tracking current project
tempo session stop         # Stop current session
tempo session pause        # Pause tracking
tempo session resume       # Resume tracking
tempo session current      # Show active session
tempo session list         # List recent sessions
tempo session edit <id>    # Edit session details
tempo session delete <id>  # Delete a session
```

### Project Operations
```bash
tempo init "Project Name"   # Initialize project tracking
tempo list                  # List all projects
tempo list --archived       # Include archived projects
tempo project archive <id> # Archive a project
tempo project unarchive <id> # Restore archived project
tempo project update-path <id> <path> # Update project path
```

### Reporting & Analytics
```bash
tempo report               # Terminal-formatted time report
tempo report --format csv  # Export to CSV
tempo report --format json # Export to JSON
tempo report --from 2024-01-01 # Date range filter
tempo report --project <id> # Project-specific report
```

### Interactive Interfaces
```bash
tempo dashboard           # Real-time tracking dashboard
tempo timer              # Visual timer interface
tempo history            # Browse session history
```

### Configuration
```bash
tempo config show        # View current settings
tempo config set <key> <value> # Update setting
tempo config reset       # Reset to defaults
```

### Daemon Control
```bash
tempo start              # Start background daemon
tempo stop               # Stop daemon
tempo restart            # Restart daemon
tempo status             # Show daemon and session status
```

---

## Configuration

Tempo stores configuration in `~/.tempo/config.toml`:

```toml
idle_timeout_minutes = 15
auto_pause_enabled = true
default_context = "terminal"
log_level = "info"
```

Available settings:
- `idle_timeout_minutes` - Auto-pause after inactivity (default: 15)
- `auto_pause_enabled` - Enable automatic pausing (default: true)  
- `default_context` - Default tracking context (default: "terminal")
- `log_level` - Logging verbosity: error, warn, info, debug (default: "info")

Update settings with: `tempo config set <key> <value>`

---

## Data Storage

All data is stored locally in `~/.tempo/`:

```
~/.tempo/
├── data.db              # SQLite database (all tracking data)
├── config.toml          # Configuration settings
├── daemon.sock          # IPC socket for daemon communication
├── daemon.pid           # Daemon process ID
└── logs/
    └── tempo.log        # Application logs
```

**Privacy**: No data ever leaves your machine. No telemetry or tracking.

---

## Project Detection

Tempo automatically detects projects by scanning for:

- **Git repositories** (`.git/` directory)
- **Node.js projects** (`package.json`)
- **Rust projects** (`Cargo.toml`) 
- **Python projects** (`pyproject.toml`, `setup.py`, `requirements.txt`)
- **Go projects** (`go.mod`)
- **Java projects** (`pom.xml`, `build.gradle`)
- **And many more...**

When you run `tempo session start` in a recognized project directory, tracking begins automatically.

---

## Performance

- **Memory Usage**: < 1MB for daemon process
- **CPU Overhead**: Negligible on modern systems
- **Startup Time**: < 100ms for all commands
- **Database Size**: ~1MB per year of tracking data
- **Battery Impact**: Minimal on laptops

---

## Contributing

Contributions welcome! This is an active open-source project.

### Development Setup
```bash
git clone https://github.com/own-path/vibe.git
cd vibe
cargo build
cargo test
cargo run -- status
```

### Pull Requests
1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Submit a pull request with clear description

---

## License

MIT License - see [LICENSE](LICENSE) file for details.

Free to use in personal and commercial projects.

---

## Support

- **Issues**: [GitHub Issues](https://github.com/own-path/vibe/issues)
- **Discussions**: [GitHub Discussions](https://github.com/own-path/vibe/discussions)
- **Documentation**: Available in the repository wiki

---

**Built for developers who value simplicity and accuracy in time tracking.**

⭐ Star the project if it helps you track time effectively!

🚀 Get started: `uv install tempo-cli`