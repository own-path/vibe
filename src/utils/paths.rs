use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::fs;

pub fn get_data_dir() -> Result<PathBuf> {
    let data_dir = dirs::data_dir()
        .or_else(|| dirs::home_dir())
        .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;
    
    let tempo_dir = data_dir.join(".tempo");
    create_secure_directory(&tempo_dir)
        .context("Failed to create tempo data directory")?;
    
    Ok(tempo_dir)
}

pub fn get_log_dir() -> Result<PathBuf> {
    let log_dir = get_data_dir()?.join("logs");
    create_secure_directory(&log_dir)
        .context("Failed to create log directory")?;
    Ok(log_dir)
}

pub fn get_backup_dir() -> Result<PathBuf> {
    let backup_dir = get_data_dir()?.join("backups");
    create_secure_directory(&backup_dir)
        .context("Failed to create backup directory")?;
    Ok(backup_dir)
}

/// Securely canonicalize a path with validation
pub fn canonicalize_path(path: &Path) -> Result<PathBuf> {
    validate_path_security(path)?;
    path.canonicalize()
        .with_context(|| format!("Failed to canonicalize path: {}", path.display()))
}

pub fn is_git_repository(path: &Path) -> bool {
    path.join(".git").exists()
}

pub fn has_tempo_marker(path: &Path) -> bool {
    path.join(".tempo").exists()
}

pub fn detect_project_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| is_valid_project_name(name))
        .unwrap_or("unknown")
        .to_string()
}

pub fn get_git_hash(path: &Path) -> Option<String> {
    if !is_git_repository(path) {
        return None;
    }
    
    // Try to read .git/HEAD and .git/config to create a unique hash
    let git_dir = path.join(".git");
    
    let head_content = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let config_content = std::fs::read_to_string(git_dir.join("config")).ok()?;
    
    // Create a simple hash from the combination
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    head_content.hash(&mut hasher);
    config_content.hash(&mut hasher);
    
    Some(format!("{:x}", hasher.finish()))
}

/// Create a directory with secure permissions
fn create_secure_directory(path: &Path) -> Result<()> {
    if path.exists() {
        validate_directory_permissions(path)?;
        return Ok(());
    }
    
    fs::create_dir_all(path)
        .with_context(|| format!("Failed to create directory: {}", path.display()))?;
        
    // Set secure permissions on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o700); // Owner read/write/execute only
        fs::set_permissions(path, perms)?;
    }
    
    Ok(())
}

/// Validate path for security issues
fn validate_path_security(path: &Path) -> Result<()> {
    let path_str = path.to_str()
        .ok_or_else(|| anyhow::anyhow!("Path contains invalid Unicode"))?;
    
    // Check for path traversal attempts
    if path_str.contains("..") {
        return Err(anyhow::anyhow!("Path traversal detected: {}", path_str));
    }
    
    // Check for null bytes
    if path_str.contains('\0') {
        return Err(anyhow::anyhow!("Path contains null bytes: {}", path_str));
    }
    
    // Check for excessively long paths
    if path_str.len() > 4096 {
        return Err(anyhow::anyhow!("Path is too long: {} characters", path_str.len()));
    }
    
    Ok(())
}

/// Validate directory permissions
fn validate_directory_permissions(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("Failed to read metadata for: {}", path.display()))?;
    
    if !metadata.is_dir() {
        return Err(anyhow::anyhow!("Path is not a directory: {}", path.display()));
    }
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode();
        // Check that directory is not world-writable
        if mode & 0o002 != 0 {
            return Err(anyhow::anyhow!(
                "Directory is world-writable (insecure): {}", 
                path.display()
            ));
        }
    }
    
    Ok(())
}

/// Validate project name for security and sanity
fn is_valid_project_name(name: &str) -> bool {
    // Must not be empty or just whitespace
    if name.trim().is_empty() {
        return false;
    }
    
    // Must not contain dangerous characters
    if name.contains('\0') || name.contains('/') || name.contains('\\') {
        return false;
    }
    
    // Must not be relative path components
    if name == "." || name == ".." {
        return false;
    }
    
    // Length check
    if name.len() > 255 {
        return false;
    }
    
    true
}

/// Validate and sanitize project path for creation
pub fn validate_project_path(path: &Path) -> Result<PathBuf> {
    validate_path_security(path)?;
    
    // Ensure path is absolute for security
    let canonical_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .context("Failed to get current directory")?
            .join(path)
    };
    
    // Additional validation for project paths
    if !canonical_path.exists() {
        return Err(anyhow::anyhow!("Project path does not exist: {}", canonical_path.display()));
    }
    
    if !canonical_path.is_dir() {
        return Err(anyhow::anyhow!("Project path is not a directory: {}", canonical_path.display()));
    }
    
    Ok(canonical_path)
}