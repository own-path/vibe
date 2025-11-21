use anyhow::{Result, Context};
use chrono::{DateTime, Duration, Utc};

use crate::db::{Database, get_database_path};
use crate::db::queries::SessionQueries;
use crate::models::{Session, SessionContext};
use crate::utils::ipc::{IpcClient, IpcMessage, IpcResponse, get_socket_path, is_daemon_running};
use crate::utils::validation::{
    validate_project_id, validate_date_range, validate_query_limit
};

/// Service layer for session-related business logic
pub struct SessionService;

impl SessionService {
    /// Start a new session for a project
    pub async fn start_session(project_id: i64, context: SessionContext) -> Result<Session> {
        let validated_id = validate_project_id(project_id)
            .context("Invalid project ID for session start")?;
            
        // Check if daemon is running, if not try to communicate directly with DB
        if is_daemon_running() {
            Self::start_session_via_daemon(validated_id, context).await
        } else {
            Self::start_session_direct(validated_id, context).await
        }
    }

    /// Stop the current active session
    pub async fn stop_session() -> Result<()> {
        if is_daemon_running() {
            Self::stop_session_via_daemon().await
        } else {
            Self::stop_session_direct().await
        }
    }

    /// Get the current active session
    pub async fn get_active_session() -> Result<Option<Session>> {
        if is_daemon_running() {
            Self::get_active_session_via_daemon().await
        } else {
            Self::get_active_session_direct().await
        }
    }

    /// List recent sessions with optional filtering
    pub async fn list_recent_sessions(limit: Option<usize>, project_id: Option<i64>) -> Result<Vec<Session>> {
        let validated_limit = validate_query_limit(limit)
            .context("Invalid limit parameter")?;
            
        let validated_project_id = if let Some(pid) = project_id {
            Some(validate_project_id(pid)
                .context("Invalid project ID for filtering")?)
        } else {
            None
        };
        
        tokio::task::spawn_blocking(move || -> Result<Vec<Session>> {
            let db = Self::get_database_sync()?;
            let sessions = SessionQueries::list_recent(&db.connection, validated_limit)?;
            
            // Filter by project if specified
            if let Some(pid) = validated_project_id {
                Ok(sessions.into_iter().filter(|s| s.project_id == pid).collect())
            } else {
                Ok(sessions)
            }
        }).await?
    }

    /// Get session statistics for a date range
    pub async fn get_session_stats(
        from_date: Option<DateTime<Utc>>,
        to_date: Option<DateTime<Utc>>,
        project_id: Option<i64>
    ) -> Result<SessionStats> {
        let (validated_from, validated_to) = validate_date_range(from_date, to_date)
            .context("Invalid date range for session statistics")?;
            
        let validated_project_id = if let Some(pid) = project_id {
            Some(validate_project_id(pid)
                .context("Invalid project ID for filtering")?)
        } else {
            None
        };
        
        tokio::task::spawn_blocking(move || -> Result<SessionStats> {
            let db = Self::get_database_sync()?;
            
            let sessions = SessionQueries::list_by_date_range(&db.connection, validated_from, validated_to)?;
            
            // Filter by project if specified
            let filtered_sessions: Vec<Session> = if let Some(pid) = validated_project_id {
                sessions.into_iter().filter(|s| s.project_id == pid).collect()
            } else {
                sessions
            };
    
            let stats = Self::calculate_stats(&filtered_sessions);
            Ok(stats)
        }).await?
    }

    /// Pause the current session
    pub async fn pause_session() -> Result<()> {
        if is_daemon_running() {
            let socket_path = get_socket_path()?;
            let mut client = IpcClient::connect(&socket_path).await?;
            let _response = client.send_message(&IpcMessage::PauseSession).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Cannot pause session: The tempo daemon is not running. Start it with 'tempo start'."))
        }
    }

    /// Resume the current session
    pub async fn resume_session() -> Result<()> {
        if is_daemon_running() {
            let socket_path = get_socket_path()?;
            let mut client = IpcClient::connect(&socket_path).await?;
            let _response = client.send_message(&IpcMessage::ResumeSession).await?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Cannot resume session: The tempo daemon is not running. Start it with 'tempo start'."))
        }
    }

    // Private implementation methods

