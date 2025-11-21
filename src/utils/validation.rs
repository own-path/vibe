use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};

/// Custom error types for better error handling
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Project name is invalid: {reason}")]
    InvalidProjectName { reason: String },
    
    #[error("Project path is invalid: {reason}")]
    InvalidProjectPath { reason: String },
    
    #[error("Session parameter is invalid: {field} - {reason}")]
    InvalidSessionParameter { field: String, reason: String },
    
    #[error("Date range is invalid: {reason}")]
    InvalidDateRange { reason: String },
    
    #[error("Input string is invalid: {reason}")]
    InvalidString { reason: String },
    
    #[error("Numeric value is invalid: {field} - {reason}")]
    InvalidNumeric { field: String, reason: String },
}

/// Comprehensive project name validation
pub fn validate_project_name(name: &str) -> Result<String> {
    let trimmed = name.trim();
    
    if trimmed.is_empty() {
        return Err(ValidationError::InvalidProjectName {
            reason: "Project name cannot be empty or whitespace only".to_string()
        }.into());
    }
    
    if trimmed.len() > 255 {
        return Err(ValidationError::InvalidProjectName {
            reason: format!("Project name too long (max 255 characters, got {})", trimmed.len())
        }.into());
    }
    
    if trimmed.len() < 2 {
        return Err(ValidationError::InvalidProjectName {
            reason: "Project name must be at least 2 characters long".to_string()
        }.into());
    }
    
    // Check for dangerous characters
    let dangerous_chars = ['\0', '/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    if let Some(bad_char) = dangerous_chars.iter().find(|&&c| trimmed.contains(c)) {
        return Err(ValidationError::InvalidProjectName {
            reason: format!("Project name contains invalid character: '{}'", bad_char)
        }.into());
    }
    
    // Check for reserved names
    let reserved_names = [".", "..", "CON", "PRN", "AUX", "NUL", "CLOCK$"];
    let upper_name = trimmed.to_uppercase();
    if reserved_names.contains(&upper_name.as_str()) {
        return Err(ValidationError::InvalidProjectName {
            reason: format!("'{}' is a reserved name and cannot be used", trimmed)
        }.into());
    }
    
    // Check for Windows reserved device names
    let windows_reserved = ["COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", 
                           "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"];
    if windows_reserved.contains(&upper_name.as_str()) {
        return Err(ValidationError::InvalidProjectName {
            reason: format!("'{}' is a Windows reserved device name", trimmed)
        }.into());
    }
    
    // Check for leading/trailing dots or spaces
    if trimmed.starts_with('.') && trimmed.len() <= 3 {
        return Err(ValidationError::InvalidProjectName {
            reason: "Project name cannot start with '.' (hidden files/directories)".to_string()
        }.into());
    }
    
    Ok(trimmed.to_string())
}

/// Validate project description
pub fn validate_project_description(description: &str) -> Result<String> {
    let trimmed = description.trim();
    
    if trimmed.len() > 1000 {
        return Err(ValidationError::InvalidString {
            reason: format!("Description too long (max 1000 characters, got {})", trimmed.len())
        }.into());
    }
    
    // Check for null bytes
    if trimmed.contains('\0') {
        return Err(ValidationError::InvalidString {
            reason: "Description contains null bytes".to_string()
        }.into());
    }
    
    Ok(trimmed.to_string())
}

/// Validate project ID
pub fn validate_project_id(id: i64) -> Result<i64> {
    if id <= 0 {
        return Err(ValidationError::InvalidNumeric {
            field: "project_id".to_string(),
            reason: format!("Project ID must be positive (got {})", id)
        }.into());
    }
    
    if id > i64::MAX / 2 {
        return Err(ValidationError::InvalidNumeric {
            field: "project_id".to_string(), 
            reason: "Project ID too large".to_string()
        }.into());
    }
    
    Ok(id)
}

/// Validate session ID
pub fn validate_session_id(id: i64) -> Result<i64> {
    if id <= 0 {
        return Err(ValidationError::InvalidNumeric {
            field: "session_id".to_string(),
            reason: format!("Session ID must be positive (got {})", id)
        }.into());
    }
    
    Ok(id)
}

/// Validate date range for queries
pub fn validate_date_range(from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>) -> Result<(DateTime<Utc>, DateTime<Utc>)> {
    let now = Utc::now();
    
    let to_date = to.unwrap_or(now);
    let from_date = from.unwrap_or_else(|| to_date - chrono::Duration::days(30));
    
    if from_date > to_date {
        return Err(ValidationError::InvalidDateRange {
            reason: format!(
                "Start date ({}) must be before end date ({})", 
                from_date.format("%Y-%m-%d %H:%M:%S"),
                to_date.format("%Y-%m-%d %H:%M:%S")
            )
        }.into());
    }
    
    // Reasonable upper limit for date ranges
    let max_range = chrono::Duration::days(3650); // ~10 years
    if to_date - from_date > max_range {
        return Err(ValidationError::InvalidDateRange {
            reason: "Date range too large (maximum 10 years)".to_string()
        }.into());
    }
    
    // Don't allow future dates
    if to_date > now + chrono::Duration::hours(1) {
        return Err(ValidationError::InvalidDateRange {
            reason: "End date cannot be more than 1 hour in the future".to_string()
        }.into());
    }
    
    Ok((from_date, to_date))
}

