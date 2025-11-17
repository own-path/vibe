# TimeTrack - Project Time Tracking CLI Tool

## Technical Specification Document

**Version:** 1.0  
**Date:** November 15, 2025  
**Status:** Design Phase

---

## Table of Contents

1. [Overview](#overview)
2. [Core Requirements](#core-requirements)
3. [System Architecture](#system-architecture)
4. [Technical Stack](#technical-stack)
5. [Features](#features)
6. [Data Model](#data-model)
7. [Components](#components)
8. [Error Handling & Recovery](#error-handling--recovery)
9. [Edge Cases & Limitations](#edge-cases--limitations)
10. [Session Management](#session-management)
11. [Resource Monitoring](#resource-monitoring)
12. [Security & Privacy](#security--privacy)
13. [Logging & Debugging](#logging--debugging)
14. [Testing Strategy](#testing-strategy)
15. [Database Migrations](#database-migrations)
16. [Versioning & Compatibility](#versioning--compatibility)
17. [Performance Requirements](#performance-requirements)
18. [Installation Guide](#installation-guide)
19. [Uninstallation Guide](#uninstallation-guide)
20. [Implementation Plan](#implementation-plan)
21. [Future Enhancements](#future-enhancements)

---

## Overview

TimeTrack is an automatic time tracking CLI tool designed for developers who work across terminal and IDE environments. It automatically detects when you enter a project directory and tracks time spent working on different projects with minimal manual intervention.

### Key Principles

- **Automatic**: Auto-start tracking when opening project folders
- **Lightweight**: Minimal resource usage (~5-10MB RAM)
- **Cross-platform**: Support for Linux, macOS, and Windows
- **Privacy-focused**: Minimal data storage, no keystroke logging
- **Flexible**: Support for single or multi-project tracking

---

## Core Requirements

### Functional Requirements

1. **Automatic Project Detection**
   - Detect projects via `.git` folders
   - Support custom `.timetrack` marker files
   - Track time when entering project directories (terminal) or opening workspaces (IDE)

2. **Time Tracking**
   - Single active project tracking (last-focused wins)
   - Optional linked project mode for related projects
   - 30-minute idle timeout with auto-pause
   - Auto-resume on activity detection

3. **Activity Detection**
   - **Terminal**: Directory changes, git commands, file modifications
   - **IDE**: Workspace switches, file saves, editor focus changes

4. **Project Organization**
   - Tag-based project grouping
   - Separate tracking with combined reporting capability
   - Project linking for related work

5. **Reporting**
   - Time summaries per project
   - Tag-based aggregated reports
   - Context breakdown (terminal vs IDE time)
   - Date-range filtering

### Non-Functional Requirements

1. **Performance**
   - RAM usage: 5-10MB maximum
   - CPU: Idle when no activity
   - Database: Minimal storage footprint
   - Startup time: <100ms

2. **Reliability**
   - Daemon auto-start on system boot
   - Graceful handling of crashes (save state)
   - Data integrity during power loss

3. **Usability**
   - Simple CLI interface
   - Minimal configuration required
   - Clear status visibility

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     User Workspace                          │
├─────────────────┬───────────────────┬───────────────────────┤
│  Terminal       │   IDE (Cursor)    │   CLI Commands        │
│  (Shell Hooks)  │   (VS Code Ext)   │   (User Interface)    │
└────────┬────────┴─────────┬─────────┴──────────┬────────────┘
         │                  │                    │
         │   Activity       │   Activity         │   Commands
         │   Signals        │   Signals          │   
         ▼                  ▼                    ▼
    ┌────────────────────────────────────────────────────┐
    │            Unix Socket / IPC Channel               │
    └────────────────────┬───────────────────────────────┘
                         │
                         ▼
         ┌───────────────────────────────────┐
         │       TimeTrack Daemon            │
         │                                   │
         │  ┌─────────────────────────────┐ │
         │  │   Event Loop (Tokio)        │ │
         │  └─────────────────────────────┘ │
         │  ┌─────────────────────────────┐ │
         │  │   State Machine             │ │
         │  │   - Active projects         │ │
         │  │   - Session tracking        │ │
         │  │   - Project switching       │ │
         │  └─────────────────────────────┘ │
         │  ┌─────────────────────────────┐ │
         │  │   Idle Detection            │ │
         │  │   (30min timeout)           │ │
         │  └─────────────────────────────┘ │
         │  ┌─────────────────────────────┐ │
         │  │   SQLite Manager            │ │
         │  └─────────────────────────────┘ │
         └───────────────┬───────────────────┘
                         │
                         ▼
                ┌─────────────────┐
                │  SQLite Database│
                │  - Projects     │
                │  - Sessions     │
                │  - Tags         │
                └─────────────────┘
```

### Component Interaction Flow

1. **Activity Detection**
   ```
   User Activity → Shell Hook / IDE Extension → IPC Signal → Daemon
   ```

2. **Project Switching**
   ```
   New Project Signal → Daemon State Machine → Pause Current → Start/Resume New → Update DB
   ```

3. **Idle Detection**
   ```
   No Activity for 30min → Daemon Pauses Session → Awaits New Signal
   ```

4. **Reporting**
   ```
   CLI Command → Daemon Query → SQLite Read → Format Output → Display
   ```

---

## Technical Stack

### Core Components

| Component | Technology | Justification |
|-----------|-----------|---------------|
| Daemon & CLI | **Rust** | Performance, single binary, cross-platform, memory safety |
| Database | **SQLite** | Embedded, serverless, minimal overhead, ACID compliance |
| IPC | **Unix Sockets** (Linux/macOS)<br>**Named Pipes** (Windows) | Fast, local-only, secure |
| IDE Integration | **VS Code Extension** (TypeScript) | Native Cursor/VS Code support |
| Shell Integration | **Bash/Zsh/PowerShell scripts** | Platform-appropriate hooks |

### Rust Dependencies (Preliminary)

```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }  # Async runtime
rusqlite = "0.30"                                  # SQLite binding
serde = { version = "1.0", features = ["derive"] } # Serialization
clap = { version = "4.0", features = ["derive"] }  # CLI parsing
chrono = "0.4"                                     # Date/time handling
notify = "6.0"                                     # Filesystem watching
sysinfo = "0.30"                                   # System resource monitoring
```

---

## Features

### 1. Project Detection & Initialization

**Automatic Detection:**
- Scan for `.git` directories
- Recognize `.timetrack` marker files

**Manual Initialization:**
```bash
# Create .timetrack marker in current directory
timetrack init

# Initialize with project name
timetrack init --name "My Project"
```

### 2. Tracking Modes

#### Single Project Mode (Default)
- Only one active project at a time
- Last-focused window wins (terminal or IDE)
- Automatic pause/resume on project switch

#### Linked Project Mode
```bash
# Link related projects
timetrack link frontend-app backend-api

# Both projects accumulate time together
# Useful for: microservices, monorepos, related codebases

# Unlink projects
timetrack unlink frontend-app backend-api
```

### 3. Activity Detection

**Terminal Activities:**
- Directory changes (`cd` command via shell hook)
- Git operations (commit, push, pull, etc.)
- File modifications in project directory (optional)

**IDE Activities:**
- Workspace opened/switched
- File saved in workspace
- Active editor changed (debounced - 30s intervals)
- Window focused/unfocused

**Idle Detection:**
- 30-minute inactivity timeout
- Auto-pause current session
- Auto-resume on next activity signal

### 4. Project Organization

**Tagging System:**
```bash
# Add tags to projects
timetrack tag add work project-a project-b
timetrack tag add personal side-project

# Remove tags
timetrack tag remove work project-a

# List projects by tag
timetrack projects --tag work
```

**Project Grouping Benefits:**
- Combined reports across related projects
- Better organization for contractors/consultants
- Client-based time tracking

### 5. Reporting & Analytics

```bash
# Current status
timetrack status
# Output: "Currently tracking: backend-api (Terminal) - 1h 23m"

# Daily summary
timetrack report --today

# Weekly summary
timetrack report --week

# Specific project
timetrack report --project backend-api

# By tag
timetrack report --tag work --week

# Custom date range
timetrack report --from 2025-11-01 --to 2025-11-15

# Export to CSV
timetrack report --export csv --output ~/reports/november.csv
```

**Report Format:**
```
Project Time Report (Nov 11 - Nov 15, 2025)
─────────────────────────────────────────────
backend-api          12h 34m  (Terminal: 8h 12m, IDE: 4h 22m)
frontend-app          8h 15m  (Terminal: 2h 05m, IDE: 6h 10m)
documentation         3h 42m  (Terminal: 0h 15m, IDE: 3h 27m)
─────────────────────────────────────────────
Total:               24h 31m

Tags:
  work:              20h 49m
  personal:           3h 42m
```

### 6. Manual Controls

```bash
# Manual start (if auto-detection fails)
timetrack start project-name

# Manual stop
timetrack stop

# Pause current session
timetrack pause

# Resume paused session
timetrack resume

# Switch to different project manually
timetrack switch other-project
```

---

## Data Model

### Database Schema

```sql
-- Projects table
CREATE TABLE projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT UNIQUE NOT NULL,           -- Absolute path to project
    name TEXT NOT NULL,                   -- Display name
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_active TIMESTAMP,
    is_archived BOOLEAN DEFAULT 0
);

-- Sessions table (time tracking entries)
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP,                   -- NULL if currently active
    context TEXT CHECK(context IN ('terminal', 'ide', 'linked')),
    paused_duration INTEGER DEFAULT 0,    -- Seconds paused
    active_duration INTEGER,              -- Computed: end - start - paused
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

-- Tags table
CREATE TABLE tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL
);

-- Project-Tag relationship (many-to-many)
CREATE TABLE project_tags (
    project_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    PRIMARY KEY (project_id, tag_id),
    FOREIGN KEY (project_id) REFERENCES projects(id),
    FOREIGN KEY (tag_id) REFERENCES tags(id)
);

-- Linked projects (for multi-project tracking)
CREATE TABLE linked_projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT 1
);

CREATE TABLE linked_project_members (
    link_id INTEGER NOT NULL,
    project_id INTEGER NOT NULL,
    PRIMARY KEY (link_id, project_id),
    FOREIGN KEY (link_id) REFERENCES linked_projects(id),
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

-- Indexes for performance
CREATE INDEX idx_sessions_project ON sessions(project_id);
CREATE INDEX idx_sessions_time ON sessions(start_time, end_time);
CREATE INDEX idx_projects_path ON projects(path);
```

### In-Memory State (Daemon)

```rust
struct DaemonState {
    active_session: Option<ActiveSession>,
    last_activity: Instant,
    linked_mode: bool,
    linked_projects: Vec<ProjectId>,
}

struct ActiveSession {
    project_id: ProjectId,
    start_time: DateTime<Utc>,
    context: Context,  // Terminal | IDE | Linked
    pauses: Vec<PausePeriod>,
}

enum Context {
    Terminal,
    IDE,
    Linked,  // Multiple projects tracked together
}

struct PausePeriod {
    start: DateTime<Utc>,
    end: Option<DateTime<Utc>>,
}
```

---

## Components

### 1. Daemon (Rust)

**File Structure:**
```
timetrack/
├── src/
│   ├── main.rs              # Entry point, CLI parsing
│   ├── daemon/
│   │   ├── mod.rs           # Daemon module
│   │   ├── server.rs        # IPC server (Unix socket)
│   │   ├── state.rs         # State machine
│   │   ├── idle.rs          # Idle detection logic
│   │   └── events.rs        # Event handling
│   ├── db/
│   │   ├── mod.rs
│   │   ├── schema.rs        # Database schema
│   │   ├── queries.rs       # SQL queries
│   │   └── migrations.rs    # Schema migrations
│   ├── models/
│   │   ├── mod.rs
│   │   ├── project.rs
│   │   ├── session.rs
│   │   └── tag.rs
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── commands.rs      # CLI command handlers
│   │   └── output.rs        # Report formatting
│   └── utils/
│       ├── config.rs        # Configuration
│       └── ipc.rs           # IPC client
├── Cargo.toml
└── README.md
```

**Key Responsibilities:**
- Listen for activity signals via IPC
- Maintain current session state in memory
- Detect idle periods (30min timeout)
- Handle project switching logic
- Persist sessions to SQLite
- Serve CLI commands

**IPC Protocol (JSON over Unix Socket):**
```json
// Activity signal from shell/IDE
{
  "type": "activity",
  "source": "terminal" | "ide",
  "project_path": "/home/user/projects/myapp",
  "timestamp": "2025-11-15T10:30:00Z"
}

// Response
{
  "status": "ok",
  "active_project": "myapp",
  "session_duration": 3600  // seconds
}
```

### 2. Shell Integration

#### Bash/Zsh Hook (`~/.timetrack/shell/hook.sh`)

```bash
# TimeTrack shell integration
_timetrack_project_change() {
    local current_path="$PWD"
    
    # Check if we're in a tracked project
    if [ -d ".git" ] || [ -f ".timetrack" ]; then
        # Send signal to daemon
        echo "{\"type\":\"activity\",\"source\":\"terminal\",\"project_path\":\"$current_path\",\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}" | \
            nc -U ~/.timetrack/daemon.sock 2>/dev/null || true
    fi
}

# Hook into directory changes
if [ -n "$ZSH_VERSION" ]; then
    # Zsh
    chpwd_functions+=(_timetrack_project_change)
elif [ -n "$BASH_VERSION" ]; then
    # Bash
    PROMPT_COMMAND="_timetrack_project_change;$PROMPT_COMMAND"
fi

# Initial check on shell start
_timetrack_project_change
```

#### PowerShell Profile (Windows)

```powershell
# TimeTrack PowerShell integration
function Send-TimeTrackActivity {
    $currentPath = Get-Location
    
    if ((Test-Path ".git") -or (Test-Path ".timetrack")) {
        $payload = @{
            type = "activity"
            source = "terminal"
            project_path = $currentPath.Path
            timestamp = (Get-Date).ToUniversalTime().ToString("o")
        } | ConvertTo-Json -Compress
        
        # Send to named pipe
        try {
            $pipe = New-Object System.IO.Pipes.NamedPipeClientStream(".", "timetrack", [System.IO.Pipes.PipeDirection]::Out)
            $pipe.Connect(100)
            $writer = New-Object System.IO.StreamWriter($pipe)
            $writer.WriteLine($payload)
            $writer.Flush()
            $pipe.Close()
        } catch {
            # Silently fail if daemon not running
        }
    }
}

# Hook into location changes
$ExecutionContext.InvokeCommand.LocationChangedAction = {
    Send-TimeTrackActivity
}

# Initial check
Send-TimeTrackActivity
```

**Installation:**
```bash
# Add to shell profile
echo 'source ~/.timetrack/shell/hook.sh' >> ~/.bashrc  # or ~/.zshrc
```

### 3. VS Code Extension

**Extension Structure:**
```
timetrack-vscode/
├── src/
│   ├── extension.ts         # Main extension entry
│   ├── tracker.ts           # Activity tracking logic
│   ├── client.ts            # IPC client to daemon
│   └── config.ts            # Extension configuration
├── package.json
└── README.md
```

**Key Features:**
```typescript
// extension.ts
import * as vscode from 'vscode';

export function activate(context: vscode.ExtensionContext) {
    const tracker = new TimeTracker();
    
    // Track workspace changes
    vscode.workspace.onDidChangeWorkspaceFolders(() => {
        tracker.sendActivity('workspace_change');
    });
    
    // Track active editor (debounced)
    let debounceTimer: NodeJS.Timeout;
    vscode.window.onDidChangeActiveTextEditor((editor) => {
        clearTimeout(debounceTimer);
        debounceTimer = setTimeout(() => {
            if (editor && isInTrackedProject(editor.document.uri)) {
                tracker.sendActivity('editor_change');
            }
        }, 30000); // 30 second debounce
    });
    
    // Track file saves
    vscode.workspace.onDidSaveTextDocument((document) => {
        if (isInTrackedProject(document.uri)) {
            tracker.sendActivity('file_save');
        }
    });
    
    // Track window focus
    vscode.window.onDidChangeWindowState((state) => {
        if (state.focused) {
            tracker.sendActivity('window_focus');
        }
    });
}

class TimeTracker {
    private client: DaemonClient;
    
    constructor() {
        this.client = new DaemonClient();
    }
    
    sendActivity(activityType: string) {
        const workspacePath = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
        if (!workspacePath) return;
        
        this.client.send({
            type: 'activity',
            source: 'ide',
            project_path: workspacePath,
            timestamp: new Date().toISOString(),
            activity_type: activityType
        });
    }
}

function isInTrackedProject(uri: vscode.Uri): boolean {
    // Check if URI is within a workspace that has .git or .timetrack
    const workspace = vscode.workspace.getWorkspaceFolder(uri);
    if (!workspace) return false;
    
    const fs = require('fs');
    const path = require('path');
    
    return fs.existsSync(path.join(workspace.uri.fsPath, '.git')) ||
           fs.existsSync(path.join(workspace.uri.fsPath, '.timetrack'));
}
```

**package.json Configuration:**
```json
{
  "name": "timetrack",
  "displayName": "TimeTrack",
  "description": "Automatic project time tracking",
  "version": "0.1.0",
  "engines": {
    "vscode": "^1.80.0"
  },
  "activationEvents": [
    "onStartupFinished"
  ],
  "main": "./out/extension.js",
  "contributes": {
    "configuration": {
      "title": "TimeTrack",
      "properties": {
        "timetrack.enabled": {
          "type": "boolean",
          "default": true,
          "description": "Enable automatic time tracking"
        },
        "timetrack.debounceInterval": {
          "type": "number",
          "default": 30000,
          "description": "Debounce interval for activity signals (ms)"
        }
      }
    }
  }
}
```

### 4. CLI Tool

**Commands:**

```bash
# Daemon management
timetrack daemon start          # Start daemon
timetrack daemon stop           # Stop daemon
timetrack daemon status         # Check daemon status
timetrack daemon restart        # Restart daemon

# Project management
timetrack init [--name NAME]    # Initialize project in current directory
timetrack projects              # List all tracked projects
timetrack projects --tag TAG    # List projects by tag
timetrack archive PROJECT       # Archive a project (stop tracking)

# Tracking controls
timetrack start [PROJECT]       # Manually start tracking
timetrack stop                  # Stop tracking
timetrack pause                 # Pause current session
timetrack resume                # Resume paused session
timetrack switch PROJECT        # Switch to different project
timetrack status                # Show current tracking status

# Linking projects
timetrack link PROJ1 PROJ2...   # Link projects for combined tracking
timetrack unlink PROJ1 PROJ2... # Unlink projects
timetrack links                 # Show all linked project groups

# Tagging
timetrack tag add TAG PROJ...   # Add tag to projects
timetrack tag remove TAG PROJ...# Remove tag from projects
timetrack tag list              # List all tags

# Reporting
timetrack report                # Summary of today
timetrack report --today        # Today's time
timetrack report --yesterday    # Yesterday's time
timetrack report --week         # This week
timetrack report --month        # This month
timetrack report --project PROJ # Specific project
timetrack report --tag TAG      # By tag
timetrack report --from DATE --to DATE  # Custom range
timetrack report --export csv --output FILE  # Export to CSV

# Configuration
timetrack config set KEY VALUE  # Set config value
timetrack config get KEY        # Get config value
timetrack config list           # List all config
```

**Sample Output:**

```bash
$ timetrack status
Currently tracking: backend-api (Terminal)
Session duration: 1h 23m 45s
Last activity: 2 minutes ago

$ timetrack report --week
─────────────────────────────────────────────
Weekly Report (Nov 11 - Nov 15, 2025)
─────────────────────────────────────────────
backend-api          12h 34m
  Terminal:           8h 12m
  IDE:                4h 22m
  
frontend-app          8h 15m
  Terminal:           2h 05m
  IDE:                6h 10m
  
documentation         3h 42m
  Terminal:           0h 15m
  IDE:                3h 27m
─────────────────────────────────────────────
Total:               24h 31m

Top tags:
  work:              20h 49m
  personal:           3h 42m
```

---

## Error Handling & Recovery

### Crash Recovery

**Interrupted Session Handling:**

When the daemon crashes or is forcefully terminated while tracking a session, the recovery process follows these steps:

1. **Session State Persistence**
   - Daemon writes session state to SQLite every 5 minutes (configurable via `flush_interval`)
   - On each project switch or pause, state is immediately flushed
   - Last known state includes: active project, start time, last activity timestamp

2. **Recovery on Daemon Restart**
   ```
   Daemon Start → Check for incomplete sessions → Apply recovery logic
   ```

3. **Recovery Logic**
   ```rust
   // Pseudocode for recovery
   fn recover_sessions() {
       let incomplete_sessions = db.get_sessions_where(end_time IS NULL);
       
       for session in incomplete_sessions {
           let crash_time = last_daemon_shutdown_time();
           let time_since_last_activity = crash_time - session.last_activity;
           
           if time_since_last_activity < idle_timeout {
               // Assume work continued until crash
               session.end_time = crash_time;
               session.mark_as_recovered();
           } else {
               // Assume idle before crash
               session.end_time = session.last_activity + idle_timeout;
               session.mark_as_recovered();
           }
           
           db.update_session(session);
       }
   }
   ```

4. **Recovery Markers**
   - Add `recovery_status` field to sessions table: `normal | recovered | manual_edit`
   - Allow users to view and edit recovered sessions
   - Log recovery actions for audit trail

**Example Recovery Scenarios:**

| Scenario | Last Activity | Crash Time | Recovery Action |
|----------|--------------|------------|-----------------|
| Working when crashed | 10:55 AM | 11:00 AM | End session at 11:00 AM |
| Idle before crash | 10:00 AM | 11:00 AM | End session at 10:30 AM (idle timeout) |
| Power loss | 3:00 PM | Unknown | End session at 3:30 PM (last activity + timeout) |

### Database Corruption

**Detection:**
- On startup, run SQLite integrity check: `PRAGMA integrity_check;`
- If corruption detected, attempt automatic repair

**Repair Process:**
```bash
# Automatic repair sequence
1. Copy corrupted DB to ~/.timetrack/corrupted/data.db.[timestamp]
2. Attempt SQLite recovery: sqlite3 .recover
3. If recovery fails, restore from most recent backup
4. If no backup, create new DB and notify user
5. Log all recovery actions
```

**Prevention:**
- Enable SQLite Write-Ahead Logging (WAL mode)
- Use `PRAGMA synchronous = FULL` for critical writes
- Atomic file operations for config files

### Network/IPC Failures

**Socket Connection Issues:**

```rust
// Retry logic for IPC communication
fn send_activity_signal(signal: ActivitySignal) -> Result<(), Error> {
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY: Duration = Duration::from_millis(100);
    
    for attempt in 1..=MAX_RETRIES {
        match try_send_signal(&signal) {
            Ok(_) => return Ok(()),
            Err(e) if is_retriable(&e) => {
                if attempt < MAX_RETRIES {
                    thread::sleep(RETRY_DELAY * attempt);
                    continue;
                }
            }
            Err(e) => return Err(e),
        }
    }
    
    // If all retries fail, queue for later (optional)
    queue_for_retry(&signal);
    Err(Error::MaxRetriesExceeded)
}
```

**Daemon Unavailable:**
- Shell hooks and IDE extension fail silently if daemon is not running
- Queue activity signals in memory (max 100 signals)
- Flush queued signals when daemon becomes available
- Prevent infinite queue growth

### Configuration Errors

**Invalid Config File:**
```rust
// Config loading with fallback
fn load_config() -> Config {
    match Config::from_file(config_path()) {
        Ok(config) => validate_config(config),
        Err(e) => {
            log_error!("Failed to load config: {}", e);
            log_info!("Using default configuration");
            Config::default()
        }
    }
}

fn validate_config(config: Config) -> Config {
    // Validate and clamp values
    let idle_timeout = config.idle_timeout.max(60).min(7200); // 1min to 2hrs
    let max_memory = config.max_memory_mb.max(5).min(100);
    
    Config {
        idle_timeout,
        max_memory_mb: max_memory,
        ..config
    }
}
```

### Graceful Degradation

**Component Failures:**

| Component | Failure Mode | Degradation Strategy |
|-----------|--------------|---------------------|
| Shell Hook | Script error | Silent fail, log warning, continue |
| IDE Extension | Connection timeout | Queue signals, retry later |
| Database | Write failure | Buffer in memory, retry writes, alert user |
| File System | Disk full | Stop new sessions, alert user, suggest cleanup |

### User Notifications

**Error Notification Levels:**

1. **Silent** (logged only): Transient network issues, retryable errors
2. **Warning** (CLI status shows warning): Recovered sessions, config fallbacks
3. **Critical** (immediate user notification): Database corruption, disk full, permission denied

**Notification Channels:**
```bash
# CLI status shows warnings
$ timetrack status
⚠️  Warning: Last session was recovered after daemon crash
Currently tracking: backend-api (Terminal) - 1h 23m

# Critical errors via stderr
$ timetrack daemon start
Error: Cannot start daemon - another instance is running (PID 12345)
```

---

## Edge Cases & Limitations

### Terminal Edge Cases

**1. Symlinks & Real Paths**
- **Issue**: Project accessed via symlink vs real path
- **Solution**: Always canonicalize paths to real absolute paths
- **Implementation**: Use `std::fs::canonicalize()` in Rust
```rust
let real_path = std::fs::canonicalize(project_path)?;
```

**2. Multiple Terminal Windows**
- **Issue**: User opens multiple terminals in the same project
- **Behavior**: All terminals send activity signals → last activity wins
- **Effect**: Idle timeout resets on any terminal activity in that project
- **Note**: This is desired behavior (any activity = working)

**3. Docker Containers & Remote Sessions**
- **SSH Sessions**: Shell hooks work if installed on remote machine
- **Docker**: Shell hooks must be installed inside container
- **Limitation**: Cross-machine tracking not supported in v1.0
- **Workaround**: Install TimeTrack in each environment independently

**4. Nested Projects**
- **Issue**: Project A contains Project B (e.g., monorepo with submodules)
- **Behavior**: Most specific (deepest) project wins
```bash
/projects/monorepo/.git          # Parent project
/projects/monorepo/backend/.git  # Child project

cd /projects/monorepo/backend    # Tracks "backend"
cd /projects/monorepo            # Tracks "monorepo"
```

**5. Network Mounted Directories**
- **NFS/SMB**: Supported, but may have latency issues
- **Cloud Sync (Dropbox, OneDrive)**: Supported, but .git detection may be unreliable
- **Recommendation**: Keep projects on local disk for best performance

### IDE Edge Cases

**1. Multi-Root Workspaces**
- **VS Code Feature**: Workspace with multiple project folders
- **Behavior**: Track primary (first) root folder
- **Alternative**: User can manually switch tracked project via CLI
```bash
timetrack switch secondary-project
```

**2. Split Editors & Multiple Files**
- **Issue**: Multiple files open from different projects simultaneously
- **Behavior**: Track the workspace of the active editor
- **Debouncing**: 30-second debounce prevents rapid switching

**3. Remote Development (SSH, WSL, Containers)**
- **VS Code Remote**: Daemon must run on remote machine
- **WSL**: Daemon runs in WSL, Windows shell hook separate
- **Dev Containers**: Install TimeTrack in container image

**4. IDE Extensions Disabled**
- **Behavior**: Tracking falls back to terminal-only
- **Detection**: Daemon logs warning if no IDE signals received for 1 week
- **User Action**: Re-enable extension or ignore warning

**5. JetBrains IDEs (IntelliJ, PyCharm, WebStorm)**
- **Current Support**: VS Code/Cursor only in v1.0
- **Workaround**: Terminal tracking still works
- **Future**: v2.0 will include JetBrains plugin (see Future Enhancements)

### Time Tracking Edge Cases

**1. Overnight Sessions**
- **Issue**: User leaves IDE open and computer on overnight
- **Behavior**: 
  - If no activity for 30 minutes → auto-pause
  - Session can span multiple days
  - Reports group by day (split multi-day sessions)

**2. System Sleep/Hibernate**
- **Detection**: On wake, daemon checks elapsed time
- **Behavior**: If sleep duration > 5 minutes → pause session before sleep
- **Implementation**: 
```rust
// Detect sleep by comparing system time vs monotonic clock
let system_elapsed = SystemTime::now() - last_check_system_time;
let monotonic_elapsed = Instant::now() - last_check_monotonic;

if system_elapsed > monotonic_elapsed + Duration::from_secs(300) {
    // System slept for 5+ minutes
    pause_session_retroactively(last_activity_time);
}
```

**3. Clock Changes (DST, Manual Adjustment)**
- **All timestamps stored in UTC** (immune to DST)
- **Display in local time** for user convenience
- **Reports**: Use UTC for calculations, convert to local for display

**4. Rapid Project Switching**
- **Issue**: User switches between 5 projects in 2 minutes
- **Rate Limiting**: Maximum 1 project switch per 10 seconds
- **Behavior**: Rapid switches ignored, last switch wins after 10-second settle
- **Rationale**: Prevents accidental tracking from quick navigation

**5. Manual Time Conflicts**
- **Issue**: User manually starts project A while project B is auto-tracked
- **Resolution**: Manual commands always win
```bash
# Auto-tracking backend-api in terminal
$ timetrack start frontend-app  # Manual override
# Now tracking: frontend-app (Manual)
```

### Data Integrity Edge Cases

**1. Database File Deleted Mid-Session**
- **Detection**: Write failure on next flush
- **Behavior**: Create new database, log error, continue tracking new session
- **User Action**: Lost historical data (unless backup exists)

**2. Concurrent Daemon Instances**
- **Prevention**: PID file lock mechanism
- **Detection**: Check PID file on startup
- **Behavior**: Second instance refuses to start
```bash
$ timetrack daemon start
Error: Daemon already running (PID 12345)
Use 'timetrack daemon restart' to restart
```

**3. Disk Full**
- **Detection**: SQLite write failure (SQLITE_FULL error)
- **Behavior**: 
  - Stop accepting new sessions
  - Keep current session in memory only
  - Alert user via CLI status
  - Suggest cleanup or database export

### Path Handling & Canonicalization Rules

**1. Path Normalization**
- All paths canonicalized to absolute real paths before storage
- Symlinks resolved to target paths
- Environment variables expanded (`~` → `/home/user`)

```rust
fn canonicalize_project_path(path: &str) -> Result<PathBuf> {
    // Expand ~ and environment variables
    let expanded = shellexpand::full(path)?;
    
    // Convert to absolute path
    let absolute = if Path::new(&*expanded).is_relative() {
        env::current_dir()?.join(&*expanded)
    } else {
        PathBuf::from(&*expanded)
    };
    
    // Resolve symlinks and canonicalize
    let canonical = absolute.canonicalize()?;
    
    // Remove trailing slashes
    let normalized = canonical.to_string_lossy().trim_end_matches('/');
    
    Ok(PathBuf::from(normalized))
}
```

**2. Case Sensitivity**
- **Windows**: Case-insensitive comparison
- **Unix/macOS**: Case-sensitive comparison
- **Storage**: Preserve original case as returned by canonicalize

```rust
#[cfg(target_os = "windows")]
fn paths_equal(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
}

#[cfg(not(target_os = "windows"))]
fn paths_equal(a: &Path, b: &Path) -> bool {
    a == b
}
```

**3. Trailing Slashes**
- Always removed before storage
- `/home/user/project/` → `/home/user/project`

**4. Relative Paths**
- Shell hooks always send absolute paths
- CLI commands convert relative to absolute
- Working directory context used for resolution

**5. Unicode & Special Characters**
- Full Unicode support (UTF-8)
- No restrictions on characters except OS limitations
- Examples that work: `~/projects/プロジェクト/`, `/home/user/my project/`

### Performance Limitations

**1. Large Number of Projects**
- **Tested Up To**: 1,000 projects
- **Performance**: O(n) query time for project lookups
- **Recommendation**: Archive projects after 1 year of inactivity

**2. Very Long Sessions**
- **Soft Limit**: 12 hours (warning issued)
- **Hard Limit**: 48 hours (session auto-ends)
- **Rationale**: Prevent forgotten sessions from skewing data

**3. High-Frequency Activity Signals**
- **Rate Limiting**: 10 signals/sec per source
- **Debouncing**: 30s minimum between IDE signals
- **Buffer Size**: 100 queued signals maximum

### Known Limitations (v1.0)

**Not Supported:**
- Multi-machine synchronization (manual export/import planned for v2.0)
- Team/collaborative tracking
- JetBrains IDE auto-detection (terminal tracking works)
- Browser-based IDEs (requires browser extension)
- Mobile IDEs (Xcode/Android Studio work via terminal only)

**Platform-Specific:**

| Platform | Limitation | Workaround |
|----------|-----------|------------|
| Windows | WSL projects need separate daemon | Run daemon in WSL |
| macOS | No focus tracking (future feature) | Terminal + VS Code works |
| Linux | Wayland limited support | X11 fully supported |

### Offline-First Design

**Core Principle**: TimeTrack is **100% offline-first** and requires **zero network connectivity**

**Guarantees:**
- ✅ Works without internet
- ✅ All data stored locally
- ✅ No cloud dependencies
- ✅ No telemetry or tracking
- ✅ No license servers

**Network Usage:**
- Normal operation: **Zero network traffic**
- Optional features: Manual cloud backup (user-initiated)
- Future: Optional sync (explicitly opt-in, v2.0)

**Multi-Machine Scenarios:**
- Each machine tracks independently
- No automatic sync between machines
- Manual export/import for consolidation (v2.0)

**Example Multi-Machine Workflow:**
```bash
# Machine 1 (Laptop)
$ timetrack report --export csv --output ~/laptop-time.csv

# Machine 2 (Desktop) - Transfer file manually
$ timetrack import ~/laptop-time.csv --merge
# Intelligently merges, detects overlaps by timestamp
```

---

## Session Management

### Timezone Handling

**Storage Strategy:**
- **Database**: All timestamps in UTC (ISO 8601)
- **System Time**: Use system's configured timezone
- **Display**: Convert UTC to local timezone for user
- **Reports**: Local timezone unless --utc flag specified

**Implementation:**
```rust
use chrono::{DateTime, Utc, Local};

// Store (always UTC)
fn store_timestamp() -> DateTime<Utc> {
    Utc::now()
}

// Display (local time)
fn display_timestamp(utc: DateTime<Utc>) -> String {
    let local: DateTime<Local> = utc.into();
    local.format("%Y-%m-%d %H:%M:%S %Z").to_string()
}

// Reports use local time
fn generate_report(start_utc: DateTime<Utc>, end_utc: DateTime<Utc>) {
    let start_local: DateTime<Local> = start_utc.into();
    let end_local: DateTime<Local> = end_utc.into();
    // ... generate report with local times
}
```

**DST Handling:**
- UTC storage immune to DST transitions
- Reports automatically adjust for DST
- No manual intervention needed

**Timezone Changes:**
- User changes system timezone → reports recalculate automatically
- Travel across timezones → works correctly (relative to system time)
- No special handling required

**Example:**
```bash
# New York (EST, UTC-5)
$ timetrack start backend-api
Started: 10:00 AM EST

# Database: 2025-11-15T15:00:00Z

# London (GMT, UTC+0)
$ timetrack status
Started: 3:00 PM GMT (5 hours ago)
```

### Session Boundary Rules

**Daily Boundaries:**
- Sessions can span midnight (continuous work)
- Reports split multi-day sessions by calendar day
- Each day's portion counted separately

**Example:**
```
Session: 11:00 PM Nov 14 → 2:00 AM Nov 15 (3 hours total)

Daily breakdown:
  Nov 14: 1h (11 PM - midnight)
  Nov 15: 2h (midnight - 2 AM)
```

**System Sleep Detection:**

```rust
fn on_system_wake() {
    let sleep_duration = detect_sleep_duration();
    
    if sleep_duration > Duration::from_secs(300) {  // 5+ min
        if let Some(mut session) = get_active_session() {
            let sleep_start = system_sleep_timestamp();
            session.add_pause(sleep_start, Utc::now());
            log_info!("Auto-paused during system sleep");
        }
    }
}

fn detect_sleep_duration() -> Duration {
    // Compare system clock jump vs monotonic clock
    let system_elapsed = SystemTime::now() - last_system_check;
    let monotonic_elapsed = Instant::now() - last_monotonic_check;
    
    if system_elapsed > monotonic_elapsed {
        system_elapsed - monotonic_elapsed
    } else {
        Duration::ZERO
    }
}
```

**Sleep Behavior:**

| Sleep Duration | Behavior |
|----------------|----------|
| < 5 minutes | Ignore (brief screen lock) |
| 5-30 minutes | Pause session at sleep start |
| > 30 minutes | Pause + idle timeout |

**Hibernate/Shutdown:**
- Session pauses at last activity
- Daemon recovery on restart (see Error Handling)
- User can manually adjust timestamps if needed

**Maximum Session Duration:**
- **Warning**: 12 hours (flagged in reports)
- **Hard Limit**: 48 hours (auto-terminated)
- Sessions exceeding limits marked for review

**Session Splitting in Reports:**
```bash
$ timetrack report --week
⚠️  Long session detected:
backend-api: 14h 32m (Mon 9 AM - Tue 11:32 AM)
  Monday: 9h 0m
  Tuesday: 5h 32m
```

### Rate Limiting & Activity Signals

**Purpose**: Prevent daemon overload, maintain stability

**Rate Limits:**

| Source | Max Rate | Window | On Exceed |
|--------|----------|--------|-----------|
| Shell Hook | 10/sec | 1 second | Drop, log warning |
| IDE Extension | 2/sec | 1 second | Drop, log warning |
| CLI | 5/sec | 1 second | Queue, serialize |

**Implementation:**
```rust
struct RateLimiter {
    window: VecDeque<Instant>,
    max_requests: usize,
    window_duration: Duration,
}

impl RateLimiter {
    fn allow_request(&mut self) -> bool {
        let now = Instant::now();
        
        // Remove old entries
        self.window.retain(|&t| now.duration_since(t) < self.window_duration);
        
        // Check limit
        if self.window.len() < self.max_requests {
            self.window.push_back(now);
            true
        } else {
            false  // Rate limited
        }
    }
}
```

**Debouncing:**
- IDE file saves: 30 seconds minimum
- Project switches: 10 seconds minimum
- Status queries: No debouncing (read-only)

**Rate Limit Exceeded:**
```bash
# Daemon logs
[WARN] Rate limit exceeded: shell_hook (15/sec, max 10/sec)
[INFO] Dropping excess signals to maintain stability

# User notification
$ timetrack status
⚠️  High activity rate detected (possible integration issue)
Currently tracking: backend-api (1h 23m)
```

---

## Resource Monitoring

### Memory Management

**Target**: 5-10MB RAM usage

**Monitoring:**
```rust
use sysinfo::{System, SystemExt, ProcessExt};

fn monitor_memory_usage() {
    let mut sys = System::new_all();
    sys.refresh_process(std::process::id());
    
    if let Some(process) = sys.process(std::process::id()) {
        let memory_mb = process.memory() / 1024 / 1024;
        
        if memory_mb > MAX_MEMORY_MB {
            log_warn!("Memory usage high: {}MB (limit: {}MB)", memory_mb, MAX_MEMORY_MB);
            trigger_cleanup();
        }
    }
}

fn trigger_cleanup() {
    // Force garbage collection
    // Flush old sessions from memory
    // Clear log buffers
    // Compact in-memory structures
}
```

**Memory Limits:**
- **Soft Limit**: 10MB (trigger cleanup)
- **Hard Limit**: 20MB (refuse new sessions, alert user)

**Cleanup Strategy:**
1. Flush buffered data to database
2. Clear old log entries from memory
3. Compact session history cache
4. Drop queued signals if buffer full

### CPU Management

**Target**: <1% CPU when active, 0% when idle

**Optimization Strategies:**
- Event-driven architecture (no polling)
- Sleep when no activity
- Efficient data structures (no unnecessary allocations)

**CPU Monitoring:**
```rust
fn monitor_cpu_usage() {
    let mut sys = System::new_all();
    sys.refresh_process(std::process::id());
    
    if let Some(process) = sys.process(std::process::id()) {
        let cpu_usage = process.cpu_usage();
        
        if cpu_usage > CPU_LIMIT_PERCENT {
            log_warn!("High CPU usage: {}%", cpu_usage);
            // Investigate and log cause
        }
    }
}
```

### Database Size Management

**Target**: <1MB per month, <50MB per year

**Strategies:**
1. **Session Aggregation** (after 90 days)
```sql
-- Aggregate old sessions by day
INSERT INTO daily_summaries (project_id, date, total_duration)
SELECT project_id, DATE(start_time), SUM(active_duration)
FROM sessions
WHERE start_time < DATE('now', '-90 days')
GROUP BY project_id, DATE(start_time);

-- Delete aggregated sessions
DELETE FROM sessions WHERE start_time < DATE('now', '-90 days');
```

2. **Automatic VACUUM** (weekly)
```rust
fn vacuum_database() {
    db.execute("VACUUM", [])?;
    log_info!("Database vacuumed, space reclaimed");
}
```

3. **Backup Rotation** (keep 7 daily, 4 weekly)
```bash
~/.timetrack/backups/
  daily/
    data-2025-11-15.db
    data-2025-11-14.db
    ... (keep 7)
  weekly/
    data-2025-w46.db
    ... (keep 4)
```

### Disk I/O Management

**Target**: <1MB writes per day

**Optimization:**
- Batch writes (flush every 5 minutes)
- Write-Ahead Logging (WAL mode)
- Minimize fsync calls

**I/O Monitoring:**
```rust
fn monitor_io() {
    // Track write operations
    // Log if writes exceed threshold
    // Alert user if disk performance degrades
}
```

### Resource Limit Enforcement

**Enforcement Actions:**

| Resource | Soft Limit | Action | Hard Limit | Action |
|----------|-----------|--------|-----------|--------|
| RAM | 10MB | Trigger cleanup | 20MB | Refuse new sessions |
| CPU | 5% avg | Log warning | 10% avg | Throttle processing |
| Disk | 100MB | Suggest vacuum | 500MB | Refuse writes |
| Signals/sec | 10 | Drop excess | 50 | Disable source |

**User Notification:**
```bash
$ timetrack status
⚠️  Resource limits approaching:
  Memory: 12MB / 10MB (cleanup triggered)
  Database: 95MB / 100MB (consider vacuum)

$ timetrack daemon vacuum
Vacuuming database...
Freed 23MB of disk space
Database size: 72MB
```

### Self-Monitoring Dashboard

```bash
$ timetrack daemon stats
TimeTrack Daemon Statistics
───────────────────────────────────────
Uptime:              3d 14h 23m
Memory Usage:        7.2 MB / 10 MB
CPU Usage (avg):     0.3%
Database Size:       42 MB
Active Sessions:     1
Total Projects:      23
Signals Today:       1,247

Recent Activity:
  10:23 AM  Project switch: frontend → backend
  10:15 AM  IDE signal received
  10:02 AM  Shell activity detected

Warnings:            None
Last Vacuum:         2 days ago
Last Backup:         Today 3:00 AM
```

---

## Security & Privacy

### Data Security

**Storage Security:**
- All data stored in `~/.timetrack/` directory
- File permissions: `0700` (user-only access)
- Database file: `0600` (read/write owner only)
- Socket: `0600` (owner only)

**Encryption (Optional Feature):**
```toml
# config.toml
[security]
encrypt_database = true  # Default: false
encryption_password = "prompt"  # or "keychain"
```

```rust
// Database encryption using SQLCipher
fn open_encrypted_db(password: &str) -> Connection {
    let conn = Connection::open(db_path)?;
    conn.execute(&format!("PRAGMA key = '{}'", password), [])?;
    conn
}
```

**Password Management:**
- Option 1: Prompt on daemon start
- Option 2: Store in system keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- Never store plaintext passwords in config

### IPC Security

**Unix Socket Security:**
- Socket path: `~/.timetrack/daemon.sock`
- Permissions: `0600` (owner only)
- No network exposure (local only)

**Authentication:**
```rust
// Verify client is same user as daemon
fn authenticate_client(stream: &UnixStream) -> Result<bool> {
    let creds = stream.peer_cred()?;
    let daemon_uid = users::get_current_uid();
    
    Ok(creds.uid() == daemon_uid)
}
```

**Connection Limits:**
- Max concurrent connections: 10
- Connection timeout: 30 seconds
- Rate limiting per connection

### Privacy Guarantees

**What is Tracked:**
- ✅ Project paths
- ✅ Session start/end times
- ✅ Context (terminal/IDE)
- ✅ Pause durations

**What is NOT Tracked:**
- ❌ File contents
- ❌ File names (except for project detection)
- ❌ Commands run
- ❌ Keystrokes
- ❌ Screenshots
- ❌ URLs visited
- ❌ Network traffic
- ❌ Personal information

**No Telemetry:**
- Zero data sent to external servers
- No crash reports without explicit user consent
- No usage analytics
- No tracking pixels or beacons

**Data Ownership:**
- User owns all data (stored locally)
- Easy export: CSV, JSON
- Easy deletion: Remove `~/.timetrack/` directory
- No vendor lock-in

### Multi-User Security

**Isolation:**
- Each user has separate daemon instance
- No cross-user data access
- Separate sockets per user

**PID File Protection:**
```bash
~/.timetrack/daemon.pid  # Permissions: 0600
```

**Prevent Privilege Escalation:**
```rust
// Never run as root
fn check_not_root() {
    if users::get_current_uid() == 0 {
        eprintln!("Error: Do not run TimeTrack as root");
        std::process::exit(1);
    }
}
```

### Audit Trail

**Optional Audit Logging:**
```toml
[security]
enable_audit_log = true  # Default: false
audit_log_path = "~/.timetrack/audit.log"
```

**Audit Events:**
- Session start/stop/pause
- Manual time edits
- Project switches
- Configuration changes
- Database operations (vacuum, backup, restore)

**Audit Log Format:**
```
2025-11-15T10:00:00Z [INFO] Session started: backend-api
2025-11-15T11:30:00Z [INFO] Session paused: backend-api (idle timeout)
2025-11-15T14:00:00Z [WARN] Manual edit: session end time adjusted
2025-11-15T15:00:00Z [INFO] Database backup created
```

### Secure Defaults

**Default Configuration:**
- Encryption: Off (user must opt-in)
- Network access: None
- Auto-updates: Manual only
- Telemetry: Disabled
- Crash reports: Prompt user

**Principle of Least Privilege:**
- Minimal file system access
- No network access required
- No system-level permissions
- No access to other user data

---

## Logging & Debugging

### Log Levels

**Standard Levels:**
1. **ERROR**: Critical errors requiring immediate attention
2. **WARN**: Important warnings, degraded functionality
3. **INFO**: Normal operational messages
4. **DEBUG**: Detailed diagnostic information
5. **TRACE**: Very verbose, internal state tracking

**Default**: INFO level

**Configuration:**
```toml
[logging]
level = "info"  # error | warn | info | debug | trace
log_to_file = true
log_to_stderr = false
max_log_size_mb = 10
max_log_files = 5
```

### Log Files

**Location:**
```
~/.timetrack/logs/
  daemon.log         # Main daemon log
  shell.log          # Shell hook activity
  ide.log            # IDE extension activity
  errors.log         # Errors only (rotated separately)
```

**Rotation:**
- **Size-based**: Max 10MB per file
- **Count-based**: Keep 5 files
- **Naming**: `daemon.log`, `daemon.log.1`, `daemon.log.2`, etc.

**Log Format:**
```
2025-11-15T10:00:23.456Z [INFO] timetrack::daemon: Daemon started (PID 12345)
2025-11-15T10:00:24.123Z [DEBUG] timetrack::ipc: Unix socket listening on ~/.timetrack/daemon.sock
2025-11-15T10:01:15.789Z [INFO] timetrack::tracker: Session started: backend-api (Terminal)
2025-11-15T10:31:20.456Z [WARN] timetrack::tracker: Idle timeout reached, pausing session
2025-11-15T11:00:00.123Z [ERROR] timetrack::db: Database write failed: disk full
```

### Debug Mode

**Enable Debug Logging:**
```bash
# Environment variable
export RUST_LOG=timetrack=debug
timetrack daemon start

# Or via config
timetrack config set logging.level debug
timetrack daemon restart
```

**Debug Output:**
```
[DEBUG] Received activity signal: {source: "shell", project: "/home/user/backend", timestamp: "2025-11-15T10:00:00Z"}
[DEBUG] Rate limiter check: shell_hook (5/10 requests in window)
[DEBUG] State transition: Idle → Active (project: backend-api)
[DEBUG] Database flush: 1 session updated
[DEBUG] Memory usage: 7.2 MB
```

### Diagnostic Commands

```bash
# View recent logs
timetrack logs
timetrack logs --tail 50
timetrack logs --follow

# Filter logs
timetrack logs --level error
timetrack logs --level warn
timetrack logs --grep "session"

# Daemon statistics
timetrack daemon stats

# Database integrity check
timetrack database check

# Export diagnostic bundle
timetrack diagnostic-export --output ~/timetrack-diag.tar.gz
# Includes: logs, config, database stats (no session data)
```

### Structured Logging

**JSON Format (Optional):**
```toml
[logging]
format = "json"  # or "text" (default)
```

```json
{
  "timestamp": "2025-11-15T10:00:23.456Z",
  "level": "INFO",
  "target": "timetrack::daemon",
  "message": "Daemon started",
  "fields": {
    "pid": 12345,
    "version": "1.0.0"
  }
}
```

### Performance Logging

**Slow Operation Warnings:**
```rust
fn log_slow_operation<F>(name: &str, threshold: Duration, f: F) 
where F: FnOnce() {
    let start = Instant::now();
    f();
    let elapsed = start.elapsed();
    
    if elapsed > threshold {
        log_warn!("Slow operation: {} took {:?} (threshold: {:?})", 
                  name, elapsed, threshold);
    }
}
```

**Example:**
```
[WARN] Slow operation: database_query took 523ms (threshold: 100ms)
[WARN] Slow operation: session_flush took 1.2s (threshold: 500ms)
```

### Error Reporting

**User-Facing Errors:**
```bash
$ timetrack start nonexistent-project
Error: Project not found: nonexistent-project

Did you mean:
  - backend-project
  - frontend-project

Run 'timetrack init' in the project directory to start tracking it.
```

**Error Logging:**
```rust
fn log_error(err: &Error) {
    log_error!("Operation failed: {}", err);
    log_debug!("Error details: {:?}", err);
    
    // User-friendly message
    eprintln!("Error: {}", err);
    eprintln!("
For more details, run: timetrack logs --level error");
}
```

### Privacy in Logs

**Redaction:**
- Full paths redacted to last component: `/home/user/secret-project` → `***/ secret-project`
- Project names preserved (needed for debugging)
- No file contents or command output logged

**Sensitive Data Handling:**
```rust
fn log_activity_signal(signal: &ActivitySignal) {
    log_debug!("Activity signal: source={}, project={}", 
               signal.source,
               redact_path(&signal.project_path));
}

fn redact_path(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| format!("***/{}",n))
        .unwrap_or_else(|| "***".to_string())
}
```

---

## Testing Strategy

### Unit Testing

**Coverage Target**: 80%+ for core logic

**Key Areas:**
- Session management logic
- Time calculations and timezone handling
- Rate limiting and debouncing
- Path canonicalization
- Database queries
- State machine transitions

**Example:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_duration_calculation() {
        let start = Utc::now();
        let end = start + Duration::from_secs(3600);
        let session = Session::new(project_id, start);
        session.end(end);
        
        assert_eq!(session.active_duration(), Duration::from_secs(3600));
    }
    
    #[test]
    fn test_idle_timeout() {
        let mut tracker = Tracker::new();
        tracker.start_session(project_id);
        
        // Simulate 30 min of inactivity
        advance_time(Duration::from_secs(1800));
        tracker.check_idle_timeout();
        
        assert!(tracker.is_paused());
    }
    
    #[test]
    fn test_path_canonicalization() {
        assert_eq!(
            canonicalize_path("~/project").unwrap(),
            PathBuf::from("/home/user/project")
        );
    }
}
```

### Integration Testing

**Test Scenarios:**
1. **Shell Integration**
   - Shell hook sends signal → Daemon receives → Session starts
   - Multiple `cd` commands → Correct project switches
   - Socket communication end-to-end

2. **IDE Integration**
   - VS Code extension sends signals
   - Debouncing works correctly
   - Workspace switches tracked

3. **Database Operations**
   - Session CRUD operations
   - Query performance under load
   - Crash recovery scenarios

4. **IPC Communication**
   - Unix socket reliability
   - Rate limiting enforcement
   - Connection handling

**Example:**
```rust
#[tokio::test]
async fn test_end_to_end_tracking() {
    // Start daemon
    let daemon = TestDaemon::start().await;
    
    // Send shell activity signal
    let signal = ActivitySignal {
        source: Source::Shell,
        project_path: "/test/project".into(),
        timestamp: Utc::now(),
    };
    daemon.send_signal(signal).await.unwrap();
    
    // Verify session started
    let status = daemon.get_status().await.unwrap();
    assert_eq!(status.active_project, Some("project".to_string()));
    
    // Cleanup
    daemon.stop().await;
}
```

### System Testing

**Full System Tests:**
- Install → Configure → Track → Report cycle
- Multi-hour sessions with sleep/wake
- Rapid project switching
- Resource limits enforcement
- Error recovery

**Test Environments:**
- Ubuntu 24.04 LTS
- macOS 14 (Sonoma)
- Windows 11
- WSL2 on Windows

### Performance Testing

**Benchmarks:**
```rust
#[bench]
fn bench_activity_signal_processing(b: &mut Bencher) {
    let daemon = TestDaemon::new();
    let signal = ActivitySignal::test_signal();
    
    b.iter(|| {
        daemon.process_signal(&signal);
    });
}

// Target: <1ms per signal

#[bench]
fn bench_report_generation(b: &mut Bencher) {
    let db = setup_test_db_with_100_projects();
    
    b.iter(|| {
        generate_weekly_report(&db);
    });
}

// Target: <100ms for weekly report
```

**Load Testing:**
- 1000 projects in database
- 10,000 sessions
- 100 signals/second for 1 minute
- Verify: Memory usage, CPU usage, response time

### Time-Based Testing

**Mock Time:**
```rust
// Use mock time for testing idle timeout
struct MockClock {
    current_time: AtomicU64,
}

impl MockClock {
    fn advance(&self, duration: Duration) {
        self.current_time.fetch_add(
            duration.as_secs(),
            Ordering::SeqCst
        );
    }
}

#[test]
fn test_30min_idle_timeout_with_mock_clock() {
    let clock = MockClock::new();
    let tracker = Tracker::with_clock(clock.clone());
    
    tracker.start_session(project_id);
    clock.advance(Duration::from_secs(1800));
    tracker.check_idle();
    
    assert!(tracker.is_paused());
}
```

### Regression Testing

**Regression Suite:**
- Run full test suite before each release
- Automated CI/CD pipeline
- Test against previous versions' databases (migration testing)

**Example CI Configuration:**
```yaml
# .github/workflows/test.yml
name: Test

on: [push, pull_request]

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - run: cargo test --all-features
      - run: cargo test --release  # Test optimized build
```

### Manual Testing Checklist

**Before Release:**
- [ ] Install on fresh system
- [ ] Auto-start on boot works
- [ ] Shell integration (bash, zsh, PowerShell)
- [ ] VS Code extension installation
- [ ] Track session for 30+ minutes
- [ ] Idle timeout triggers correctly
- [ ] System sleep handled properly
- [ ] Reports generate correctly
- [ ] Manual time edit works
- [ ] Project linking works
- [ ] Tag system works
- [ ] Uninstall removes all components
- [ ] Database backup/restore works
- [ ] Resource usage within limits

### Test Data Generation

```rust
// Generate realistic test data
fn generate_test_sessions(count: usize) -> Vec<Session> {
    let mut rng = rand::thread_rng();
    let projects = vec!["frontend", "backend", "docs", "mobile"];
    
    (0..count)
        .map(|i| {
            let project = projects[rng.gen_range(0..projects.len())];
            let start = Utc::now() - Duration::from_secs(rng.gen_range(0..86400 * 30));
            let duration = Duration::from_secs(rng.gen_range(600..14400));  // 10min - 4hr
            
            Session {
                project_id: project.into(),
                start_time: start,
                end_time: Some(start + duration),
                context: Context::random(),
                ..Default::default()
            }
        })
        .collect()
}
```

---

## Database Migrations

### Schema Versioning

**Version Tracking:**
```sql
CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    description TEXT
);

INSERT INTO schema_version (version, description) 
VALUES (1, 'Initial schema');
```

**Current Schema Version**: 1

### Migration System

**Migration Files:**
```
migrations/
  001_initial_schema.sql
  002_add_tags.sql
  003_add_recovery_status.sql
  ...
```

**Migration Runner:**
```rust
struct Migration {
    version: i32,
    description: String,
    up_sql: String,
    down_sql: String,
}

fn run_migrations(conn: &Connection) -> Result<()> {
    let current_version = get_schema_version(conn)?;
    let migrations = load_migrations()?;
    
    for migration in migrations {
        if migration.version > current_version {
            apply_migration(conn, &migration)?;
        }
    }
    
    Ok(())
}

fn apply_migration(conn: &Connection, migration: &Migration) -> Result<()> {
    conn.execute_batch("BEGIN TRANSACTION;")?;
    
    match conn.execute_batch(&migration.up_sql) {
        Ok(_) => {
            conn.execute(
                "INSERT INTO schema_version (version, description) VALUES (?1, ?2)",
                params![migration.version, migration.description],
            )?;
            conn.execute_batch("COMMIT;")?;
            log_info!("Applied migration {}: {}", migration.version, migration.description);
            Ok(())
        }
        Err(e) => {
            conn.execute_batch("ROLLBACK;")?;
            Err(e.into())
        }
    }
}
```

### Backward Compatibility

**Version Support:**
- **Current version**: Always supported
- **N-1 version**: Supported (auto-upgrade on startup)
- **N-2 version**: Supported with warning
- **Older**: Prompt user to upgrade

**Example:**
```bash
$ timetrack daemon start
Warning: Database schema is 2 versions old (v1, current: v3)
Automatic migration will be performed.

Backing up database to: ~/.timetrack/backups/pre-migration-v3.db
Running migrations...
  ✓ Migration 2: Add tags support
  ✓ Migration 3: Add recovery status
Database migrated successfully to v3
```

### Rollback Strategy

**Automatic Backup Before Migration:**
```rust
fn backup_before_migration(db_path: &Path) -> Result<PathBuf> {
    let backup_path = get_backup_path("pre-migration");
    std::fs::copy(db_path, &backup_path)?;
    log_info!("Created pre-migration backup: {:?}", backup_path);
    Ok(backup_path)
}
```

**Manual Rollback:**
```bash
$ timetrack database rollback --to-version 2
Warning: Rolling back to version 2 will lose data added in version 3
Continue? (y/N): y

Rolling back migration 3...
  ✓ Executed down migration
  ✓ Schema version: 2
Database rolled back successfully
```

### Migration Testing

**Test Each Migration:**
```rust
#[test]
fn test_migration_002_add_tags() {
    let conn = create_test_db_v1();
    
    // Apply migration
    apply_migration(&conn, &migrations::M002_ADD_TAGS).unwrap();
    
    // Verify schema changes
    assert!(table_exists(&conn, "tags"));
    assert!(table_exists(&conn, "project_tags"));
    
    // Test rollback
    rollback_migration(&conn, 2).unwrap();
    assert!(!table_exists(&conn, "tags"));
}
```

---

## Versioning & Compatibility

### Semantic Versioning

**Version Format**: MAJOR.MINOR.PATCH

- **MAJOR**: Breaking changes (incompatible API/database changes)
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

**Examples:**
- `1.0.0` → `1.1.0`: Added tag system (backward compatible)
- `1.1.0` → `1.1.1`: Fixed idle timeout bug
- `1.9.0` → `2.0.0`: Complete rewrite (breaking changes)

### Compatibility Policy

**Database Compatibility:**
- Support migrations from previous MAJOR version
- Example: v2.x can upgrade v1.x databases
- Downgrades not supported (use backups)

**Config Compatibility:**
- Backward compatible within MAJOR version
- New options added with defaults
- Deprecated options warned for 1 MINOR version, removed in next

**CLI Compatibility:**
- Commands stable within MAJOR version
- New commands/flags added freely
- Deprecated commands warned, removed in next MAJOR

**API Stability (IPC Protocol):**
- Protocol version embedded in messages
- Backward compatible for 2 MINOR versions
- Example: v1.3 daemon works with v1.1 shell hooks

### Version Checking

**Daemon Version Check:**
```rust
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn check_compatibility(client_version: &str) -> Result<bool> {
    let daemon_version = Version::parse(VERSION)?;
    let client_version = Version::parse(client_version)?;
    
    // Same major version required
    if daemon_version.major != client_version.major {
        return Ok(false);
    }
    
    // Within 2 minor versions
    let minor_diff = (daemon_version.minor as i32 - client_version.minor as i32).abs();
    Ok(minor_diff <= 2)
}
```

**CLI Version Mismatch:**
```bash
$ timetrack status
Warning: CLI version (1.5.0) differs from daemon (1.3.0)
Some features may not work correctly. Consider upgrading.

Currently tracking: backend-api (1h 23m)
```

### Deprecation Policy

**Deprecation Process:**
1. **Announce**: Mark feature as deprecated in docs
2. **Warn**: Log warnings when deprecated feature used (1 minor version)
3. **Remove**: Remove in next major version

**Example:**
```rust
// Version 1.3.0 - Deprecation announced
#[deprecated(since = "1.3.0", note = "Use `timetrack projects` instead")]
pub fn list_projects_old() { }

// Version 1.4.0 - Warning logged
if using_old_command {
    log_warn!("'timetrack list' is deprecated, use 'timetrack projects'");
}

// Version 2.0.0 - Removed
// Old command no longer exists
```

### Changelog

**Maintain CHANGELOG.md:**
```markdown
# Changelog

All notable changes to this project will be documented in this file.

## [1.1.0] - 2025-12-01

### Added
- Tag system for project organization
- Export to CSV format
- Project linking for multi-project tracking

### Changed
- Improved idle detection accuracy
- Database query performance optimizations

### Fixed
- Bug in timezone handling during DST transitions
- Memory leak in long-running sessions

### Deprecated
- `timetrack list` command (use `timetrack projects`)

## [1.0.0] - 2025-11-15

Initial release
```

---

## Performance Testing Methodology

### Benchmark Suite

**Core Benchmarks:**
```rust
// Criterion-based benchmarks
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_signal_processing(c: &mut Criterion) {
    let daemon = TestDaemon::new();
    
    c.bench_function("process_activity_signal", |b| {
        b.iter(|| {
            daemon.process_signal(black_box(test_signal()))
        });
    });
}

fn benchmark_report_generation(c: &mut Criterion) {
    let db = setup_db_with_1000_sessions();
    
    c.bench_function("generate_weekly_report", |b| {
        b.iter(|| {
            generate_report(black_box(&db), ReportType::Weekly)
        });
    });
}

criterion_group!(benches, benchmark_signal_processing, benchmark_report_generation);
criterion_main!(benches);
```

**Targets:**
| Operation | Target | Measured |
|-----------|--------|----------|
| Activity signal processing | <1ms | |
| Project switch | <5ms | |
| Database write (single session) | <10ms | |
| Weekly report generation | <100ms | |
| Daemon startup | <100ms | |
| Status query | <5ms | |

### Load Testing

**Stress Test Scenarios:**

1. **High Signal Rate**
```rust
#[tokio::test]
async fn stress_test_signal_flood() {
    let daemon = TestDaemon::start().await;
    
    // Send 1000 signals in 10 seconds (100/sec)
    for _ in 0..1000 {
        daemon.send_signal(random_signal()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    
    // Verify daemon still responsive
    let status = daemon.get_status().await.unwrap();
    assert!(status.is_ok());
    
    // Check resource usage
    let stats = daemon.get_stats().await.unwrap();
    assert!(stats.memory_mb < 15);  // Allow some overhead
    assert!(stats.cpu_percent < 5.0);
}
```

2. **Large Database**
```rust
#[test]
fn performance_test_large_database() {
    let db = setup_db_with_10000_sessions_across_1000_projects();
    
    // Query performance should remain acceptable
    let start = Instant::now();
    let report = generate_monthly_report(&db);
    let elapsed = start.elapsed();
    
    assert!(elapsed < Duration::from_millis(500));
    assert!(report.projects.len() > 0);
}
```

3. **Memory Leak Detection**
```rust
#[test]
fn memory_leak_test() {
    let daemon = TestDaemon::new();
    
    let initial_memory = daemon.memory_usage();
    
    // Run for 10000 iterations
    for i in 0..10000 {
        daemon.process_signal(test_signal());
        
        if i % 1000 == 0 {
            let current_memory = daemon.memory_usage();
            let growth = current_memory - initial_memory;
            
            // Memory growth should be bounded
            assert!(growth < 5 * 1024 * 1024);  // <5MB growth
        }
    }
}
```

### Profiling

**CPU Profiling:**
```bash
# Using perf on Linux
cargo build --release
perf record --call-graph=dwarf ./target/release/timetrack daemon start
# Let it run for a while
perf report

# Using Instruments on macOS
cargo build --release
instruments -t "Time Profiler" ./target/release/timetrack daemon start
```

**Memory Profiling:**
```bash
# Using valgrind
cargo build
valgrind --tool=massif --massif-out-file=massif.out ./target/debug/timetrack daemon start
ms_print massif.out

# Using heaptrack on Linux
heaptrack ./target/release/timetrack daemon start
heaptrack_gui heaptrack.timetrack.*.gz
```

### Continuous Performance Monitoring

**CI/CD Integration:**
```yaml
# .github/workflows/benchmark.yml
name: Benchmark

on:
  push:
    branches: [main]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Run benchmarks
        run: cargo bench --bench main
      
      - name: Store benchmark result
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: target/criterion/output.json
          
      - name: Alert on regression
        if: github.event_name == 'push'
        run: |
          # Compare with previous benchmark
          # Alert if >10% regression
```

### Real-World Performance Testing

**Test Scenarios:**
1. **Normal Use**: 8-hour workday, 3-5 project switches
2. **Heavy Use**: 12-hour day, 10+ projects, frequent switches
3. **Light Use**: Occasional tracking, 1-2 projects
4. **Weekend Use**: Long sessions (4+ hours)

**Monitoring:**
```bash
# Run performance monitor during testing
while true; do
  echo "$(date): $(timetrack daemon stats | grep -E 'Memory|CPU')" >> perf-log.txt
  sleep 60
done

# Analysis
grep "Memory Usage" perf-log.txt | awk '{print $3}' | sort -n | tail -1  # Peak memory
grep "CPU Usage" perf-log.txt | awk '{print $3}' | awk '{sum+=$1; count++} END {print sum/count}'  # Avg CPU
```

---

## Time Entry Editing

### Manual Time Adjustments

**Use Cases:**
- Forgot to stop tracking
- Need to adjust start/end times
- Incorrectly tracked project
- Merge or split sessions

**CLI Commands:**
```bash
# List recent sessions
$ timetrack sessions --recent 10
ID   Project      Start             End               Duration
42   backend-api  Nov 15 09:00 AM   Nov 15 11:30 AM   2h 30m
41   frontend     Nov 15 07:00 AM   Nov 15 09:00 AM   2h 00m

# Edit session end time
$ timetrack session edit 42 --end "Nov 15 10:30 AM"
Updated session 42: backend-api
  Old duration: 2h 30m
  New duration: 1h 30m

# Edit session start time
$ timetrack session edit 42 --start "Nov 15 09:30 AM"

# Change project
$ timetrack session edit 42 --project frontend-app

# Delete session
$ timetrack session delete 42
Deleted session 42 (backend-api, 2h 30m)

# Split session
$ timetrack session split 42 --at "Nov 15 10:00 AM"
Split session 42 into:
  Session 42: backend-api (9:00 AM - 10:00 AM, 1h)
  Session 43: backend-api (10:00 AM - 11:30 AM, 1h 30m)

# Merge sessions
$ timetrack session merge 42 43
Merged sessions 42 and 43:
  Result: Session 42 (backend-api, 9:00 AM - 11:30 AM, 2h 30m)
```

### Interactive Editing

```bash
$ timetrack session edit 42 --interactive
Session 42: backend-api
  Start:    Nov 15 09:00 AM
  End:      Nov 15 11:30 AM
  Duration: 2h 30m
  Context:  Terminal

What would you like to edit?
1) Start time
2) End time
3) Project
4) Delete session
5) Cancel

Choice: 2

Enter new end time (or 'now'): 10:30 AM

Updated session:
  Start:    Nov 15 09:00 AM
  End:      Nov 15 10:30 AM
  Duration: 1h 30m
```

### Audit Trail for Edits

**Track Modifications:**
```sql
CREATE TABLE session_edits (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    edited_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    field_changed TEXT NOT NULL,  -- 'start_time', 'end_time', 'project_id'
    old_value TEXT,
    new_value TEXT,
    reason TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);
```

**View Edit History:**
```bash
$ timetrack session history 42
Session 42 edit history:
  Nov 15 11:45 AM - End time changed: 11:30 AM → 10:30 AM
  Nov 15 12:00 PM - Project changed: backend-api → frontend-app
```

### Validation Rules

**Prevent Invalid Edits:**
- Start time must be before end time
- No overlapping sessions for same project
- Cannot set future times
- Duration must be positive

```rust
fn validate_session_edit(edit: &SessionEdit) -> Result<()> {
    if let (Some(start), Some(end)) = (edit.start_time, edit.end_time) {
        if end <= start {
            return Err(Error::InvalidEdit("End time must be after start time"));
        }
    }
    
    if let Some(end) = edit.end_time {
        if end > Utc::now() {
            return Err(Error::InvalidEdit("Cannot set future end time"));
        }
    }
    
    // Check for overlaps
    if has_overlapping_session(edit)? {
        return Err(Error::InvalidEdit("Edit would create overlapping sessions"));
    }
    
    Ok(())
}
```

### Bulk Edits

```bash
# Edit all sessions from a specific day
$ timetrack session bulk-edit --date 2025-11-14 --adjust-end -30m
Found 5 sessions on Nov 14, 2025
Adjust all end times by -30 minutes? (y/N): y

Updated 5 sessions:
  backend-api: 2h 30m → 2h 00m
  frontend: 1h 45m → 1h 15m
  ...
```

---

## Project Path Updates

### Handling Moved/Renamed Projects

**Scenario**: User moves project directory

```bash
# Old path: /home/user/projects/backend
# New path: /home/user/work/backend-api

# Option 1: Automatic detection on next access
cd /home/user/work/backend-api
# Daemon detects .git folder matches existing project
# Prompts user to update path

# Option 2: Manual update
$ timetrack project update-path backend --new-path /home/user/work/backend-api
Updated project path:
  Old: /home/user/projects/backend
  New: /home/user/work/backend-api
All historical sessions preserved.
```

### Path Update Strategy

**Detection:**
```rust
fn detect_project_move(new_path: &Path) -> Option<ProjectId> {
    // Check if .git folder has same content hash
    let git_dir = new_path.join(".git");
    if !git_dir.exists() {
        return None;
    }
    
    let git_hash = hash_git_metadata(&git_dir);
    
    // Look for existing project with same git hash
    db.find_project_by_git_hash(&git_hash)
}

fn hash_git_metadata(git_dir: &Path) -> String {
    // Hash .git/config and .git/HEAD to identify project
    let config = std::fs::read(git_dir.join("config")).ok()?;
    let head = std::fs::read(git_dir.join("HEAD")).ok()?;
    
    let mut hasher = Sha256::new();
    hasher.update(&config);
    hasher.update(&head);
    format!("{:x}", hasher.finalize())
}
```

**Automatic Update Prompt:**
```bash
$ cd /home/user/work/backend-api
Detected existing project at new location:
  Project: backend-api
  Old path: /home/user/projects/backend
  New path: /home/user/work/backend-api

Update project path? (Y/n): y
Project path updated successfully.
```

### Bulk Path Updates

```bash
# User moved all projects to new directory
$ timetrack project bulk-update-path     --old-base /home/user/projects     --new-base /home/user/work

Found 5 projects under /home/user/projects:
  backend-api
  frontend-app
  mobile-app
  docs
  scripts

Update all paths? (y/N): y

Updated 5 projects:
  ✓ backend-api: /home/user/work/backend-api
  ✓ frontend-app: /home/user/work/frontend-app
  ...
```

### Orphaned Projects

**Handle Deleted Projects:**
```bash
$ timetrack projects --include-orphaned
Active Projects:
  backend-api     Last active: 2 hours ago
  frontend-app    Last active: 1 day ago

Orphaned Projects (path no longer exists):
  old-project     Last active: 3 months ago
                  Original path: /home/user/deleted/old-project
                  
Options for orphaned projects:
  1. Archive (keeps data, hides from reports)
  2. Delete (removes all data)
  3. Update path (if moved)

$ timetrack project archive old-project
Archived project: old-project
Data preserved for historical reports.
```

---

## User Feedback Mechanisms

### Notifications

**Notification Types:**

1. **Desktop Notifications** (Optional, v2.0)
```rust
// Using notify-rust crate
fn send_notification(title: &str, body: &str) {
    Notification::new()
        .summary(title)
        .body(body)
        .icon("timetrack")
        .timeout(Timeout::Milliseconds(5000))
        .show()
        .ok();
}

// Examples:
// "Session Auto-Paused" - "No activity for 30 minutes"
// "Long Session Detected" - "You've been working for 8 hours. Take a break!"
```

2. **CLI Status Messages**
```bash
$ timetrack status
Currently tracking: backend-api (Terminal) - 4h 23m
💡 Tip: You've been working for over 4 hours. Consider taking a break.
```

3. **Progress Indicators**
```bash
$ timetrack report --month --verbose
Generating monthly report...
[====================] 100% Analyzing 234 sessions
Report generated successfully!
```

### User Prompts

**Interactive Confirmations:**
```bash
$ timetrack project delete backend-api
⚠️  Warning: This will delete all time tracking data for backend-api
Total sessions: 156
Total time: 234h 56m

Are you sure? Type 'backend-api' to confirm: backend-api
Deleted project: backend-api
```

**Smart Suggestions:**
```bash
$ timetrack start backe
Project 'backe' not found.

Did you mean:
  - backend-api
  - backend-mobile

Or create new project: timetrack init --name backe
```

### Status Indicators

**Daemon Health:**
```bash
$ timetrack status --detailed
TimeTrack Status
─────────────────────────────────────
Daemon:              🟢 Running (PID 12345, uptime: 3d 14h)
Active Session:      🟢 backend-api (Terminal, 1h 23m)
Last Activity:       🟢 2 minutes ago
Resource Usage:      🟢 Memory: 7.2 MB, CPU: 0.3%
Database:            🟢 42 MB, last backup: 6 hours ago

Warnings:            None
Next auto-backup:    In 18 hours
```

**Color Coding:**
- 🟢 Green: Normal, healthy
- 🟡 Yellow: Warning, attention needed
- 🔴 Red: Error, action required

### Feedback Collection

**Optional Telemetry (Explicitly Opt-In):**
```toml
[feedback]
enable_anonymous_usage_stats = false  # Default: false
enable_crash_reports = false          # Default: false
```

```bash
# First run prompt
TimeTrack First Run Setup
─────────────────────────────────────
Help improve TimeTrack by sharing anonymous usage statistics?
  - Number of projects tracked
  - Average session length
  - Feature usage (no project names or paths)

Your privacy is important. This is completely optional.
Enable anonymous statistics? (y/N): n

Enable automatic crash report submission? (y/N): n

Settings saved. You can change these anytime with:
  timetrack config set feedback.enable_anonymous_usage_stats true
```

### In-App Tips

**Context-Sensitive Tips:**
```bash
$ timetrack report
Project Time Report (Nov 11 - Nov 15, 2025)
─────────────────────────────────────────────
backend-api          12h 34m
frontend-app          8h 15m
─────────────────────────────────────────────
Total:               20h 49m

💡 Tip: Use tags to organize your projects:
   timetrack tag add work backend-api frontend-app
```

**Tip Rotation:**
- Show different tip each time
- Mark tips as seen (don't repeat frequently)
- Disable with `timetrack config set tips.enabled false`

### Error Guidance

**Helpful Error Messages:**
```bash
$ timetrack daemon start
Error: Permission denied: ~/.timetrack/daemon.sock

Possible solutions:
  1. Check file permissions: ls -la ~/.timetrack/
  2. Ensure you're not running as root
  3. Check if another user is running the daemon

For more help, run: timetrack troubleshoot
```

### Success Feedback

**Positive Reinforcement:**
```bash
$ timetrack report --export csv --output ~/report.csv
✓ Report exported successfully!
  Location: ~/report.csv
  Sessions: 234
  Total time: 87h 23m

You've been productive this month! 🎉
```

### Survey/Feedback System (Optional)

```bash
$ timetrack feedback
Share your thoughts about TimeTrack

Rate your experience (1-5): 5
What do you like most? (optional): Simple and reliable
What could be improved? (optional): Would love JetBrains IDE support

Thank you for your feedback!
```

### Help System

```bash
$ timetrack help
TimeTrack - Automatic Project Time Tracking

Common commands:
  timetrack status              Show current tracking status
  timetrack report              Generate time report
  timetrack projects            List all projects
  
For detailed help on any command:
  timetrack help <command>

Need more help?
  - Documentation: https://timetrack.dev/docs
  - Issues: https://github.com/yourusername/timetrack/issues
  - Community: https://github.com/yourusername/timetrack/discussions
```

---

## Uninstallation Guide

### Complete Removal

**Automated Uninstall Script:**
```bash
$ timetrack uninstall
TimeTrack Uninstallation
─────────────────────────────────────
This will remove:
  ✓ Daemon and CLI binary
  ✓ Shell integration
  ✓ VS Code extension
  ✓ Configuration files
  ✓ All tracking data

⚠️  Warning: This action cannot be undone!

What would you like to do with your data?
1) Export data before removal
2) Keep data (remove only application)
3) Delete everything

Choice: 1

Exporting data...
  ✓ Exported to ~/timetrack-backup-2025-11-15.csv
  ✓ Backup saved

Removing TimeTrack components...
  ✓ Stopped daemon
  ✓ Removed binary from /usr/local/bin/timetrack
  ✓ Removed shell hooks from ~/.bashrc
  ✓ Uninstalled VS Code extension
  ✓ Removed ~/.timetrack/ directory
  ✓ Removed ~/.config/timetrack/ directory

TimeTrack has been completely uninstalled.
Thank you for using TimeTrack!
```

### Manual Uninstallation

**Step-by-Step:**

```bash
# 1. Stop daemon
timetrack daemon stop

# 2. Export data (optional)
timetrack report --export csv --output ~/timetrack-backup.csv

# 3. Remove binary
sudo rm /usr/local/bin/timetrack  # Linux/macOS
# Or on Windows:
# Remove-Item C:\Windows\System32	imetrack.exe

# 4. Remove shell integration
# Edit ~/.bashrc or ~/.zshrc, remove line:
# source ~/.timetrack/shell/hook.sh

# For PowerShell, edit $PROFILE, remove TimeTrack section

# 5. Uninstall VS Code extension
code --uninstall-extension timetrack

# 6. Remove data directories
rm -rf ~/.timetrack
rm -rf ~/.config/timetrack  # Linux/macOS
# Or on Windows:
# Remove-Item -Recurse $env:APPDATA	imetrack

# 7. Remove auto-start (if configured)
# Linux systemd:
systemctl --user disable timetrack
systemctl --user stop timetrack
rm ~/.config/systemd/user/timetrack.service

# macOS LaunchAgent:
launchctl unload ~/Library/LaunchAgents/dev.timetrack.daemon.plist
rm ~/Library/LaunchAgents/dev.timetrack.daemon.plist

# Windows:
# Remove from Task Scheduler or Startup folder
```

### Partial Uninstallation

**Keep Data, Remove Application:**
```bash
$ timetrack uninstall --keep-data
Removing TimeTrack application...
  ✓ Stopped daemon
  ✓ Removed binary
  ✓ Removed shell hooks
  ✓ Uninstalled VS Code extension

Data preserved in:
  ~/.timetrack/data.db
  ~/.config/timetrack/config.toml

To reinstall later with existing data:
  Run standard installation, data will be detected automatically
```

### Verification

**Ensure Complete Removal:**
```bash
# Check for remaining processes
ps aux | grep timetrack
# Should return nothing

# Check for remaining files
find ~ -name "*timetrack*" 2>/dev/null

# Check shell configuration
grep -r "timetrack" ~/.bashrc ~/.zshrc ~/.config/fish/ 2>/dev/null

# Check systemd/launchd
systemctl --user list-units | grep timetrack  # Linux
launchctl list | grep timetrack  # macOS
```

### Data Recovery After Uninstall

**If Uninstalled But Want Data Back:**
```bash
# Data is in ~/.timetrack/data.db (if not deleted)
# Reinstall TimeTrack
curl -L https://timetrack.dev/install.sh | bash

# Daemon will automatically detect existing database
timetrack daemon start
# "Existing database found, loading historical data..."

timetrack report --month
# All historical data intact
```

---

## Implementation Plan

### ## Implementation Plan

### Phase 1: Core Daemon (Week 1-2)

**Goals:**
- Basic daemon with event loop
- IPC server (Unix socket)
- SQLite database setup
- State machine for single project tracking
- Basic CLI (start/stop/status)

**Deliverables:**
- `timetrack daemon start/stop`
- `timetrack start/stop/status`
- Manual time tracking works
- Database persistence

### Phase 2: Shell Integration (Week 2-3)

**Goals:**
- Bash/Zsh hooks
- PowerShell integration
- Auto-detection via `cd` command
- Project detection (.git, .timetrack)

**Deliverables:**
- Automatic tracking in terminal
- Shell hooks installation script
- Cross-platform support (Linux, macOS, Windows)

### Phase 3: IDE Integration (Week 3-4)

**Goals:**
- VS Code extension
- Workspace tracking
- File activity monitoring
- Debounced activity signals

**Deliverables:**
- Published VS Code extension
- Works with Cursor
- Auto-detection in IDE

### Phase 4: Advanced Features (Week 4-5)

**Goals:**
- Project linking
- Tag system
- Idle detection (30min timeout)
- Last-focused project logic

**Deliverables:**
- `timetrack link/unlink`
- `timetrack tag` commands
- Automatic idle pause/resume
- Multi-context tracking

### Phase 5: Reporting & Polish (Week 5-6)

**Goals:**
- Comprehensive reporting
- CSV export
- Date range queries
- Performance optimization
- Documentation

**Deliverables:**
- `timetrack report` with filters
- Export functionality
- User documentation
- Performance tuning

### Phase 6: Testing & Release (Week 6-7)

**Goals:**
- End-to-end testing
- Cross-platform validation
- Bug fixes
- Packaging (installers)

**Deliverables:**
- Test coverage >80%
- Installation packages
- GitHub release
- Usage documentation

---

## Performance Requirements

### Resource Constraints

| Metric | Target | Maximum |
|--------|--------|---------|
| RAM Usage | 5MB | 10MB |
| CPU (Idle) | 0% | 0.1% |
| CPU (Active) | 0.5% | 2% |
| Disk I/O | <1MB/day | 10MB/day |
| Database Size | <1MB/month | 5MB/month |
| Startup Time | 50ms | 100ms |

### Optimization Strategies

1. **Memory Management**
   - Keep only current session in memory
   - Lazy-load historical data
   - Periodic memory cleanup

2. **CPU Efficiency**
   - Event-driven architecture (no polling)
   - Sleep when inactive
   - Debounce rapid events

3. **Database Optimization**
   - Batch writes (flush every 5min or on state change)
   - Weekly aggregation of old sessions
   - Indexed queries
   - VACUUM on schedule

4. **Network/IPC**
   - Unix sockets (faster than TCP)
   - Binary protocol option (instead of JSON)
   - Connection pooling

### Benchmark Targets

```bash
# Startup time
time timetrack status  # <100ms

# Report generation
time timetrack report --month  # <500ms

# Memory footprint
ps aux | grep timetrack  # <10MB RSS

# Database size after 1 year
du -h ~/.timetrack/data.db  # <50MB
```

---

## Configuration

### Config File Location

- **Linux/macOS:** `~/.config/timetrack/config.toml`
- **Windows:** `%APPDATA%\timetrack\config.toml`

### Default Configuration

```toml
[daemon]
auto_start = true
socket_path = "~/.timetrack/daemon.sock"  # Unix
# pipe_name = "timetrack"  # Windows

[tracking]
idle_timeout = 1800  # 30 minutes in seconds
auto_detect_git = true
auto_detect_marker = true
marker_file = ".timetrack"

[storage]
database_path = "~/.timetrack/data.db"
backup_enabled = true
backup_interval = 86400  # Daily in seconds
max_backups = 7

[reporting]
default_format = "table"  # table | json | csv
time_format = "human"     # human | hours | seconds
week_start = "monday"     # monday | sunday

[performance]
max_memory_mb = 10
flush_interval = 300  # Flush to DB every 5 minutes
aggregate_after_days = 90  # Aggregate sessions older than 90 days

[ide]
vscode_enabled = true
debounce_interval = 30000  # 30 seconds

[shell]
bash_enabled = true
zsh_enabled = true
powershell_enabled = true
```

---

## Future Enhancements

### Version 2.0 Features

1. **Advanced Analytics**
   - Productivity insights (most productive hours)
   - Project velocity tracking
   - Burnout detection (excessive hours)
   - Weekly/monthly trends

2. **Integrations**
   - Export to Toggl, Harvest, Clockify
   - Calendar integration (Google Calendar, Outlook)
   - Invoice generation from time data
   - JIRA/Linear ticket time attribution

3. **Team Features**
   - Shared project tracking
   - Team analytics
   - Time approval workflows
   - Billing/invoicing

4. **Enhanced Detection**
   - Machine learning for better idle detection
   - Meeting detection (don't track during meetings)
   - Break reminders
   - Focus time protection

5. **UI/UX**
   - Web dashboard
   - Native desktop app (Tauri)
   - Mobile companion app
   - System tray icon with quick stats

6. **Improved Reporting**
   - PDF report generation
   - Charts and graphs
   - Comparison reports (this week vs last week)
   - Customizable report templates

### Version 3.0 Ideas

- AI-powered task categorization
- Automatic context switching detection
- Integration with project management tools
- Voice commands for time tracking
- Pomodoro timer integration
- Distraction tracking (detect when switching projects rapidly)

---

## Installation Guide

### Prerequisites

- Rust 1.70+ (for building from source)
- VS Code or Cursor (for IDE integration)
- Bash/Zsh (Linux/macOS) or PowerShell (Windows)

### Installation Steps

#### 1. Install CLI & Daemon

**From Pre-built Binary:**
```bash
# Linux/macOS
curl -L https://github.com/yourusername/timetrack/releases/latest/download/timetrack-linux -o timetrack
chmod +x timetrack
sudo mv timetrack /usr/local/bin/

# Windows (PowerShell)
Invoke-WebRequest -Uri https://github.com/yourusername/timetrack/releases/latest/download/timetrack-windows.exe -OutFile timetrack.exe
Move-Item timetrack.exe C:\Windows\System32\
```

**From Source:**
```bash
git clone https://github.com/yourusername/timetrack.git
cd timetrack
cargo build --release
sudo cp target/release/timetrack /usr/local/bin/
```

#### 2. Initialize Daemon

```bash
# Create config directory
timetrack init-config

# Start daemon (will auto-start on boot)
timetrack daemon start

# Verify daemon is running
timetrack daemon status
```

#### 3. Install Shell Integration

```bash
# Automatic installation
timetrack install-shell

# Manual installation
echo 'source ~/.timetrack/shell/hook.sh' >> ~/.bashrc  # or ~/.zshrc
source ~/.bashrc
```

#### 4. Install VS Code Extension

**From Marketplace:**
1. Open VS Code/Cursor
2. Go to Extensions (Ctrl+Shift+X)
3. Search for "TimeTrack"
4. Click Install

**From VSIX:**
```bash
code --install-extension timetrack-0.1.0.vsix
```

#### 5. Verify Installation

```bash
# Check version
timetrack --version

# Start tracking in a project
cd ~/projects/myapp
timetrack status
# Should show: "Currently tracking: myapp (Terminal)"
```

---

## Troubleshooting

### Common Issues

**1. Daemon Not Starting**
```bash
# Check if already running
ps aux | grep timetrack

# Check logs
tail -f ~/.timetrack/logs/daemon.log

# Try restarting
timetrack daemon restart
```

**2. Shell Integration Not Working**
```bash
# Verify hook is loaded
type _timetrack_project_change

# Check socket connection
nc -U ~/.timetrack/daemon.sock < /dev/null
```

**3. VS Code Extension Not Tracking**
- Check extension is enabled in VS Code settings
- Verify daemon is running: `timetrack daemon status`
- Check VS Code Developer Tools for errors (Help → Toggle Developer Tools)

**4. Database Corruption**
```bash
# Restore from backup
cp ~/.timetrack/backups/data.db.backup ~/.timetrack/data.db

# Or rebuild database
timetrack database rebuild
```

---

## Security & Privacy

### Data Storage

- All data stored **locally only**
- No cloud synchronization by default
- No telemetry or analytics
- Database encrypted at rest (optional feature)

### What is Tracked

**Tracked:**
- Project paths
- Session start/end times
- Pause durations
- Context (terminal/IDE)

**NOT Tracked:**
- File contents
- Keystrokes
- Specific commands run
- URLs visited
- Screen contents
- Personal information

### Permissions Required

- **Filesystem:** Read project directories (to detect .git/.timetrack)
- **IPC:** Unix socket/named pipe for communication
- **System:** Auto-start on boot (optional)

---

## Contributing

### Development Setup

```bash
# Clone repository
git clone https://github.com/yourusername/timetrack.git
cd timetrack

# Install dependencies
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=debug cargo run -- daemon start

# Format code
cargo fmt

# Lint
cargo clippy
```

### Code Structure Standards

- Follow Rust best practices
- Write unit tests for all logic
- Document public APIs
- Use meaningful commit messages
- Keep functions focused and small

---

## License

MIT License - See LICENSE file for details

---

## Support

- **Documentation:** https://timetrack.dev/docs
- **Issues:** https://github.com/yourusername/timetrack/issues
- **Discussions:** https://github.com/yourusername/timetrack/discussions
- **Email:** support@timetrack.dev

---

## Acknowledgments

Built with:
- Rust programming language
- Tokio async runtime
- SQLite database
- VS Code Extension API

Inspired by: Toggl, WakaTime, ActivityWatch

---

**Document Version:** 1.0  
**Last Updated:** November 15, 2025  
**Author:** Project Specification  
**Status:** Ready for Implementation
