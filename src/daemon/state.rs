use vibe::db::{Database, queries::{ProjectQueries, SessionQueries}};
use vibe::models::{Project, Session, SessionContext};
use vibe::utils::paths::{canonicalize_path, detect_project_name, get_git_hash, is_git_repository, has_vibe_marker};
use anyhow::Result;
use chrono::{DateTime, Utc, Duration};
use log::{debug, info, warn, error};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration as TokioDuration};

#[derive(Debug, Clone)]
pub struct ActiveSession {
    pub session_id: i64,
    pub project_id: i64,
    pub project_name: String,
    pub project_path: PathBuf,
    pub start_time: DateTime<Utc>,
    pub context: SessionContext,
    pub last_activity: DateTime<Utc>,
    pub paused_at: Option<DateTime<Utc>>,
    pub total_paused: Duration,
}

pub struct DaemonState {
    pub db: Arc<Mutex<Database>>,
    pub active_session: Option<ActiveSession>,
    pub projects_cache: HashMap<PathBuf, Project>,
    pub idle_timeout: Duration,
    pub started_at: DateTime<Utc>,
}

impl DaemonState {
    pub fn new(db: Database) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
            active_session: None,
            projects_cache: HashMap::new(),
            idle_timeout: Duration::minutes(30),
            started_at: Utc::now(),
        }
    }

    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing daemon state...");
        
        // Load projects into cache
        let db = self.db.lock().unwrap();
        let projects = ProjectQueries::list_all(&db.connection, false)?;
        for project in projects {
            if let Some(id) = project.id {
                self.projects_cache.insert(project.path.clone(), project);
            }
        }
        
        // Check for any active sessions from previous daemon run
        if let Some(session) = SessionQueries::find_active_session(&db.connection)? {
            warn!("Found active session from previous run, ending it");
            SessionQueries::end_session(&db.connection, session.id.unwrap())?;
        }
        drop(db); // Release the mutex
        
        info!("Daemon state initialized with {} cached projects", self.projects_cache.len());
        Ok(())
    }

    pub async fn handle_project_entered(&mut self, path: PathBuf, context: String) -> Result<()> {
        debug!("Project entered: {:?}, context: {}", path, context);
        
        let canonical_path = match canonicalize_path(&path) {
            Ok(p) => p,
            Err(_) => {
                warn!("Could not canonicalize path: {:?}", path);
                return Ok(());
            }
        };

        // Check if we're already tracking this project
        if let Some(active) = &self.active_session {
            if active.project_path == canonical_path {
                debug!("Already tracking this project, updating activity");
                self.update_activity().await?;
                return Ok(());
            }
        }

        // Stop current session if any
        if self.active_session.is_some() {
            self.stop_session().await?;
        }

        // Find or create project
        let project = self.find_or_create_project(&canonical_path).await?;
        
        // Start new session
        self.start_session_for_project(project, context.parse().unwrap_or(SessionContext::Terminal)).await?;
        
        Ok(())
    }

    pub async fn handle_project_left(&mut self, _path: PathBuf) -> Result<()> {
        debug!("Project left");
        // We don't immediately stop tracking - let idle timeout handle it
        Ok(())
    }

    pub async fn start_session_for_project(&mut self, project: Project, context: SessionContext) -> Result<()> {
        let project_id = project.id.unwrap();
        let project_name = project.name.clone();
        let project_path = project.path.clone();
        
        info!("Starting session for project: {}", project_name);
        
        // Create session in database
        let session = Session::new(project_id, context);
        let db = self.db.lock().unwrap();
        let session_id = SessionQueries::create(&db.connection, &session)?;
        drop(db);
        
        // Update active session state
        self.active_session = Some(ActiveSession {
            session_id,
            project_id,
            project_name: project_name.clone(),
            project_path: project_path.clone(),
            start_time: session.start_time,
            context,
            last_activity: Utc::now(),
            paused_at: None,
            total_paused: Duration::zero(),
        });
        
        info!("Started session {} for project {}", session_id, project_name);
        Ok(())
    }

    pub async fn stop_session(&mut self) -> Result<()> {
        if let Some(session) = &self.active_session {
            info!("Stopping session {} for project {}", session.session_id, session.project_name);
            
            // End session in database
            let db = self.db.lock().unwrap();
            SessionQueries::end_session(&db.connection, session.session_id)?;
            drop(db);
            
            // Clear active session
            self.active_session = None;
            
            info!("Session stopped");
        }
        
        Ok(())
    }

    pub async fn pause_session(&mut self) -> Result<()> {
        if let Some(session) = &mut self.active_session {
            if session.paused_at.is_none() {
                info!("Pausing session {} for project {}", session.session_id, session.project_name);
                session.paused_at = Some(Utc::now());
            }
        }
        Ok(())
    }

    pub async fn resume_session(&mut self) -> Result<()> {
        if let Some(session) = &mut self.active_session {
            if let Some(paused_at) = session.paused_at {
                info!("Resuming session {} for project {}", session.session_id, session.project_name);
                
                let pause_duration = Utc::now() - paused_at;
                session.total_paused = session.total_paused + pause_duration;
                session.paused_at = None;
                session.last_activity = Utc::now();
            }
        }
        Ok(())
    }

    pub async fn update_activity(&mut self) -> Result<()> {
        if let Some(session) = &mut self.active_session {
            session.last_activity = Utc::now();
            
            // Resume if paused
            if session.paused_at.is_some() {
                self.resume_session().await?;
            }
        }
        Ok(())
    }

    pub async fn check_idle_timeout(&mut self) -> Result<()> {
        if let Some(session) = &self.active_session {
            // Skip if already paused
            if session.paused_at.is_some() {
                return Ok(());
            }
            
            let time_since_activity = Utc::now() - session.last_activity;
            if time_since_activity > self.idle_timeout {
                info!("Session idle for {}m, auto-pausing", time_since_activity.num_minutes());
                self.pause_session().await?;
            }
        }
        Ok(())
    }

    async fn find_or_create_project(&mut self, path: &PathBuf) -> Result<Project> {
        // Check cache first
        if let Some(project) = self.projects_cache.get(path) {
            return Ok(project.clone());
        }

        // Check database
        let db = self.db.lock().unwrap();
        if let Some(project) = ProjectQueries::find_by_path(&db.connection, path)? {
            drop(db);
            self.projects_cache.insert(path.clone(), project.clone());
            return Ok(project);
        }

        // Create new project
        let project_name = detect_project_name(path);
        let git_hash = get_git_hash(path);
        
        let mut project = Project::new(project_name, path.clone());
        project = project.with_git_hash(git_hash);
        
        if is_git_repository(path) {
            project = project.with_description(Some("Git repository".to_string()));
        } else if has_vibe_marker(path) {
            project = project.with_description(Some("Vibe tracked project".to_string()));
        }

        let project_id = ProjectQueries::create(&db.connection, &project)?;
        drop(db);
        project.id = Some(project_id);

        info!("Created new project: {} at {:?}", project.name, project.path);
        
        // Cache the project
        self.projects_cache.insert(path.clone(), project.clone());
        
        Ok(project)
    }

    pub fn get_status(&self) -> vibe::utils::ipc::IpcResponse {
        let active_session = self.active_session.as_ref().map(|session| {
            let duration = if let Some(paused_at) = session.paused_at {
                // If paused, calculate duration up to pause time
                (paused_at - session.start_time - session.total_paused).num_seconds()
            } else {
                // If active, calculate current duration
                (Utc::now() - session.start_time - session.total_paused).num_seconds()
            };

            vibe::utils::ipc::SessionInfo {
                id: session.session_id,
                project_name: session.project_name.clone(),
                project_path: session.project_path.clone(),
                start_time: session.start_time,
                context: session.context.to_string(),
                duration,
            }
        });

        vibe::utils::ipc::IpcResponse::Status {
            daemon_running: true,
            active_session,
            uptime: (Utc::now() - self.started_at).num_seconds() as u64,
        }
    }
}

pub type SharedDaemonState = Arc<RwLock<DaemonState>>;

pub async fn start_idle_checker(state: SharedDaemonState) {
    let mut interval = tokio::time::interval(TokioDuration::from_secs(60)); // Check every minute
    
    loop {
        interval.tick().await;
        
        let mut state_guard = state.write().await;
        if let Err(e) = state_guard.check_idle_timeout().await {
            error!("Error checking idle timeout: {}", e);
        }
    }
}