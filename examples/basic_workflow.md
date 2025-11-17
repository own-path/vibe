# Basic Vibe Workflow Example

This example demonstrates a typical day using Vibe for time tracking.

## Setup

```bash
# Install vibe
cargo install vibe

# Start the daemon
vibe start
```

## Daily Workflow

### Morning Setup

```bash
# Check daemon status
vibe status
# Output: Shows daemon is online

# Navigate to your project
cd ~/projects/my-app

# Start tracking (automatic with shell hooks, or manual)
vibe session start
```

### Working Sessions

```bash
# Check current session
vibe session current
┌─────────────────────────────────────────┐
│           Current Session               │
├─────────────────────────────────────────┤
│ Status:   ● Active                      │
│ Project:  my-app                        │
│ Duration: 1h 23m 45s                    │
│ Started:  09:15:30                      │
│ Context:  terminal                      │
│ Path:     /Users/dev/projects/my-app    │
└─────────────────────────────────────────┘

# Take a break
vibe session pause

# Resume work
vibe session resume

# Switch to different project
cd ~/projects/other-project
# Shell hook automatically starts new session

# Or manually manage
vibe session stop
vibe session start --project other-project --context ide
```

### End of Day

```bash
# Stop current session
vibe session stop

# Generate daily report
vibe report
┌─────────────────────────────────────────┐
│            Time Report                  │
├─────────────────────────────────────────┤
│ my-app               4h 23m 15s         │
│   terminal              2h 45m 30s     │
│   ide                   1h 37m 45s     │
│                                         │
│ other-project        1h 15m 20s        │
│   ide                   1h 15m 20s     │
│                                         │
├─────────────────────────────────────────┤
│ Total Time:              5h 38m 35s    │
└─────────────────────────────────────────┘

# Export for external tools
vibe report --format csv --from 2024-01-01
# Output: "Report exported to: vibe-report.csv"
```

## Advanced Usage

### Project Management

```bash
# Initialize a project with metadata
vibe init --name "My App" --description "Main application project"

# List all projects
vibe list

# Archive completed projects
vibe project archive old-project

# Tag projects for organization
vibe project add-tag my-app "web-development"
vibe project add-tag my-app "rust"
```

### Reporting Options

```bash
# Project-specific reports
vibe report --project my-app

# Date range filtering
vibe report --from 2024-01-01 --to 2024-01-31

# Group by different criteria
vibe report --group week
vibe report --group project

# Export in different formats
vibe report --format json > report.json
vibe report --format csv --project my-app > my-app-time.csv
```

### Session Management

```bash
# View recent sessions
vibe session list --limit 10

# Edit a session (requires session ID)
vibe session edit 123 --start "09:00:00" --reason "Adjusted start time"

# Delete erroneous sessions
vibe session delete 124 --force
```

## Shell Integration Setup

### Bash/Zsh

Add to your `~/.bashrc` or `~/.zshrc`:

```bash
# Enable automatic project switching
source /path/to/vibe/shell-hooks/vibe-hook.sh

# Optional: Enable debug mode
export VIBE_DEBUG=1
```

### Fish Shell

Add to your `~/.config/fish/config.fish`:

```fish
source /path/to/vibe/shell-hooks/vibe-hook.fish
```

## Tips and Best Practices

1. **Consistent Project Structure**: Keep projects in a consistent directory structure for better auto-detection

2. **Use Tags**: Tag projects by technology, client, or type for better reporting

3. **Regular Breaks**: Use pause/resume to exclude break time from tracking

4. **Daily Reviews**: Check your daily report to understand time allocation

5. **Weekly Analysis**: Export weekly reports to identify patterns and optimize workflow

## Troubleshooting

### Daemon Issues

```bash
# If daemon won't start
vibe stop
vibe start

# Check daemon logs
tail -f ~/.vibe/logs/daemon.log
```

### Session Problems

```bash
# If session isn't tracking
vibe session current
vibe session start --force

# Reset if stuck
vibe session stop
vibe restart
```

### Shell Integration

```bash
# Test shell hook manually
source /path/to/vibe/shell-hooks/vibe-hook.sh

# Debug mode for troubleshooting
export VIBE_DEBUG=1
cd ~/projects/my-app  # Should show debug output
```

This basic workflow covers the most common Vibe usage patterns. For more advanced features, see the main documentation.