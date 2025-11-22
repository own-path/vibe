use anyhow::Result;
use log::{error, info, warn};
use std::sync::Arc;
use tokio::signal;
use tokio::sync::RwLock;

mod project_cache;
mod server;
mod state;

use server::DaemonServer;
use state::{start_idle_checker, DaemonState};
use tempo_cli::db::{initialize_database, initialize_pool};
use tempo_cli::utils::ipc::{get_socket_path, remove_pid_file, write_pid_file};
use tempo_cli::utils::paths::get_data_dir;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    info!("Starting tempo daemon...");

    // Write PID file
    if let Err(e) = write_pid_file() {
        error!("Failed to write PID file: {}", e);
        return Err(e);
    }

    // Ensure cleanup on exit
    let _cleanup_guard = CleanupGuard;

    // Initialize database pool
    if let Err(e) = initialize_pool() {
        error!("Failed to initialize database pool: {}", e);
        return Err(e);
    }

    // Get a connection for daemon state initialization
    let db_path = get_data_dir()?.join("data.db");
    let db = match initialize_database(&db_path) {
        Ok(db) => db,
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            return Err(e);
        }
    };

    // Initialize daemon state
    let mut daemon_state = DaemonState::new(db);
    if let Err(e) = daemon_state.initialize().await {
        error!("Failed to initialize daemon state: {}", e);
        return Err(e);
    }

    let shared_state = Arc::new(RwLock::new(daemon_state));

    // Start idle checker task
    let idle_checker_state = shared_state.clone();
    tokio::spawn(async move {
        start_idle_checker(idle_checker_state).await;
    });

    // Start IPC server
    let socket_path = get_socket_path()?;
    let server = match DaemonServer::new(socket_path, shared_state.clone()) {
        Ok(server) => server,
        Err(e) => {
            error!("Failed to create IPC server: {}", e);
            return Err(e);
        }
    };

    info!("Tempo daemon started successfully");

    // Handle shutdown signals
    let server_task = tokio::spawn(async move {
        if let Err(e) = server.run().await {
            error!("IPC server error: {}", e);
        }
    });

    let shutdown_signal = tokio::spawn(async {
        signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        info!("Received CTRL+C, shutting down...");
    });

    tokio::select! {
        _ = server_task => {
            warn!("IPC server exited");
        }
        _ = shutdown_signal => {
            info!("Graceful shutdown initiated");

            // Stop any active sessions
            let mut state_guard = shared_state.write().await;
            if let Err(e) = state_guard.stop_session().await {
                warn!("Error stopping session during shutdown: {}", e);
            }
        }
    }

    info!("Tempo daemon stopped");
    Ok(())
}

struct CleanupGuard;

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        if let Err(e) = remove_pid_file() {
            eprintln!("Failed to remove PID file: {}", e);
        }

        // Remove socket file
        if let Ok(socket_path) = get_socket_path() {
            if socket_path.exists() {
                if let Err(e) = std::fs::remove_file(&socket_path) {
                    eprintln!("Failed to remove socket file: {}", e);
                }
            }
        }
    }
}
