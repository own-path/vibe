use super::state::SharedDaemonState;
use anyhow::Result;
use log::{debug, error, info, warn};
use std::path::PathBuf;
use tempo_cli::db::queries::{ProjectQueries, SessionQueries};
use tempo_cli::utils::ipc::{
    read_ipc_message, write_ipc_response, IpcMessage, IpcResponse, IpcServer,
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

            // Get completed sessions for the day
            match SessionQueries::list_with_filter(
                &db.connection,
                None,
                Some(date),
                Some(date),
                None,
            ) {
                Ok(sessions) => {
                    let completed_sessions: Vec<_> = sessions
                        .into_iter()
                        .filter(|s| s.end_time.is_some())
                        .collect();

                    let sessions_count = completed_sessions.len() as i64;

                    let total_seconds: i64 = completed_sessions
                        .iter()
                        .map(|s| s.current_active_duration().num_seconds())
                        .sum();

                    let avg_seconds = if sessions_count > 0 {
                        total_seconds / sessions_count
                    } else {
                        0
                    };

                    IpcResponse::DailyStats {
                        sessions_count,
                        total_seconds,
                        avg_seconds,
                    }
                }
                Err(e) => IpcResponse::Error(format!("Failed to get daily stats: {}", e)),
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
