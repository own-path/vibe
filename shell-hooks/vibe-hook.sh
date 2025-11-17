#!/bin/bash
# Vibe shell hook for automatic project tracking
# This script should be sourced in your shell profile (.bashrc, .zshrc, etc.)

# Configuration
VIBE_BIN="${VIBE_BIN:-vibe}"
VIBE_DEBUG="${VIBE_DEBUG:-0}"
VIBE_HOOK_ENABLED="${VIBE_HOOK_ENABLED:-1}"

# Internal variables
_VIBE_LAST_DIR=""
_VIBE_CLIENT_PATH=""

# Debug logging function
_vibe_debug() {
    if [[ "$VIBE_DEBUG" == "1" ]]; then
        echo "[VIBE DEBUG] $*" >&2
    fi
}

# Function to detect if a directory is a project
_vibe_is_project() {
    local dir="$1"
    
    # Check for git repository
    if [[ -d "$dir/.git" ]]; then
        return 0
    fi
    
    # Check for .vibe marker
    if [[ -f "$dir/.vibe" ]]; then
        return 0
    fi
    
    # Check for common project files
    if [[ -f "$dir/package.json" ]] || \
       [[ -f "$dir/Cargo.toml" ]] || \
       [[ -f "$dir/pyproject.toml" ]] || \
       [[ -f "$dir/pom.xml" ]] || \
       [[ -f "$dir/Makefile" ]] || \
       [[ -f "$dir/CMakeLists.txt" ]] || \
       [[ -f "$dir/go.mod" ]] || \
       [[ -f "$dir/composer.json" ]]; then
        return 0
    fi
    
    return 1
}

# Function to find the project root
_vibe_find_project_root() {
    local dir="$1"
    local original_dir="$dir"
    
    # Walk up the directory tree
    while [[ "$dir" != "/" ]]; do
        if _vibe_is_project "$dir"; then
            echo "$dir"
            return 0
        fi
        dir=$(dirname "$dir")
    done
    
    # If no project found, return the original directory if it's a reasonable project
    if [[ -w "$original_dir" ]] && [[ ! "$original_dir" =~ ^/(usr|bin|lib|etc|var|tmp|proc|sys|dev) ]]; then
        echo "$original_dir"
        return 0
    fi
    
    return 1
}

# Function to send IPC message to vibe daemon
_vibe_send_ipc() {
    local socket_path="${HOME}/.vibe/daemon.sock"
    
    # Simple check if socket exists
    if [[ ! -S "$socket_path" ]]; then
        _vibe_debug "Daemon socket not found at $socket_path"
        return 1
    fi
    
    # Use the vibe CLI as IPC client
    if command -v "$VIBE_BIN" >/dev/null 2>&1; then
        "$VIBE_BIN" "$@" 2>/dev/null
    else
        _vibe_debug "vibe binary not found in PATH"
        return 1
    fi
}

# Function to handle directory change
_vibe_on_directory_change() {
    if [[ "$VIBE_HOOK_ENABLED" != "1" ]]; then
        return
    fi
    
    local current_dir="$PWD"
    
    # Skip if we're in the same directory
    if [[ "$current_dir" == "$_VIBE_LAST_DIR" ]]; then
        return
    fi
    
    _vibe_debug "Directory changed from '$_VIBE_LAST_DIR' to '$current_dir'"
    
    # Find project root
    local project_root
    if project_root=$(_vibe_find_project_root "$current_dir"); then
        _vibe_debug "Found project at: $project_root"
        
        # Send project entered signal via CLI
        _vibe_send_ipc session start --project "$project_root" --context terminal
        
        # Export for other tools
        export VIBE_CURRENT_PROJECT="$project_root"
    else
        _vibe_debug "No project found for: $current_dir"
        unset VIBE_CURRENT_PROJECT
    fi
    
    _VIBE_LAST_DIR="$current_dir"
}

# Function to handle shell exit
_vibe_on_shell_exit() {
    if [[ "$VIBE_HOOK_ENABLED" == "1" ]] && [[ -n "$VIBE_CURRENT_PROJECT" ]]; then
        _vibe_debug "Shell exiting, leaving project: $VIBE_CURRENT_PROJECT"
        _vibe_send_ipc session stop
    fi
}

# Bash-specific hooks
if [[ "$SHELL" =~ bash ]]; then
    # Override cd command
    cd() {
        builtin cd "$@"
        local result=$?
        _vibe_on_directory_change
        return $result
    }
    
    # Hook into prompt command
    if [[ -z "$PROMPT_COMMAND" ]]; then
        PROMPT_COMMAND="_vibe_on_directory_change"
    else
        PROMPT_COMMAND="$PROMPT_COMMAND; _vibe_on_directory_change"
    fi
    
    # Handle shell exit
    trap '_vibe_on_shell_exit' EXIT

# Zsh-specific hooks  
elif [[ "$SHELL" =~ zsh ]]; then
    # Use chpwd hook for directory changes
    chpwd_functions+=(_vibe_on_directory_change)
    
    # Override cd command as backup
    cd() {
        builtin cd "$@"
        local result=$?
        _vibe_on_directory_change
        return $result
    }
    
    # Handle shell exit
    zshexit() {
        _vibe_on_shell_exit
    }
fi

# Initialize for current directory
_vibe_on_directory_change

# Utility functions for manual control
vibe-enable() {
    export VIBE_HOOK_ENABLED=1
    echo "✅ Vibe automatic tracking enabled"
    _vibe_on_directory_change
}

vibe-disable() {
    export VIBE_HOOK_ENABLED=0
    echo "⏸️ Vibe automatic tracking disabled"
}

vibe-status() {
    if [[ "$VIBE_HOOK_ENABLED" == "1" ]]; then
        echo "✅ Vibe automatic tracking is enabled"
        if [[ -n "$VIBE_CURRENT_PROJECT" ]]; then
            echo "   📂 Current project: $VIBE_CURRENT_PROJECT"
        else
            echo "   💤 No active project"
        fi
    else
        echo "⏸️ Vibe automatic tracking is disabled"
    fi
    
    # Show daemon status
    if command -v "$VIBE_BIN" >/dev/null 2>&1; then
        echo ""
        "$VIBE_BIN" status
    fi
}

vibe-debug() {
    if [[ "$VIBE_DEBUG" == "1" ]]; then
        export VIBE_DEBUG=0
        echo "🔇 Vibe debug logging disabled"
    else
        export VIBE_DEBUG=1
        echo "🔊 Vibe debug logging enabled"
    fi
}

# Print installation message
if [[ "$VIBE_DEBUG" == "1" ]]; then
    echo "🔄 Vibe shell hook loaded (debug mode)"
fi