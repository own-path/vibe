use anyhow::{Result, Context};
use std::path::PathBuf;

use crate::db::{Database, get_database_path};
use crate::db::queries::ProjectQueries;
use crate::models::Project;
use crate::utils::paths::{
    canonicalize_path, detect_project_name, get_git_hash, 
    is_git_repository, has_tempo_marker
};
use crate::utils::validation::{
    validate_project_name, validate_project_description, validate_project_id,
    validate_project_path_enhanced
};

#[cfg(test)]
use crate::test_utils::{TestContext, with_test_db_async};

/// Service layer for project-related business logic
pub struct ProjectService;

impl ProjectService {
    /// Create a new project with validation and auto-detection
    pub async fn create_project(
        name: Option<String>, 
        path: Option<PathBuf>, 
        description: Option<String>
    ) -> Result<Project> {
        // Validate inputs early
        let validated_name = if let Some(n) = name {
            Some(validate_project_name(&n)
                .context("Invalid project name provided")?)
        } else {
            None
        };
        
        let validated_description = if let Some(d) = description {
            Some(validate_project_description(&d)
                .context("Invalid project description provided")?)
        } else {
            None
        };
        
        let project_path = if let Some(path) = path {
            validate_project_path_enhanced(&path)
                .context("Invalid project path provided")?
        } else {
            std::env::current_dir()
                .context("Failed to get current directory")?
        };

        let canonical_path = canonicalize_path(&project_path)?;
        
        // Auto-detect project name if not provided
        let project_name = validated_name.unwrap_or_else(|| {
            let detected = detect_project_name(&canonical_path);
            validate_project_name(&detected).unwrap_or_else(|_| "project".to_string())
        });
        
        // Perform database operations in a blocking task
        let mut project = Project::new(project_name, canonical_path.clone());
        project = project.with_description(validated_description);
        
        // Add Git metadata if available
        let git_hash = get_git_hash(&canonical_path);
        project = project.with_git_hash(git_hash);

        // Set description based on project type
        if project.description.is_none() {
            let auto_description = if is_git_repository(&canonical_path) {
                Some("Git repository".to_string())
            } else if has_tempo_marker(&canonical_path) {
                Some("Tempo tracked project".to_string())
            } else {
                None
            };
            project = project.with_description(auto_description);
        }

        // Database operations in blocking task
        let canonical_path_clone = canonical_path.clone();
        let project_clone = project.clone();
        
        let project_id = tokio::task::spawn_blocking(move || -> Result<i64> {
            let db = Self::get_database_sync()?;
            
            // Check if project already exists
            if let Some(existing) = ProjectQueries::find_by_path(&db.connection, &canonical_path_clone)? {
                return Err(anyhow::anyhow!(
                    "A project named '{}' already exists at this path. Use 'tempo list' to see existing projects.", 
                    existing.name
                ));
            }
            
            // Save to database
            ProjectQueries::create(&db.connection, &project_clone)
        }).await??;
        
        project.id = Some(project_id);

        Ok(project)
    }

    /// List projects with optional filtering
    pub async fn list_projects(include_archived: bool, _tag_filter: Option<String>) -> Result<Vec<Project>> {
        tokio::task::spawn_blocking(move || -> Result<Vec<Project>> {
            let db = Self::get_database_sync()?;
            
            // TODO: Add tag filtering logic when tag system is implemented
            let projects = ProjectQueries::list_all(&db.connection, include_archived)?;
            
            Ok(projects)
        }).await?
    }

    /// Get a project by ID
    pub async fn get_project_by_id(project_id: i64) -> Result<Option<Project>> {
        let validated_id = validate_project_id(project_id)
            .context("Invalid project ID")?;
            
        tokio::task::spawn_blocking(move || -> Result<Option<Project>> {
            let db = Self::get_database_sync()?;
            ProjectQueries::find_by_id(&db.connection, validated_id)
        }).await?
    }

    /// Get a project by path
    pub async fn get_project_by_path(path: &PathBuf) -> Result<Option<Project>> {
        let canonical_path = canonicalize_path(path)?;
        tokio::task::spawn_blocking(move || -> Result<Option<Project>> {
            let db = Self::get_database_sync()?;
            ProjectQueries::find_by_path(&db.connection, &canonical_path)
        }).await?
    }

    /// Update project metadata
    pub async fn update_project(project_id: i64, name: Option<String>, description: Option<String>) -> Result<bool> {
        let validated_id = validate_project_id(project_id)
            .context("Invalid project ID")?;
            
        let validated_name = if let Some(n) = name {
            Some(validate_project_name(&n)
                .context("Invalid project name")?)
        } else {
            None
        };
        
        let validated_description = if let Some(d) = description {
            Some(validate_project_description(&d)
                .context("Invalid project description")?)
        } else {
            None
        };
        
        tokio::task::spawn_blocking(move || -> Result<bool> {
            let db = Self::get_database_sync()?;
            
            let mut updated = false;
            
            if let Some(name) = validated_name {
                let result = ProjectQueries::update_name(&db.connection, validated_id, name)?;
                if !result {
                    return Err(anyhow::anyhow!("Project with ID {} not found", validated_id));
                }
                updated = true;
            }
            
            if let Some(description) = validated_description {
                let result = ProjectQueries::update_project_description(&db.connection, validated_id, Some(description))?;
                if !result {
                    return Err(anyhow::anyhow!("Project with ID {} not found", validated_id));
                }
                updated = true;
            }
            
            Ok(updated)
        }).await?
    }