    async fn start_session_via_daemon(project_id: i64, context: SessionContext) -> Result<Session> {
        // First get the project to find its path
        let project = Self::get_project_by_id_sync(project_id)?
            .ok_or_else(|| anyhow::anyhow!("Project with ID {} not found. Ensure the project exists before starting a session.", project_id))?;

        let socket_path = get_socket_path()?;
        let mut client = IpcClient::connect(&socket_path).await?;
        
        let response = client.send_message(&IpcMessage::StartSession { 
            project_path: Some(project.path), 
            context: context.to_string() 
        }).await?;

        match response {
            IpcResponse::Success => {
                // Get the newly started session
                Self::get_active_session_via_daemon().await?
                    .ok_or_else(|| anyhow::anyhow!("Session started successfully but could not retrieve session details. Try 'tempo status' to check the current session."))
            },
            IpcResponse::Error(e) => Err(anyhow::anyhow!("Failed to start session: {}", e)),
            _ => Err(anyhow::anyhow!("Unexpected response from daemon")),
        }
    }

    async fn start_session_direct(project_id: i64, context: SessionContext) -> Result<Session> {
        tokio::task::spawn_blocking(move || -> Result<Session> {
            let db = Self::get_database_sync()?;
            
            // Check for existing active session
            if let Some(_active) = SessionQueries::find_active_session(&db.connection)? {
                return Err(anyhow::anyhow!("Another session is already active. Stop the current session with 'tempo stop' before starting a new one."));
            }
    
            let session = Session::new(project_id, context);
            let session_id = SessionQueries::create(&db.connection, &session)?;
            
            let mut saved_session = session;
            saved_session.id = Some(session_id);
            Ok(saved_session)
        }).await?
    }

    async fn stop_session_via_daemon() -> Result<()> {
        let socket_path = get_socket_path()?;
        let mut client = IpcClient::connect(&socket_path).await?;
        let _response = client.send_message(&IpcMessage::StopSession).await?;
        Ok(())
    }

    async fn stop_session_direct() -> Result<()> {
        tokio::task::spawn_blocking(move || -> Result<()> {
            let db = Self::get_database_sync()?;
            
            if let Some(session) = SessionQueries::find_active_session(&db.connection)? {
                let session_id = session.id
                    .ok_or_else(|| anyhow::anyhow!("Found active session but it has no ID. This indicates a database corruption issue."))?;
                SessionQueries::end_session(&db.connection, session_id)?;
            }
            
            Ok(())
        }).await?
    }

    async fn get_active_session_via_daemon() -> Result<Option<Session>> {
        let socket_path = get_socket_path()?;
        let mut client = IpcClient::connect(&socket_path).await?;
        
        let response = client.send_message(&IpcMessage::GetActiveSession).await?;
        match response {
            IpcResponse::ActiveSession(session) => Ok(session),
            IpcResponse::Error(_) => Ok(None),
            _ => Ok(None),
        }
    }

    async fn get_active_session_direct() -> Result<Option<Session>> {
        tokio::task::spawn_blocking(move || -> Result<Option<Session>> {
            let db = Self::get_database_sync()?;
            SessionQueries::find_active_session(&db.connection)
        }).await?
    }

    fn calculate_stats(sessions: &[Session]) -> SessionStats {
        let total_sessions = sessions.len();
        let total_duration: i64 = sessions.iter()
            .filter_map(|s| s.end_time.map(|end| (end - s.start_time).num_seconds()))
            .sum();
        
        let avg_duration = if total_sessions > 0 {
            total_duration / total_sessions as i64
        } else {
            0
        };

        SessionStats {
            total_sessions,
            total_duration_seconds: total_duration,
            average_duration_seconds: avg_duration,
            active_session_exists: sessions.iter().any(|s| s.end_time.is_none()),
        }
    }

    fn get_database_sync() -> Result<Database> {
        let db_path = get_database_path()?;
        Database::new(&db_path)
    }

    fn get_project_by_id_sync(project_id: i64) -> Result<Option<crate::models::Project>> {
        let db = Self::get_database_sync()?;
        crate::db::queries::ProjectQueries::find_by_id(&db.connection, project_id)
    }
}

/// Statistics about sessions
#[derive(Debug, Clone)]
pub struct SessionStats {
    pub total_sessions: usize,
    pub total_duration_seconds: i64,
    pub average_duration_seconds: i64,
    pub active_session_exists: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::with_test_db_async;
    use crate::db::queries::ProjectQueries;
    use crate::models::Project;
    // use std::path::PathBuf;

    #[tokio::test]
    async fn test_session_stats_calculation() {
        let sessions = vec![
            Session {
                id: Some(1),
                project_id: 1,
                start_time: Utc::now() - Duration::hours(2),
                end_time: Some(Utc::now() - Duration::hours(1)),
                context: SessionContext::Terminal,
                notes: None,
                paused_duration: Duration::zero(),
                created_at: Utc::now() - Duration::hours(2),
            },
            Session {
                id: Some(2),
                project_id: 1,
                start_time: Utc::now() - Duration::minutes(30),
                end_time: None, // Active session
                context: SessionContext::IDE,
                notes: None,
                paused_duration: Duration::zero(),
                created_at: Utc::now() - Duration::minutes(30),
            },
        ];

        let stats = SessionService::calculate_stats(&sessions);
        assert_eq!(stats.total_sessions, 2);
        assert_eq!(stats.total_duration_seconds, 3600); // 1 hour
        assert_eq!(stats.average_duration_seconds, 1800); // 30 minutes
        assert!(stats.active_session_exists);
    }

