# Vibe shell hook for Fish shell
# Add this to your Fish config: ~/.config/fish/config.fish

# Configuration
set -gx VIBE_BIN (which vibe)
set -q VIBE_DEBUG; or set -gx VIBE_DEBUG 0
set -q VIBE_HOOK_ENABLED; or set -gx VIBE_HOOK_ENABLED 1

# Internal variables
set -g _VIBE_LAST_DIR ""

# Debug logging function
function _vibe_debug
    if test "$VIBE_DEBUG" = "1"
        echo "[VIBE DEBUG] $argv" >&2
    end
end

# Function to detect if a directory is a project
function _vibe_is_project
    set dir $argv[1]
    
    # Check for git repository
    if test -d "$dir/.git"
        return 0
    end
    
    # Check for .vibe marker
    if test -f "$dir/.vibe"
        return 0
    end
    
    # Check for common project files
    for file in package.json Cargo.toml pyproject.toml pom.xml Makefile CMakeLists.txt go.mod composer.json
        if test -f "$dir/$file"
            return 0
        end
    end
    
    return 1
end

# Function to find the project root
function _vibe_find_project_root
    set dir $argv[1]
    set original_dir $dir
    
    # Walk up the directory tree
    while test "$dir" != "/"
        if _vibe_is_project $dir
            echo $dir
            return 0
        end
        set dir (dirname $dir)
    end
    
    # If no project found, return the original directory if it's reasonable
    if test -w "$original_dir"; and not string match -qr '^/(usr|bin|lib|etc|var|tmp|proc|sys|dev)' "$original_dir"
        echo $original_dir
        return 0
    end
    
    return 1
end

# Function to send IPC message to vibe daemon
function _vibe_send_ipc
    set socket_path "$HOME/.vibe/daemon.sock"
    
    # Simple check if socket exists
    if not test -S "$socket_path"
        _vibe_debug "Daemon socket not found at $socket_path"
        return 1
    end
    
    # Use the vibe CLI as IPC client
    if test -n "$VIBE_BIN"; and test -x "$VIBE_BIN"
        $VIBE_BIN $argv 2>/dev/null
    else
        _vibe_debug "vibe binary not found"
        return 1
    end
end

# Function to handle directory change
function _vibe_on_directory_change
    if test "$VIBE_HOOK_ENABLED" != "1"
        return
    end
    
    set current_dir $PWD
    
    # Skip if we're in the same directory
    if test "$current_dir" = "$_VIBE_LAST_DIR"
        return
    end
    
    _vibe_debug "Directory changed from '$_VIBE_LAST_DIR' to '$current_dir'"
    
    # Find project root
    if set project_root (_vibe_find_project_root $current_dir)
        _vibe_debug "Found project at: $project_root"
        
        # Send project entered signal via CLI
        _vibe_send_ipc session start --project "$project_root" --context terminal
        
        # Export for other tools
        set -gx VIBE_CURRENT_PROJECT $project_root
    else
        _vibe_debug "No project found for: $current_dir"
        set -e VIBE_CURRENT_PROJECT
    end
    
    set -g _VIBE_LAST_DIR $current_dir
end

# Function to handle shell exit
function _vibe_on_shell_exit --on-event fish_exit
    if test "$VIBE_HOOK_ENABLED" = "1"; and test -n "$VIBE_CURRENT_PROJECT"
        _vibe_debug "Shell exiting, leaving project: $VIBE_CURRENT_PROJECT"
        _vibe_send_ipc session stop
    end
end

# Hook into directory changes
function __vibe_pwd_handler --on-variable PWD
    _vibe_on_directory_change
end

# Initialize for current directory
_vibe_on_directory_change

# Utility functions for manual control
function vibe-enable
    set -gx VIBE_HOOK_ENABLED 1
    echo "✅ Vibe automatic tracking enabled"
    _vibe_on_directory_change
end

function vibe-disable
    set -gx VIBE_HOOK_ENABLED 0
    echo "⏸️ Vibe automatic tracking disabled"
end

function vibe-status
    if test "$VIBE_HOOK_ENABLED" = "1"
        echo "✅ Vibe automatic tracking is enabled"
        if test -n "$VIBE_CURRENT_PROJECT"
            echo "   📂 Current project: $VIBE_CURRENT_PROJECT"
        else
            echo "   💤 No active project"
        end
    else
        echo "⏸️ Vibe automatic tracking is disabled"
    end
    
    # Show daemon status
    if test -n "$VIBE_BIN"; and test -x "$VIBE_BIN"
        echo ""
        $VIBE_BIN status
    end
end

function vibe-debug
    if test "$VIBE_DEBUG" = "1"
        set -gx VIBE_DEBUG 0
        echo "🔇 Vibe debug logging disabled"
    else
        set -gx VIBE_DEBUG 1
        echo "🔊 Vibe debug logging enabled"
    end
end

# Print installation message
if test "$VIBE_DEBUG" = "1"
    echo "🔄 Vibe shell hook loaded for Fish (debug mode)"
end