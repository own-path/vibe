# Contributing to Vibe

Thank you for your interest in contributing to Vibe! This guide will help you get started.

## Code of Conduct

By participating in this project, you agree to abide by our Code of Conduct. Please be respectful and constructive in all interactions.

## Getting Started

### Prerequisites

- Rust 1.70+ (latest stable recommended)
- Git
- Basic familiarity with terminal/CLI tools

### Setting Up Your Development Environment

1. **Fork and Clone**
   ```bash
   git clone https://github.com/yourusername/vibe.git
   cd vibe
   ```

2. **Install Dependencies**
   ```bash
   # Build the project
   cargo build
   
   # Run tests to ensure everything works
   cargo test
   ```

3. **Verify Installation**
   ```bash
   # Run the CLI locally
   cargo run -- status
   ```

## Development Workflow

### Creating a Feature Branch

```bash
git checkout -b feature/your-feature-name
```

### Making Changes

1. **Code Style**: We follow standard Rust formatting
   ```bash
   cargo fmt
   cargo clippy
   ```

2. **Testing**: Ensure all tests pass
   ```bash
   cargo test
   cargo test --release
   ```

3. **Documentation**: Update docs for public APIs
   ```bash
   cargo doc --open
   ```

### Commit Guidelines

We use conventional commits:

```
feat: add new session pause functionality
fix: resolve daemon startup race condition
docs: update README installation instructions
refactor: simplify IPC message handling
test: add integration tests for reporting
```

## Project Structure

```
vibe/
├── src/
│   ├── cli/           # Command-line interface and argument parsing
│   │   ├── commands.rs    # Command implementations
│   │   ├── reports.rs     # Report generation
│   │   └── types.rs       # CLI type definitions
│   ├── daemon/        # Background tracking service
│   │   ├── main.rs        # Daemon entry point
│   │   ├── server.rs      # IPC server implementation
│   │   └── state.rs       # Session state management
│   ├── db/            # Database layer
│   │   ├── connection.rs  # SQLite connection handling
│   │   ├── migrations.rs  # Schema migrations
│   │   └── queries.rs     # Database queries
│   ├── models/        # Data structures
│   │   ├── project.rs     # Project model
│   │   ├── session.rs     # Session model
│   │   └── config.rs      # Configuration model
│   ├── ui/            # Terminal UI components (Ratatui)
│   │   ├── dashboard.rs   # Real-time dashboard
│   │   ├── formatter.rs   # Text formatting utilities
│   │   └── widgets.rs     # Reusable UI components
│   └── utils/         # Shared utilities
│       ├── ipc.rs         # Inter-process communication
│       ├── paths.rs       # File system utilities
│       └── config.rs      # Configuration loading
├── migrations/        # Database schema files
├── shell-hooks/       # Shell integration scripts
└── examples/          # Usage examples and demos
```

## Types of Contributions

### 🐛 Bug Reports

Use the issue template and include:
- Clear description of the problem
- Steps to reproduce
- Expected vs actual behavior
- Your environment (OS, Rust version, etc.)
- Relevant log output

### ✨ Feature Requests

- Describe the use case and motivation
- Provide examples of how it would work
- Consider existing alternatives
- Discuss implementation approach

### 🔧 Code Contributions

#### Areas We Need Help With:

1. **Core Features**
   - Session management improvements
   - Better project auto-detection
   - Enhanced reporting capabilities

2. **UI/UX**
   - Ratatui dashboard enhancements
   - Better CLI output formatting
   - Interactive session management

3. **Platform Support**
   - Windows compatibility improvements
   - Shell integration for PowerShell
   - IDE plugin integrations

4. **Performance**
   - Daemon efficiency optimizations
   - Database query improvements
   - Memory usage reduction

#### Implementation Guidelines:

1. **Database Changes**
   - Add migrations in `migrations/` directory
   - Update models in `src/models/`
   - Test with both SQLite and in-memory databases

2. **CLI Commands**
   - Add command definitions in `src/cli/types.rs`
   - Implement in `src/cli/commands.rs`
   - Include comprehensive error handling

3. **UI Components**
   - Use the existing color scheme (see `src/ui/formatter.rs`)
   - Maintain consistent formatting
   - Test across different terminal sizes

4. **Daemon Features**
   - Ensure thread safety with proper locking
   - Handle IPC communication gracefully
   - Add appropriate logging

## Testing

### Running Tests

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration

# Test with different features
cargo test --features "full"
```

### Adding Tests

1. **Unit Tests**: Add to the same file as your code
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_feature() {
           // Your test here
       }
   }
   ```

2. **Integration Tests**: Create files in `tests/`
   ```rust
   // tests/session_management.rs
   use vibe::*;

   #[test]
   fn test_session_lifecycle() {
       // Integration test here
   }
   ```

## Documentation

- Update docstrings for all public APIs
- Add examples to complex functions
- Update README.md for user-facing changes
- Include changelog entries

## Performance Considerations

- Profile before optimizing
- Minimize database queries
- Use appropriate data structures
- Consider memory usage in daemon
- Test with large datasets

## Release Process

1. **Version Bump**: Update `Cargo.toml`
2. **Changelog**: Update `CHANGELOG.md`
3. **Documentation**: Verify docs are current
4. **Testing**: Run full test suite
5. **Tag Release**: Create git tag
6. **Publish**: `cargo publish`

## Getting Help

- **Discord**: Join our [community server](https://discord.gg/vibe-community)
- **Discussions**: Use [GitHub Discussions](https://github.com/yourusername/vibe/discussions)
- **Issues**: For bugs and feature requests

## Recognition

Contributors will be:
- Listed in our contributors file
- Mentioned in release notes
- Invited to our contributor Discord channel

---

Thank you for contributing to Vibe! 🎉