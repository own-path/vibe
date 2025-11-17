use super::state::SharedDaemonState;
use vibe::utils::ipc::{read_ipc_message, write_ipc_response, IpcMessage, IpcResponse, IpcServer};
use anyhow::Result;
use log::{debug, info, warn, error};
use std::path::PathBuf;
use tokio::net::UnixStream;
use tokio::net::unix::SocketAddr;

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
        IpcMessage::Ping => {
            IpcResponse::Pong
        }

        IpcMessage::GetStatus => {
            let state_guard = state.read().await;
            state_guard.get_status()
        }

        IpcMessage::ProjectEntered { path, context } => {
            let mut state_guard = state.write().await;
            match state_guard.handle_project_entered(path, context).await {
                Ok(()) => IpcResponse::Ok,
                Err(e) => IpcResponse::Error { 
                    message: format!("Failed to handle project entered: {}", e) 
                }
            }
        }

        IpcMessage::ProjectLeft { path } => {
            let mut state_guard = state.write().await;
            match state_guard.handle_project_left(path).await {
                Ok(()) => IpcResponse::Ok,
                Err(e) => IpcResponse::Error { 
                    message: format!("Failed to handle project left: {}", e) 
                }
            }
        }

        IpcMessage::StartSession { project_path, context } => {
            let mut state_guard = state.write().await;
            
            if let Some(path) = project_path {
                match state_guard.handle_project_entered(path, context).await {
                    Ok(()) => IpcResponse::Ok,
                    Err(e) => IpcResponse::Error { 
                        message: format!("Failed to start session: {}", e) 
                    }
                }
            } else {
                IpcResponse::Error { 
                    message: "Project path required for manual session start".to_string() 
                }
            }
        }

        IpcMessage::StopSession => {
            let mut state_guard = state.write().await;
            match state_guard.stop_session().await {
                Ok(()) => IpcResponse::Ok,
                Err(e) => IpcResponse::Error { 
                    message: format!("Failed to stop session: {}", e) 
                }
            }
        }

        IpcMessage::PauseSession => {
            let mut state_guard = state.write().await;
            match state_guard.pause_session().await {
                Ok(()) => IpcResponse::Ok,
                Err(e) => IpcResponse::Error { 
                    message: format!("Failed to pause session: {}", e) 
                }
            }
        }

        IpcMessage::ResumeSession => {
            let mut state_guard = state.write().await;
            match state_guard.resume_session().await {
                Ok(()) => IpcResponse::Ok,
                Err(e) => IpcResponse::Error { 
                    message: format!("Failed to resume session: {}", e) 
                }
            }
        }

        IpcMessage::GetActiveSession => {
            let state_guard = state.read().await;
            if let Some(session) = &state_guard.active_session {
                let duration = if let Some(paused_at) = session.paused_at {
                    (paused_at - session.start_time - session.total_paused).num_seconds()
                } else {
                    (chrono::Utc::now() - session.start_time - session.total_paused).num_seconds()
                };

                IpcResponse::SessionInfo(vibe::utils::ipc::SessionInfo {
                    id: session.session_id,
                    project_name: session.project_name.clone(),
                    project_path: session.project_path.clone(),
                    start_time: session.start_time,
                    context: session.context.to_string(),
                    duration,
                })
            } else {
                IpcResponse::Error { 
                    message: "No active session".to_string() 
                }
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