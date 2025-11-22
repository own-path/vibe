use crate::db::{initialize_database, Database};
use anyhow::Result;
use std::path::PathBuf;
use tempfile::{NamedTempFile, TempDir};

/// Test utilities for setting up isolated test environments
pub struct TestContext {
    pub temp_dir: TempDir,
    pub db_path: PathBuf,
    pub database: Database,
    _temp_file: NamedTempFile, // Keep the file alive
}

impl TestContext {
    /// Create a new isolated test context with a temporary database
    pub fn new() -> Result<Self> {
        let temp_dir = tempfile::tempdir()?;
        let temp_file = NamedTempFile::new_in(&temp_dir)?;
        let db_path = temp_file.path().to_path_buf();

        // Initialize database with schema
        let database = initialize_database(&db_path)?;

        Ok(Self {
            temp_dir,
            db_path,
            database,
            _temp_file: temp_file,
        })
    }

    /// Get the database connection
    pub fn connection(&self) -> &rusqlite::Connection {
        &self.database.connection
    }

    /// Create a temporary project directory
    pub fn create_temp_project_dir(&self) -> Result<PathBuf> {
        let project_dir = self.temp_dir.path().join("test_project");
        std::fs::create_dir_all(&project_dir)?;
        Ok(project_dir)
    }

    /// Create a temporary Git repository for testing
    pub fn create_temp_git_repo(&self) -> Result<PathBuf> {
        let git_dir = self.temp_dir.path().join("git_project");
        std::fs::create_dir_all(&git_dir)?;

        // Create .git directory to simulate Git repo
        let git_meta = git_dir.join(".git");
        std::fs::create_dir_all(&git_meta)?;

        // Create basic git files
        std::fs::write(git_meta.join("HEAD"), "ref: refs/heads/main\n")?;
        std::fs::write(
            git_meta.join("config"),
            "[core]\n\trepositoryformatversion = 0\n",
        )?;

        Ok(git_dir)
    }

    /// Create a temporary Tempo-tracked project directory  
    pub fn create_temp_tempo_project(&self) -> Result<PathBuf> {
        let tempo_dir = self.temp_dir.path().join("tempo_project");
        std::fs::create_dir_all(&tempo_dir)?;

        // Create .tempo marker file
        std::fs::write(tempo_dir.join(".tempo"), "")?;

        Ok(tempo_dir)
    }
}

/// Helper for testing database operations
pub fn with_test_db<F>(test_fn: F)
where
    F: FnOnce(&TestContext) -> Result<()>,
{
    let ctx = TestContext::new().expect("Failed to create test context");
    test_fn(&ctx).expect("Test function failed");
}

/// Helper for async tests with database
pub async fn with_test_db_async<F, Fut>(test_fn: F)
where
    F: FnOnce(TestContext) -> Fut,
    Fut: std::future::Future<Output = Result<()>>,
{
    let ctx = TestContext::new().expect("Failed to create test context");
    test_fn(ctx).await.expect("Async test function failed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = TestContext::new().unwrap();
        assert!(ctx.db_path.exists());
        assert!(ctx.temp_dir.path().exists());
    }

    #[test]
    fn test_temp_directories() {
        let ctx = TestContext::new().unwrap();

        let project_dir = ctx.create_temp_project_dir().unwrap();
        assert!(project_dir.exists());
        assert!(project_dir.is_dir());

        let git_dir = ctx.create_temp_git_repo().unwrap();
        assert!(git_dir.exists());
        assert!(git_dir.join(".git").exists());

        let tempo_dir = ctx.create_temp_tempo_project().unwrap();
        assert!(tempo_dir.exists());
        assert!(tempo_dir.join(".tempo").exists());
    }

    #[test]
    fn test_with_test_db() {
        with_test_db(|ctx| {
            // Test database connection
            let result = ctx
                .connection()
                .execute("CREATE TABLE test_table (id INTEGER PRIMARY KEY)", []);
            assert!(result.is_ok());
            Ok(())
        });
    }

    #[tokio::test]
    async fn test_with_test_db_async() {
        with_test_db_async(|ctx| async move {
            // Test async operations
            let project_dir = ctx.create_temp_project_dir()?;
            assert!(project_dir.exists());
            Ok(())
        })
        .await;
    }
}
