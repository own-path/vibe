use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn get_data_dir() -> Result<PathBuf> {
    let data_dir = dirs::data_dir()
        .or_else(|| dirs::home_dir())
        .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;
    
    let vibe_dir = data_dir.join(".vibe");
    std::fs::create_dir_all(&vibe_dir)?;
    
    Ok(vibe_dir)
}

pub fn get_log_dir() -> Result<PathBuf> {
    let log_dir = get_data_dir()?.join("logs");
    std::fs::create_dir_all(&log_dir)?;
    Ok(log_dir)
}

pub fn get_backup_dir() -> Result<PathBuf> {
    let backup_dir = get_data_dir()?.join("backups");
    std::fs::create_dir_all(&backup_dir)?;
    Ok(backup_dir)
}

pub fn canonicalize_path(path: &Path) -> Result<PathBuf> {
    Ok(path.canonicalize()?)
}

pub fn is_git_repository(path: &Path) -> bool {
    path.join(".git").exists()
}

pub fn has_vibe_marker(path: &Path) -> bool {
    path.join(".vibe").exists()
}

pub fn detect_project_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
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