    /// Archive a project
    pub async fn archive_project(project_id: i64) -> Result<bool> {
        let validated_id = validate_project_id(project_id)
            .context("Invalid project ID")?;
            
        tokio::task::spawn_blocking(move || -> Result<bool> {
            let db = Self::get_database_sync()?;
            let result = ProjectQueries::update_archived(&db.connection, validated_id, true)?;
            if !result {
                return Err(anyhow::anyhow!("Project with ID {} not found", validated_id));
            }
            Ok(result)
        }).await?
    }

    /// Unarchive a project
    pub async fn unarchive_project(project_id: i64) -> Result<bool> {
        let validated_id = validate_project_id(project_id)
            .context("Invalid project ID")?;
            
        tokio::task::spawn_blocking(move || -> Result<bool> {
            let db = Self::get_database_sync()?;
            let result = ProjectQueries::update_archived(&db.connection, validated_id, false)?;
            if !result {
                return Err(anyhow::anyhow!("Project with ID {} not found", validated_id));
            }
            Ok(result)
        }).await?
    }

    // Private helper to get database connection (synchronous for use in blocking tasks)
    fn get_database_sync() -> Result<Database> {
        let db_path = get_database_path()?;
        Database::new(&db_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::with_test_db;
    use tempfile::tempdir;
    use std::fs;

    #[tokio::test]
    async fn test_create_project_with_auto_detection() {
        let temp_dir = tempdir().unwrap();
        let project_path = temp_dir.path().to_path_buf();
        
        let result = ProjectService::create_project(
            None, // Auto-detect name
            Some(project_path.clone()),
            None
        ).await;

        assert!(result.is_ok());
        let project = result.unwrap();
        // Compare canonicalized paths since temp paths can be symlinked
        let expected_canonical = canonicalize_path(&project_path).unwrap();
        assert_eq!(project.path, expected_canonical);
        assert!(!project.name.is_empty());
    }

    #[tokio::test]
    async fn test_path_validation() {
        // Test invalid path
        let invalid_path = PathBuf::from("/nonexistent/path/that/should/not/exist");
        let result = ProjectService::create_project(
            Some("Test Project".to_string()),
            Some(invalid_path),
            None
        ).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_project_name_detection() {
        let temp_dir = tempdir().unwrap();
        
        // Test with specific directory name
        let project_dir = temp_dir.path().join("my-awesome-project");
        fs::create_dir_all(&project_dir).unwrap();
        
        let detected_name = detect_project_name(&project_dir);
        assert_eq!(detected_name, "my-awesome-project");
    }

    #[tokio::test] 
    async fn test_git_repository_detection() {
        let temp_dir = tempdir().unwrap();
        let git_dir = temp_dir.path().join("git_project");
        fs::create_dir_all(&git_dir).unwrap();
        
        // Create .git directory
        let git_meta = git_dir.join(".git");
        fs::create_dir_all(&git_meta).unwrap();
        fs::write(git_meta.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        
        // Test Git repository detection
        assert!(is_git_repository(&git_dir));
        
        // Test project creation with Git repo
        let result = ProjectService::create_project(
            Some("Git Test".to_string()),
            Some(git_dir.clone()),
            None
        ).await;
        
        if let Ok(project) = result {
            assert_eq!(project.description, Some("Git repository".to_string()));
        }
    }

    #[tokio::test]
    async fn test_tempo_marker_detection() {
        let temp_dir = tempdir().unwrap();
        let tempo_dir = temp_dir.path().join("tempo_project");
        fs::create_dir_all(&tempo_dir).unwrap();
        
        // Create .tempo marker
        fs::write(tempo_dir.join(".tempo"), "").unwrap();
        
        // Test Tempo marker detection
        assert!(has_tempo_marker(&tempo_dir));
        
        // Test project creation with Tempo marker
        let result = ProjectService::create_project(
            Some("Tempo Test".to_string()),
            Some(tempo_dir.clone()),
            None
        ).await;
        
        if let Ok(project) = result {
            assert_eq!(project.description, Some("Tempo tracked project".to_string()));
        }
    }

    #[tokio::test]
    async fn test_project_filtering() {
        // Test list projects with no tag filter
        let result = ProjectService::list_projects(false, None).await;
        assert!(result.is_ok());
        
        // Test list projects including archived
        let result_archived = ProjectService::list_projects(true, None).await;
        assert!(result_archived.is_ok());
        
        // Test tag filtering (placeholder for future implementation)
        let result_filtered = ProjectService::list_projects(false, Some("work".to_string())).await;
        assert!(result_filtered.is_ok());
    }

    #[tokio::test]
    async fn test_project_retrieval_edge_cases() {
        // Test get project by non-existent ID
        let result = ProjectService::get_project_by_id(99999).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        
        // Test get project by non-existent path
        // Use a temp directory that exists but has no project
        let temp_dir = tempdir().unwrap();
        let nonexistent_project_path = temp_dir.path().join("nonexistent_project");
        std::fs::create_dir_all(&nonexistent_project_path).unwrap();
        
        let result = ProjectService::get_project_by_path(&nonexistent_project_path).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_project_update_operations() {
        // Test updating non-existent project (should now fail with better error)
        let result = ProjectService::update_project(
            99999,
            Some("New Name".to_string()),
            Some("New Description".to_string())
        ).await;
        // Should fail with "Project not found" error
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Project with ID 99999 not found"));
        
        // Test archiving non-existent project
        let archive_result = ProjectService::archive_project(99999).await;
        assert!(archive_result.is_err());
        assert!(archive_result.unwrap_err().to_string().contains("Project with ID 99999 not found"));
        
        // Test unarchiving non-existent project
        let unarchive_result = ProjectService::unarchive_project(99999).await;
        assert!(unarchive_result.is_err());
        assert!(unarchive_result.unwrap_err().to_string().contains("Project with ID 99999 not found"));
    }
}