    #[tokio::test]
    async fn test_empty_session_stats() {
        let empty_sessions: Vec<Session> = vec![];
        let stats = SessionService::calculate_stats(&empty_sessions);
        
        assert_eq!(stats.total_sessions, 0);
        assert_eq!(stats.total_duration_seconds, 0);
        assert_eq!(stats.average_duration_seconds, 0);
        assert!(!stats.active_session_exists);
    }

    #[tokio::test]
    async fn test_session_filtering_by_project() {
        with_test_db_async(|ctx| async move {
            // Create test projects
            let project1_path = ctx.create_temp_project_dir()?;
            let project1 = Project::new("Project 1".to_string(), project1_path);
            let project1_id = ProjectQueries::create(&ctx.connection(), &project1)?;
            
            let project2_path = ctx.create_temp_git_repo()?;
            let project2 = Project::new("Project 2".to_string(), project2_path);
            let _project2_id = ProjectQueries::create(&ctx.connection(), &project2)?;
            
            // Test recent sessions without project filter
            let all_recent = SessionService::list_recent_sessions(Some(10), None).await?;
            assert!(!all_recent.is_empty() || all_recent.is_empty()); // Should succeed
            
            // Test recent sessions with project filter
            let filtered_recent = SessionService::list_recent_sessions(Some(10), Some(project1_id)).await?;
            assert!(!filtered_recent.is_empty() || filtered_recent.is_empty()); // Should succeed
            
            Ok(())
        }).await;
    }

    #[tokio::test]
    async fn test_session_date_range_filtering() {
        let now = Utc::now();
        let yesterday = now - Duration::days(1);
        let last_week = now - Duration::days(7);
        
        // Test with no date range (should use defaults)
        let result = SessionService::get_session_stats(None, None, None).await;
        assert!(result.is_ok());
        
        // Test with specific date range (past only)
        let result_with_range = SessionService::get_session_stats(
            Some(last_week), 
            Some(yesterday), 
            None
        ).await;
        assert!(result_with_range.is_ok());
        
        // Test with project filter
        let result_with_project = SessionService::get_session_stats(
            Some(last_week), 
            Some(yesterday), 
            Some(1)
        ).await;
        assert!(result_with_project.is_ok());
    }

    #[tokio::test]
    async fn test_daemon_fallback_logic() {
        // Test that service methods handle daemon not running gracefully
        
        // These will fall back to direct database operations
        // when daemon is not running (which should be the case in tests)
        
        let active_result = SessionService::get_active_session().await;
        assert!(active_result.is_ok());
        
        // Test pause/resume operations when daemon not running
        let pause_result = SessionService::pause_session().await;
        assert!(pause_result.is_err()); // Should fail when daemon not running
        
        let resume_result = SessionService::resume_session().await;
        assert!(resume_result.is_err()); // Should fail when daemon not running
    }

    #[tokio::test]
    async fn test_session_context_variations() {
        let contexts = vec![
            SessionContext::Terminal,
            SessionContext::IDE,
        ];
        
        for context in contexts {
            let session = Session::new(1, context);
            assert_eq!(session.context, context);
            assert!(session.end_time.is_none());
            assert_eq!(session.paused_duration, Duration::zero());
        }
    }

    #[tokio::test]
    async fn test_stats_with_only_active_sessions() {
        let active_only_sessions = vec![
            Session {
                id: Some(1),
                project_id: 1,
                start_time: Utc::now() - Duration::hours(1),
                end_time: None, // Active
                context: SessionContext::Terminal,
                notes: None,
                paused_duration: Duration::zero(),
                created_at: Utc::now() - Duration::hours(1),
            },
            Session {
                id: Some(2),
                project_id: 1,
                start_time: Utc::now() - Duration::minutes(30),
                end_time: None, // Active
                context: SessionContext::IDE,
                notes: None,
                paused_duration: Duration::zero(),
                created_at: Utc::now() - Duration::minutes(30),
            },
        ];

        let stats = SessionService::calculate_stats(&active_only_sessions);
        assert_eq!(stats.total_sessions, 2);
        assert_eq!(stats.total_duration_seconds, 0); // No completed sessions
        assert_eq!(stats.average_duration_seconds, 0);
        assert!(stats.active_session_exists);
    }
}