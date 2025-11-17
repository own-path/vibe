# Vibe shell hook for PowerShell
# Add this to your PowerShell profile

# Configuration
$env:VIBE_BIN = if ($env:VIBE_BIN) { $env:VIBE_BIN } else { "vibe" }
$env:VIBE_DEBUG = if ($env:VIBE_DEBUG) { $env:VIBE_DEBUG } else { "0" }
$env:VIBE_HOOK_ENABLED = if ($env:VIBE_HOOK_ENABLED) { $env:VIBE_HOOK_ENABLED } else { "1" }

# Internal variables
$Global:_VIBE_LAST_DIR = ""

# Debug logging function
function Write-VibeDebug {
    param([string]$Message)
    if ($env:VIBE_DEBUG -eq "1") {
        Write-Host "[VIBE DEBUG] $Message" -ForegroundColor Gray
    }
}

# Function to detect if a directory is a project
function Test-VibeProject {
    param([string]$Dir)
    
    # Check for git repository
    if (Test-Path (Join-Path $Dir ".git")) {
        return $true
    }
    
    # Check for .vibe marker
    if (Test-Path (Join-Path $Dir ".vibe")) {
        return $true
    }
    
    # Check for common project files
    $projectFiles = @(
        "package.json", "Cargo.toml", "pyproject.toml", "pom.xml", 
        "Makefile", "CMakeLists.txt", "go.mod", "composer.json",
        "*.sln", "*.csproj", "*.vbproj", "*.fsproj"
    )
    
    foreach ($file in $projectFiles) {
        if (Get-ChildItem -Path $Dir -Filter $file -ErrorAction SilentlyContinue) {
            return $true
        }
    }
    
    return $false
}

# Function to find the project root
function Find-VibeProjectRoot {
    param([string]$Dir)
    
    $originalDir = $Dir
    $currentDir = $Dir
    
    # Walk up the directory tree
    while ($currentDir -and $currentDir -ne [System.IO.Path]::GetPathRoot($currentDir)) {
        if (Test-VibeProject $currentDir) {
            return $currentDir
        }
        $currentDir = Split-Path $currentDir -Parent
    }
    
    # If no project found, return the original directory if it's reasonable
    $systemPaths = @("C:\Windows", "C:\Program Files", "C:\Program Files (x86)")
    $isSystemPath = $systemPaths | Where-Object { $originalDir.StartsWith($_, [StringComparison]::OrdinalIgnoreCase) }
    
    if (-not $isSystemPath -and (Test-Path $originalDir -PathType Container)) {
        return $originalDir
    }
    
    return $null
}

# Function to send IPC message to vibe daemon
function Send-VibeIpc {
    param([string[]]$Arguments)
    
    # Check if vibe is available
    try {
        $vibeCmd = Get-Command $env:VIBE_BIN -ErrorAction Stop
    }
    catch {
        Write-VibeDebug "vibe binary not found"
        return $false
    }
    
    try {
        & $vibeCmd @Arguments 2>$null
        return $true
    }
    catch {
        Write-VibeDebug "Failed to execute vibe command: $($_.Exception.Message)"
        return $false
    }
}

# Function to handle directory change
function Invoke-VibeDirectoryChange {
    if ($env:VIBE_HOOK_ENABLED -ne "1") {
        return
    }
    
    $currentDir = $PWD.Path
    
    # Skip if we're in the same directory
    if ($currentDir -eq $Global:_VIBE_LAST_DIR) {
        return
    }
    
    Write-VibeDebug "Directory changed from '$Global:_VIBE_LAST_DIR' to '$currentDir'"
    
    # Find project root
    $projectRoot = Find-VibeProjectRoot $currentDir
    if ($projectRoot) {
        Write-VibeDebug "Found project at: $projectRoot"
        
        # Send project entered signal via CLI
        Send-VibeIpc @("session", "start", "--project", $projectRoot, "--context", "terminal")
        
        # Export for other tools
        $env:VIBE_CURRENT_PROJECT = $projectRoot
    }
    else {
        Write-VibeDebug "No project found for: $currentDir"
        Remove-Item Env:VIBE_CURRENT_PROJECT -ErrorAction SilentlyContinue
    }
    
    $Global:_VIBE_LAST_DIR = $currentDir
}

# Override Set-Location to hook directory changes
$Global:OriginalSetLocation = Get-Command Set-Location
function Set-Location {
    param(
        [Parameter(ValueFromPipeline = $true, ValueFromPipelineByPropertyName = $true)]
        [string]$Path,
        [switch]$PassThru,
        [string]$StackName
    )
    
    # Call original Set-Location
    if ($StackName) {
        & $Global:OriginalSetLocation -Path $Path -StackName $StackName -PassThru:$PassThru
    }
    elseif ($Path) {
        & $Global:OriginalSetLocation -Path $Path -PassThru:$PassThru
    }
    else {
        & $Global:OriginalSetLocation -PassThru:$PassThru
    }
    
    # Trigger vibe directory change handler
    Invoke-VibeDirectoryChange
}

# Set up prompt hook
$Global:OriginalPrompt = Get-Command prompt -ErrorAction SilentlyContinue
if ($Global:OriginalPrompt) {
    function prompt {
        Invoke-VibeDirectoryChange
        & $Global:OriginalPrompt
    }
}
else {
    function prompt {
        Invoke-VibeDirectoryChange
        "PS $($PWD.Path)> "
    }
}

# Initialize for current directory
Invoke-VibeDirectoryChange

# Utility functions for manual control
function Enable-VibeTracking {
    $env:VIBE_HOOK_ENABLED = "1"
    Write-Host "✅ Vibe automatic tracking enabled" -ForegroundColor Green
    Invoke-VibeDirectoryChange
}

function Disable-VibeTracking {
    $env:VIBE_HOOK_ENABLED = "0"
    Write-Host "⏸️ Vibe automatic tracking disabled" -ForegroundColor Yellow
}

function Show-VibeStatus {
    if ($env:VIBE_HOOK_ENABLED -eq "1") {
        Write-Host "✅ Vibe automatic tracking is enabled" -ForegroundColor Green
        if ($env:VIBE_CURRENT_PROJECT) {
            Write-Host "   📂 Current project: $env:VIBE_CURRENT_PROJECT" -ForegroundColor Cyan
        }
        else {
            Write-Host "   💤 No active project" -ForegroundColor Gray
        }
    }
    else {
        Write-Host "⏸️ Vibe automatic tracking is disabled" -ForegroundColor Yellow
    }
    
    # Show daemon status
    if (Get-Command $env:VIBE_BIN -ErrorAction SilentlyContinue) {
        Write-Host ""
        & $env:VIBE_BIN status
    }
}

function Toggle-VibeDebug {
    if ($env:VIBE_DEBUG -eq "1") {
        $env:VIBE_DEBUG = "0"
        Write-Host "🔇 Vibe debug logging disabled" -ForegroundColor Gray
    }
    else {
        $env:VIBE_DEBUG = "1"
        Write-Host "🔊 Vibe debug logging enabled" -ForegroundColor Yellow
    }
}

# Set up aliases
Set-Alias -Name vibe-enable -Value Enable-VibeTracking
Set-Alias -Name vibe-disable -Value Disable-VibeTracking  
Set-Alias -Name vibe-status -Value Show-VibeStatus
Set-Alias -Name vibe-debug -Value Toggle-VibeDebug

# Print installation message
if ($env:VIBE_DEBUG -eq "1") {
    Write-Host "🔄 Vibe shell hook loaded for PowerShell (debug mode)" -ForegroundColor Green
}