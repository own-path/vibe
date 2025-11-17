# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project setup with Rust and Cargo
- Daemon architecture for background time tracking
- SQLite database for local data storage
- Beautiful terminal interface with Ratatui
- Context-aware tracking (terminal, IDE, linked, manual)
- Comprehensive CLI with session management
- Shell integration hooks for automatic project detection
- Professional color-coded output with consistent formatting
- Report generation in multiple formats (terminal, CSV, JSON)
- Session pause/resume functionality
- Project management with tagging support
- IPC communication between CLI and daemon
- Cross-platform support (macOS, Linux, Windows)

### Features
- **Core Tracking**: Automatic time tracking with context detection
- **Session Management**: Start, stop, pause, resume sessions with full control
- **Project Organization**: Initialize, list, archive projects with metadata
- **Reporting**: Flexible report generation with date filtering and grouping
- **Shell Integration**: Automatic project switching on directory changes
- **Professional UI**: Color-coded terminal output with consistent formatting
- **Data Persistence**: SQLite database with migration support
- **Configuration**: Flexible configuration system with sensible defaults

### Architecture
- **CLI Client**: Beautiful terminal interface for user interactions
- **Daemon Service**: Background process for continuous tracking
- **Database Layer**: SQLite with proper migrations and queries
- **IPC System**: Unix sockets for client-daemon communication
- **UI Components**: Ratatui-based terminal user interface
- **Shell Hooks**: Automatic integration with Bash, Zsh, Fish, PowerShell

## [0.1.0] - 2024-01-XX

### Added
- Initial release of Vibe time tracking tool
- Core functionality for automatic project time tracking
- Professional terminal interface with color-coded output
- Daemon architecture for reliable background tracking
- Comprehensive CLI with intuitive commands
- Cross-platform support and shell integration

---

**Note**: This project is in active development. Features and APIs may change between releases.