/// Validate limit parameter for queries
pub fn validate_query_limit(limit: Option<usize>) -> Result<usize> {
    let limit = limit.unwrap_or(10);
    
    if limit == 0 {
        return Err(ValidationError::InvalidNumeric {
            field: "limit".to_string(),
            reason: "Limit must be greater than 0".to_string()
        }.into());
    }
    
    if limit > 10000 {
        return Err(ValidationError::InvalidNumeric {
            field: "limit".to_string(),
            reason: "Limit too large (maximum 10,000)".to_string()
        }.into());
    }
    
    Ok(limit)
}

/// Validate session notes
pub fn validate_session_notes(notes: &str) -> Result<String> {
    let trimmed = notes.trim();
    
    if trimmed.len() > 2000 {
        return Err(ValidationError::InvalidString {
            reason: format!("Notes too long (max 2000 characters, got {})", trimmed.len())
        }.into());
    }
    
    // Check for null bytes
    if trimmed.contains('\0') {
        return Err(ValidationError::InvalidString {
            reason: "Notes contain null bytes".to_string()
        }.into());
    }
    
    Ok(trimmed.to_string())
}

/// Validate path for project creation/access
pub fn validate_project_path_enhanced(path: &Path) -> Result<PathBuf> {
    // Use existing security validation
    super::paths::validate_project_path(path)
        .context("Path failed security validation")
}

/// Validate daemon process ID
pub fn validate_process_id(pid: u32) -> Result<u32> {
    if pid == 0 {
        return Err(ValidationError::InvalidNumeric {
            field: "process_id".to_string(),
            reason: "Process ID cannot be 0".to_string()
        }.into());
    }
    
    Ok(pid)
}

/// Validate tag name for projects/sessions
pub fn validate_tag_name(tag: &str) -> Result<String> {
    let trimmed = tag.trim();
    
    if trimmed.is_empty() {
        return Err(ValidationError::InvalidString {
            reason: "Tag name cannot be empty".to_string()
        }.into());
    }
    
    if trimmed.len() > 50 {
        return Err(ValidationError::InvalidString {
            reason: format!("Tag name too long (max 50 characters, got {})", trimmed.len())
        }.into());
    }
    
    // Tags should be simple alphanumeric with limited special chars
    if !trimmed.chars().all(|c| c.is_alphanumeric() || "-_".contains(c)) {
        return Err(ValidationError::InvalidString {
            reason: "Tag name can only contain letters, numbers, hyphens, and underscores".to_string()
        }.into());
    }
    
    Ok(trimmed.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_project_name() {
        // Valid names
        assert!(validate_project_name("my-project").is_ok());
        assert!(validate_project_name("  ProjectName  ").is_ok());
        assert!(validate_project_name("Valid_Project123").is_ok());
        
        // Invalid names
        assert!(validate_project_name("").is_err());
        assert!(validate_project_name("   ").is_err());
        assert!(validate_project_name("a").is_err()); // too short
        assert!(validate_project_name("project/with/slash").is_err());
        assert!(validate_project_name("project\0null").is_err());
        assert!(validate_project_name("CON").is_err()); // reserved
        assert!(validate_project_name("COM1").is_err()); // Windows reserved
        
        // Long name
        let long_name = "a".repeat(300);
        assert!(validate_project_name(&long_name).is_err());
    }

    #[test]
    fn test_validate_project_id() {
        assert!(validate_project_id(1).is_ok());
        assert!(validate_project_id(1000).is_ok());
        
        assert!(validate_project_id(0).is_err());
        assert!(validate_project_id(-1).is_err());
    }

    #[test]
    fn test_validate_date_range() {
        let now = Utc::now();
        let yesterday = now - chrono::Duration::days(1);
        
        // Valid range
        assert!(validate_date_range(Some(yesterday), Some(now)).is_ok());
        
        // Invalid range (from > to)
        assert!(validate_date_range(Some(now), Some(yesterday)).is_err());
        
        // Future date
        let future = now + chrono::Duration::days(1);
        assert!(validate_date_range(Some(yesterday), Some(future)).is_err());
    }

    #[test]
    fn test_validate_query_limit() {
        assert_eq!(validate_query_limit(Some(100)).unwrap(), 100);
        assert_eq!(validate_query_limit(None).unwrap(), 10); // default
        
        assert!(validate_query_limit(Some(0)).is_err());
        assert!(validate_query_limit(Some(20000)).is_err()); // too large
    }

    #[test]
    fn test_validate_tag_name() {
        assert_eq!(validate_tag_name("Work").unwrap(), "work");
        assert_eq!(validate_tag_name("  project-tag_123  ").unwrap(), "project-tag_123");
        
        assert!(validate_tag_name("").is_err());
        assert!(validate_tag_name("tag with spaces").is_err());
        assert!(validate_tag_name("tag@special").is_err());
        
        // Too long
        let long_tag = "a".repeat(60);
        assert!(validate_tag_name(&long_tag).is_err());
    }
}