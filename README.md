# Vibe

**Automatic project time tracking CLI tool**

A beautiful, intelligent time tracking application that automatically detects your work context and provides detailed insights into your productivity patterns.

[![Crates.io](https://img.shields.io/crates/v/vibe.svg)](https://crates.io/crates/vibe)
[![Documentation](https://docs.rs/vibe/badge.svg)](https://docs.rs/vibe)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://github.com/yourusername/vibe/workflows/CI/badge.svg)](https://github.com/yourusername/vibe/actions)

---

## Features

- **Automatic Detection**: Seamlessly tracks time across terminal, IDE, and linked project contexts
- **Beautiful CLI Output**: Color-coded, professional terminal interface with context-aware formatting
- **Daemon Architecture**: Lightweight background service for continuous tracking
- **Shell Integration**: Automatic project switching with directory changes
- **Flexible Reporting**: Generate detailed reports in multiple formats (terminal, CSV, JSON)
- **Session Management**: Pause, resume, and edit tracking sessions with full audit trails
- **Cross-platform**: Works on macOS, Linux, and Windows

## Quick Start

### Installation

#### 🍺 Homebrew (Recommended)

```bash
brew install own-path/tap/vibe
```

#### 📦 Cargo

```bash
cargo install vibe
```

#### 🔧 From Source

```bash
git clone https://github.com/own-path/vibe.git
cd vibe
cargo install --path .
```

### Basic Usage

```bash
# Start the daemon
vibe start

# Begin tracking in current directory
vibe session start

# Check current status
vibe status
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

# View current session details
vibe session current

# Generate reports
vibe report
vibe report --format csv --from 2024-01-01
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
│   ├── cli/           # Command-line interface
│   ├── daemon/        # Background tracking service
│   ├── db/            # SQLite database layer
│   ├── models/        # Data structures
│   ├── ui/            # Terminal UI components
│   └── utils/         # Utilities and IPC
├── migrations/        # Database schema migrations
├── shell-hooks/       # Shell integration scripts
└── examples/          # Usage examples
```

## Commands

### Daemon Management
```bash
vibe start              # Start tracking daemon
vibe stop               # Stop daemon
vibe restart            # Restart daemon
vibe status             # Show daemon and session status
```

### Session Control
```bash
vibe session start      # Start tracking current project
vibe session stop       # Stop current session
vibe session pause      # Pause tracking
vibe session resume     # Resume tracking
vibe session current    # Show active session
```

### Project Management
```bash
vibe init [name]        # Initialize project tracking
vibe list               # List all projects
vibe project archive    # Archive a project
vibe project add-tag    # Tag projects for organization
```

### Reporting
```bash
vibe report                           # Terminal report
vibe report --format csv              # Export to CSV
vibe report --project myapp           # Project-specific report
vibe report --from 2024-01-01         # Date range filtering
```

### Interactive UIs
```bash
vibe dashboard          # Real-time dashboard (TUI)
vibe tui               # Interactive project viewer
```

## Shell Integration

Vibe includes shell hooks for automatic project switching:

### Bash/Zsh
```bash
# Add to ~/.bashrc or ~/.zshrc
source /path/to/vibe/shell-hooks/vibe-hook.sh
```

### Fish
```fish
# Add to ~/.config/fish/config.fish
source /path/to/vibe/shell-hooks/vibe-hook.fish
```

This enables automatic time tracking when you `cd` into different project directories.

## Configuration

Vibe stores configuration in your system's standard config directory:

- **Linux**: `~/.config/vibe/`
- **macOS**: `~/Library/Application Support/vibe/`
- **Windows**: `%APPDATA%\vibe\`

### Database Location

Time tracking data is stored in SQLite:
- **Database**: `~/.vibe/data.db`
- **Logs**: `~/.vibe/logs/`

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