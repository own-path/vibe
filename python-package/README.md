# Vibe CLI

**Automatic project time tracking CLI tool**

A beautiful, intelligent time tracking application that automatically detects your work context and provides detailed insights into your productivity patterns.

## Quick Installation

```bash
pip install vibe-cli
```

That's it! After installation, you can use the `vibe` command directly.

## Quick Start

```bash
# Start the daemon
vibe start

# Begin tracking in current directory
vibe session start

# Check current status
vibe status

# Generate reports
vibe report
```

## Features

- **Automatic Detection**: Seamlessly tracks time across terminal, IDE, and linked project contexts
- **Beautiful CLI Output**: Color-coded, professional terminal interface with context-aware formatting
- **Daemon Architecture**: Lightweight background service for continuous tracking
- **Shell Integration**: Automatic project switching with directory changes
- **Flexible Reporting**: Generate detailed reports in multiple formats (terminal, CSV, JSON)
- **Session Management**: Pause, resume, and edit tracking sessions with full audit trails
- **Cross-platform**: Works on macOS, Linux, and Windows

## Context-Aware Tracking

Vibe automatically detects your work environment and color-codes contexts:

- **Terminal** - Bright Cyan: Command-line development
- **IDE** - Bright Magenta: Integrated development environments  
- **Linked** - Bright Yellow: Multi-project workflows
- **Manual** - Bright Blue: Explicitly started sessions

## Installation Requirements

This package requires Rust to be installed on your system for the initial setup. The installation process will automatically install the Vibe binary via cargo.

### Installing Rust (if not already installed)

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Then install Vibe
pip install vibe-cli
```

## Alternative Installation Methods

If you prefer other installation methods:

```bash
# Via Homebrew (macOS/Linux)
brew tap own-path/tap && brew install vibe

# Via Cargo (direct)
cargo install vibe

# From source
git clone https://github.com/own-path/vibe.git
cd vibe
cargo install --path .
```

## Commands

### Daemon Management
```bash
vibe start              # Start tracking daemon
vibe stop               # Stop daemon
vibe restart            # Restart daemon
vibe status             # Check daemon status
```

### Session Management
```bash
vibe session start      # Start tracking current project
vibe session stop       # Stop current session
vibe session pause      # Pause tracking
vibe session resume     # Resume tracking
vibe session current    # Show current session
vibe session list       # List recent sessions
```

### Project Management
```bash
vibe init               # Initialize project in current directory
vibe list               # List all projects
vibe project archive    # Archive a project
vibe project add-tag    # Add tag to project
```

### Reporting
```bash
vibe report             # Generate time report
vibe report --format csv              # Export as CSV
vibe report --from 2024-01-01         # Date range filtering
vibe report --group week              # Group by week
```

## Shell Integration

Enable automatic project switching by adding to your shell profile:

### Bash/Zsh
```bash
# Add to ~/.bashrc or ~/.zshrc
eval "$(vibe completions bash)"  # or zsh
```

### Fish
```bash
# Add to ~/.config/fish/config.fish
vibe completions fish | source
```

### PowerShell
```powershell
# Add to your PowerShell profile
vibe completions powershell | Out-String | Invoke-Expression
```

## Documentation

For comprehensive documentation, examples, and advanced usage, visit:
- [GitHub Repository](https://github.com/own-path/vibe)
- [Usage Examples](https://github.com/own-path/vibe/blob/main/examples/)

## License

MIT License - see [LICENSE](https://github.com/own-path/vibe/blob/main/LICENSE) for details.

## Support

- [GitHub Issues](https://github.com/own-path/vibe/issues)
- [Discussions](https://github.com/own-path/vibe/discussions)