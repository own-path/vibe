use tempo::db::{Database, queries::{ProjectQueries, SessionQueries}, advanced_queries::GoalQueries};
use tempo::models::{Project, Session, SessionContext, GoalStatus};
use tempo::utils::paths::{canonicalize_path, detect_project_name, get_git_hash, is_git_repository, has_vibe_marker};
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
    
    // Enhanced monitoring fields
    pub activity_events: Vec<ActivityEvent>,
    pub activity_score: f64,
    pub milestone_tracker: MilestoneTracker,
}

#[derive(Debug, Clone)]
pub struct ActivityEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub duration_delta: i64,
}

#[derive(Debug, Clone)]
pub struct MilestoneTracker {
    pub last_milestone: Option<String>,
    pub reached_milestones: Vec<String>,
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
            
            // Initialize monitoring fields
            activity_events: vec![ActivityEvent {
                timestamp: Utc::now(),
                event_type: "session_started".to_string(),
                duration_delta: 0,
            }],
            activity_score: 1.0,
            milestone_tracker: MilestoneTracker {
                last_milestone: None,
                reached_milestones: Vec::new(),
            },
        });
        
        info!("Started session {} for project {}", session_id, project_name);
        Ok(())
    }

    pub async fn stop_session(&mut self) -> Result<()> {
        if let Some(session) = &self.active_session {
            info!("Stopping session {} for project {}", session.session_id, session.project_name);
            
            // Calculate session duration for goal progress
            let session_duration_hours = if let Some(paused_at) = session.paused_at {
                (paused_at - session.start_time - session.total_paused).num_seconds() as f64 / 3600.0
            } else {
                (Utc::now() - session.start_time - session.total_paused).num_seconds() as f64 / 3600.0
            };
            
            // End session in database
            let db = self.db.lock().unwrap();
            SessionQueries::end_session(&db.connection, session.session_id)?;
            
            // Update goals for this project with the session duration
            if session_duration_hours > 0.0 {
                match GoalQueries::list_by_project(&db.connection, Some(session.project_id)) {
                    Ok(goals) => {
                        for goal in goals.iter().filter(|g| g.status == GoalStatus::Active) {
                            if let Some(goal_id) = goal.id {
                                match GoalQueries::update_progress(&db.connection, goal_id, session_duration_hours) {
                                    Ok(true) => {
                                        info!("Updated goal '{}' progress by {:.2} hours", goal.name, session_duration_hours);
                                    }
                                    Ok(false) => {
                                        debug!("Goal '{}' progress update returned false", goal.name);
                                    }
                                    Err(e) => {
                                        warn!("Failed to update goal '{}' progress: {}", goal.name, e);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to fetch goals for automatic progress update: {}", e);
                    }
                }
            }
            
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
        let should_resume = if let Some(session) = &mut self.active_session {
            let now = Utc::now();
            let time_since_last = (now - session.last_activity).num_seconds();
            
            session.last_activity = now;
            
            // Add activity event
            session.activity_events.push(ActivityEvent {
                timestamp: now,
                event_type: "activity_detected".to_string(),
                duration_delta: time_since_last,
            });
            
            // Check if we need to resume
            session.paused_at.is_some()
        } else {
            false
        };

        // Update activity score based on recent activity
        self.update_activity_score();
        
        // Check for milestones
        self.check_session_milestones().await?;
        
        // Resume if needed
        if should_resume {
            self.resume_session().await?;
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
    
    fn update_activity_score(&mut self) {
        if let Some(session) = &mut self.active_session {
            let now = Utc::now();
            let session_duration = (now - session.start_time).num_seconds();
            let recent_events = session.activity_events.iter()
                .filter(|event| (now - event.timestamp).num_seconds() < 300) // Last 5 minutes
                .count();
            
            // Calculate activity score based on recent activity frequency
            // Score ranges from 0.0 (no activity) to 1.0 (high activity)
            session.activity_score = (recent_events as f64 / 10.0).min(1.0);
        }
    }
    
    async fn check_session_milestones(&mut self) -> Result<()> {
        if let Some(session) = &mut self.active_session {
            let now = Utc::now();
            let duration_minutes = (now - session.start_time - session.total_paused).num_minutes();
            
            let milestones = [
                (15, "15-minute focus session"),
                (30, "Half hour milestone"), 
                (60, "One hour of focused work"),
                (90, "90-minute deep work session"),
                (120, "Two hours of productivity"),
                (180, "Three hour marathon session"),
            ];
            
            for (minutes, message) in milestones {
                let milestone_key = format!("{}_minutes", minutes);
                
                if duration_minutes >= minutes 
                    && !session.milestone_tracker.reached_milestones.contains(&milestone_key) {
                    
                    session.milestone_tracker.reached_milestones.push(milestone_key.clone());
                    session.milestone_tracker.last_milestone = Some(message.to_string());
                    
                    // Add milestone event
                    session.activity_events.push(ActivityEvent {
                        timestamp: now,
                        event_type: format!("milestone_reached: {}", message),
                        duration_delta: 0,
                    });
                    
                    info!("Milestone reached: {} (Session {})", message, session.session_id);
                }
            }
        }
        Ok(())
    }
    
    pub fn get_session_metrics(&self) -> Option<tempo::utils::ipc::SessionMetrics> {
        self.active_session.as_ref().map(|session| {
            let now = Utc::now();
            let total_duration = (now - session.start_time).num_seconds();
            let active_duration = total_duration - session.total_paused.num_seconds();
            
            tempo::utils::ipc::SessionMetrics {
                session_id: session.session_id,
                active_duration,
                total_duration,
                paused_duration: session.total_paused.num_seconds(),
                activity_score: session.activity_score,
                last_activity: session.last_activity,
                productivity_rating: None, // Could be calculated based on activity patterns
            }
        })
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

    pub fn get_status(&self) -> tempo::utils::ipc::IpcResponse {
        let active_session = self.active_session.as_ref().map(|session| {
            let duration = if let Some(paused_at) = session.paused_at {
                // If paused, calculate duration up to pause time
                (paused_at - session.start_time - session.total_paused).num_seconds()
            } else {
                // If active, calculate current duration
                (Utc::now() - session.start_time - session.total_paused).num_seconds()
            };

            tempo::utils::ipc::SessionInfo {
                id: session.session_id,
                project_name: session.project_name.clone(),
                project_path: session.project_path.clone(),
                start_time: session.start_time,
                context: session.context.to_string(),
                duration,
            }
        });

        tempo::utils::ipc::IpcResponse::Status {
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