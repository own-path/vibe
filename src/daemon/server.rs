use super::state::SharedDaemonState;
use anyhow::Result;
use chrono::Datelike;
use log::{debug, error, info, warn};
use std::path::PathBuf;
use tempo_cli::{
    db::queries::{ProjectQueries, SessionQueries},
    utils::ipc::{read_ipc_message, write_ipc_response, IpcMessage, IpcResponse, IpcServer},
};
use tokio::net::UnixStream;

pub struct DaemonServer {
    server: IpcServer,
    state: SharedDaemonState,
}

impl DaemonServer {
    pub fn new(socket_path: PathBuf, state: SharedDaemonState) -> Result<Self> {
        let server = IpcServer::new(&socket_path)?;
        Ok(Self { server, state })
    }

    pub async fn run(&self) -> Result<()> {
        info!("Starting IPC server...");

        loop {
            match self.server.accept().await {
                Ok((stream, addr)) => {
                    debug!("Client connected: {:?}", addr);
                    let state = self.state.clone();

                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, state).await {
                            error!("Error handling client: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}

async fn handle_client(mut stream: UnixStream, state: SharedDaemonState) -> Result<()> {
    loop {
        match read_ipc_message(&mut stream).await {
            Ok(message) => {
                debug!("Received message: {:?}", message);
                let response = handle_message(message, &state).await;

                if let Err(e) = write_ipc_response(&mut stream, &response).await {
                    error!("Error writing response: {}", e);
                    break;
                }

                // Check if client requested shutdown
                if matches!(response, IpcResponse::Ok) {
                    if let IpcResponse::Ok = response {
                        // Continue normally
                    }
                }
            }
            Err(e) => {
                debug!("Client disconnected or error reading message: {}", e);
                break;
            }
        }
    }

    Ok(())
}

async fn handle_message(message: IpcMessage, state: &SharedDaemonState) -> IpcResponse {
    match message {
        IpcMessage::Ping => IpcResponse::Pong,

        IpcMessage::GetStatus => {
            let state_guard = state.read().await;
            state_guard.get_status()
        }

        IpcMessage::ProjectEntered { path, context } => {
            let mut state_guard = state.write().await;
            match state_guard.handle_project_entered(path, context).await {
                Ok(()) => IpcResponse::Ok,
                Err(e) => IpcResponse::Error(format!("Failed to handle project entered: {}", e)),
            }
        }

        IpcMessage::ProjectLeft { path } => {
            let mut state_guard = state.write().await;
            match state_guard.handle_project_left(path).await {
                Ok(()) => IpcResponse::Ok,
                Err(e) => IpcResponse::Error(format!("Failed to handle project left: {}", e)),
            }
        }

        IpcMessage::StartSession {
            project_path,
            context,
        } => {
            let mut state_guard = state.write().await;

            if let Some(path) = project_path {
                match state_guard.handle_project_entered(path, context).await {
                    Ok(()) => IpcResponse::Ok,
                    Err(e) => IpcResponse::Error(format!("Failed to start session: {}", e)),
                }
            } else {
                IpcResponse::Error("Project path required for manual session start".to_string())
            }
        }

        IpcMessage::StopSession => {
            let mut state_guard = state.write().await;
            match state_guard.stop_session().await {
                Ok(()) => IpcResponse::Ok,
                Err(e) => IpcResponse::Error(format!("Failed to stop session: {}", e)),
            }
        }

        IpcMessage::PauseSession => {
            let mut state_guard = state.write().await;
            match state_guard.pause_session().await {
                Ok(()) => IpcResponse::Ok,
                Err(e) => IpcResponse::Error(format!("Failed to pause session: {}", e)),
            }
        }

        IpcMessage::ResumeSession => {
            let mut state_guard = state.write().await;
            match state_guard.resume_session().await {
                Ok(()) => IpcResponse::Ok,
                Err(e) => IpcResponse::Error(format!("Failed to resume session: {}", e)),
            }
        }

        IpcMessage::GetActiveSession => {
            let state_guard = state.read().await;
            if let Some(session) = &state_guard.active_session {
                // Convert active session to actual Session model
                let session_model = tempo_cli::models::Session {
                    id: Some(session.session_id),
                    project_id: session.project_id,
                    start_time: session.start_time,
                    end_time: None,
                    context: session.context,
                    paused_duration: session.total_paused,
                    notes: None,
                    created_at: session.start_time,
                };

                IpcResponse::ActiveSession(Some(session_model))
            } else {
                IpcResponse::ActiveSession(None)
            }
        }

        IpcMessage::GetProject(project_id) => {
            let state_guard = state.read().await;
            let db = match state_guard.db.lock() {
                Ok(db) => db,
                Err(e) => {
                    return IpcResponse::Error(format!("Failed to acquire database lock: {}", e))
                }
            };
            match ProjectQueries::find_by_id(&db.connection, project_id) {
                Ok(project) => IpcResponse::Project(project),
                Err(e) => IpcResponse::Error(format!("Failed to get project: {}", e)),
            }
        }

        IpcMessage::GetDailyStats(date) => {
            let state_guard = state.read().await;
            let db = match state_guard.db.lock() {
                Ok(db) => db,
                Err(e) => {
                    return IpcResponse::Error(format!("Failed to acquire database lock: {}", e))
                }
            };
            match SessionQueries::get_daily_stats(&db.connection, date) {
                Ok((sessions_count, total_seconds, avg_seconds)) => IpcResponse::DailyStats {
                    sessions_count,
                    total_seconds,
                    avg_seconds,
                },
                Err(e) => IpcResponse::Error(format!("Failed to get daily stats: {}", e)),
            }
        }

        IpcMessage::GetWeeklyStats => {
            let state_guard = state.read().await;
            let db = match state_guard.db.lock() {
                Ok(db) => db,
                Err(e) => {
                    return IpcResponse::Error(format!("Failed to acquire database lock: {}", e))
                }
            };

            let now = chrono::Local::now().date_naive();
            let days_from_monday = now.weekday().num_days_from_monday();
            let start_of_week = now - chrono::Duration::days(days_from_monday as i64);

            match SessionQueries::get_weekly_stats(&db.connection, start_of_week) {
                Ok(total_seconds) => IpcResponse::WeeklyStats { total_seconds },
                Err(e) => IpcResponse::Error(format!("Failed to get weekly stats: {}", e)),
            }
        }

        IpcMessage::GetSessionsForDate(date) => {
            let state_guard = state.read().await;
            let db = match state_guard.db.lock() {
                Ok(db) => db,
                Err(e) => {
                    return IpcResponse::Error(format!("Failed to acquire database lock: {}", e))
                }
            };
            let from = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
            let to = date.and_hms_opt(23, 59, 59).unwrap().and_utc();
            match SessionQueries::list_by_date_range(&db.connection, from, to) {
                Ok(sessions) => IpcResponse::SessionList(sessions),
                Err(e) => IpcResponse::Error(format!("Failed to get sessions for date: {}", e)),
            }
        }

        IpcMessage::GetSessionMetrics(_session_id) => {
            let state_guard = state.read().await;
            match state_guard.get_session_metrics() {
                Some(metrics) => IpcResponse::SessionMetrics(metrics),
                None => IpcResponse::Error("No active session".to_string()),
            }
        }

        IpcMessage::ActivityHeartbeat => {
            let mut state_guard = state.write().await;
            match state_guard.update_activity().await {
                Ok(()) => IpcResponse::Ok,
                Err(e) => IpcResponse::Error(format!("Failed to update activity: {}", e)),
            }
        }

        IpcMessage::SubscribeToUpdates => {
            // For now, just confirm subscription
            // In a full implementation, this would maintain a list of subscribers
            IpcResponse::SubscriptionConfirmed
        }

        IpcMessage::UnsubscribeFromUpdates => IpcResponse::Ok,

        IpcMessage::ListProjects => {
            let state_guard = state.read().await;
            let db = match state_guard.db.lock() {
                Ok(db) => db,
                Err(e) => {
                    return IpcResponse::Error(format!("Failed to acquire database lock: {}", e))
                }
            };
            match ProjectQueries::list_all(&db.connection, false) {
                Ok(projects) => IpcResponse::ProjectList(projects),
                Err(e) => IpcResponse::Error(format!("Failed to list projects: {}", e)),
            }
        }

        IpcMessage::GetRecentProjects => {
            let state_guard = state.read().await;
            let db = match state_guard.db.lock() {
                Ok(db) => db,
                Err(e) => {
                    return IpcResponse::Error(format!("Failed to acquire database lock: {}", e))
                }
            };
            match ProjectQueries::list_recent_with_stats(&db.connection, 5) {
                Ok(projects) => {
                    let projects_with_stats = projects
                        .into_iter()
                        .map(|(project, today_seconds, total_seconds, last_active)| {
                            use tempo_cli::utils::ipc::ProjectWithStats;
                            ProjectWithStats {
                                project,
                                today_seconds,
                                total_seconds,
                                last_active,
                            }
                        })
                        .collect();
                    IpcResponse::RecentProjects(projects_with_stats)
                }
                Err(e) => IpcResponse::Error(format!("Failed to list recent projects: {}", e)),
            }
        }

        IpcMessage::SwitchProject(project_id) => {
            let mut state_guard = state.write().await;

            // Stop current session if active
            if state_guard.active_session.is_some() {
                if let Err(e) = state_guard.stop_session().await {
                    return IpcResponse::Error(format!("Failed to stop current session: {}", e));
                }
            }

            // Start new session for the selected project
            let project = {
                let db = match state_guard.db.lock() {
                    Ok(db) => db,
                    Err(e) => {
                        return IpcResponse::Error(format!(
                            "Failed to acquire database lock: {}",
                            e
                        ))
                    }
                };
                match ProjectQueries::find_by_id(&db.connection, project_id) {
                    Ok(Some(p)) => Some(p),
                    Ok(None) => None,
                    Err(e) => return IpcResponse::Error(format!("Database error: {}", e)),
                }
            }; // db lock is dropped here

            match project {
                Some(project) => {
                    use tempo_cli::models::SessionContext;
                    let context = SessionContext::Manual; // Manual switch via TUI
                    match state_guard
                        .start_session_for_project(project, context)
                        .await
                    {
                        Ok(_) => IpcResponse::Success,
                        Err(e) => IpcResponse::Error(format!("Failed to start session: {}", e)),
                    }
                }
                None => IpcResponse::Error("Project not found".to_string()),
            }
        }

        IpcMessage::Shutdown => {
            info!("Shutdown requested via IPC");
            let mut state_guard = state.write().await;
            if let Err(e) = state_guard.stop_session().await {
                warn!("Error stopping session during shutdown: {}", e);
            }

            // Signal shutdown
            std::process::exit(0);
        }
    }
}
