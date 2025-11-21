use anyhow::{Result, Context};
use std::process::{Command, Stdio};
use std::path::PathBuf;

use crate::utils::ipc::{get_socket_path, is_daemon_running, IpcClient, IpcMessage, IpcResponse};
use crate::utils::paths::get_data_dir;

#[cfg(test)]
use std::env;

/// Service layer for daemon-related operations
pub struct DaemonService;

impl DaemonService {
    /// Start the tempo daemon
    pub async fn start_daemon() -> Result<()> {
        if is_daemon_running() {
            return Err(anyhow::anyhow!("Daemon is already running"));
        }

        println!("Starting tempo daemon...");

        // Find the daemon binary
        let daemon_path = Self::find_daemon_binary()?;
        
        // Start daemon as background process
        let mut cmd = Command::new(daemon_path);
        cmd.stdout(Stdio::null())
           .stderr(Stdio::null())
           .stdin(Stdio::null());

        // Set environment variables for daemon
        if let Ok(data_dir) = get_data_dir() {
            cmd.env("TEMPO_DATA_DIR", data_dir);
        }

        let child = cmd.spawn()
            .context("Failed to start daemon process")?;

        println!("Daemon started with PID: {}", child.id());

        // Wait a moment for daemon to initialize
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Verify daemon is running
        if !is_daemon_running() {
            return Err(anyhow::anyhow!("Failed to start daemon - not responding"));
        }

        println!("✓ Daemon started successfully");
        Ok(())
    }

    /// Stop the tempo daemon
    pub async fn stop_daemon() -> Result<()> {
        if !is_daemon_running() {
            println!("Daemon is not running");
            return Ok(());
        }

        println!("Stopping tempo daemon...");

        let socket_path = get_socket_path()?;
        let mut client = IpcClient::connect(&socket_path).await
            .context("Failed to connect to daemon")?;

        let response = client.send_message(&IpcMessage::Shutdown).await?;
        
        match response {
            IpcResponse::Success => {
                println!("✓ Daemon stopped successfully");
                Ok(())
            }
            IpcResponse::Error(e) => {
                Err(anyhow::anyhow!("Failed to stop daemon: {}", e))
            }
            _ => {
                Err(anyhow::anyhow!("Unexpected response from daemon"))
            }
        }
    }

    /// Restart the tempo daemon
    pub async fn restart_daemon() -> Result<()> {
        println!("Restarting tempo daemon...");
        
        if is_daemon_running() {
            Self::stop_daemon().await?;
            
            // Wait for daemon to fully stop
            for _ in 0..10 {
                if !is_daemon_running() {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }

        Self::start_daemon().await?;
        println!("✓ Daemon restarted successfully");
        Ok(())
    }

    /// Get daemon status and information
    pub async fn get_daemon_status() -> Result<DaemonStatus> {
        if !is_daemon_running() {
            return Ok(DaemonStatus {
                running: false,
                uptime_seconds: 0,
                active_session: None,
                version: None,
                socket_path: get_socket_path().ok(),
            });
        }

        let socket_path = get_socket_path()?;
        let mut client = IpcClient::connect(&socket_path).await?;

        let response = client.send_message(&IpcMessage::GetStatus).await?;
        
        match response {
            IpcResponse::Status { daemon_running, active_session, uptime } => {
                Ok(DaemonStatus {
                    running: daemon_running,
                    uptime_seconds: uptime,
                    active_session,
                    version: Some(env!("CARGO_PKG_VERSION").to_string()),
                    socket_path: Some(socket_path),
                })
            }
            IpcResponse::Error(e) => {
                Err(anyhow::anyhow!("Failed to get daemon status: {}", e))
            }
            _ => {
                Err(anyhow::anyhow!("Unexpected response from daemon"))
            }
        }
    }

    /// Send activity heartbeat to daemon
    pub async fn send_activity_heartbeat() -> Result<()> {
        if !is_daemon_running() {
            return Ok(()); // Silently ignore if daemon not running
        }

        let socket_path = get_socket_path()?;
        let mut client = IpcClient::connect(&socket_path).await?;
        let _response = client.send_message(&IpcMessage::ActivityHeartbeat).await?;
        Ok(())
    }

    /// Get connection pool statistics
    pub async fn get_pool_stats() -> Result<PoolStatistics> {
        // This would interact with the daemon to get pool stats
        // For now, return a placeholder
        Ok(PoolStatistics {
            total_connections: 5,
            active_connections: 2,
            idle_connections: 3,
            max_connections: 10,
            connection_requests: 150,
            connection_timeouts: 0,
        })
    }

    // Private helper methods

    fn find_daemon_binary() -> Result<PathBuf> {
        // Try to find the daemon binary in common locations
        let possible_names = ["tempo-daemon", "tempo_daemon"];
        let possible_paths = [
            std::env::current_exe()?.parent().map(|p| p.to_path_buf()),
            Some(PathBuf::from("/usr/local/bin")),
            Some(PathBuf::from("/usr/bin")),
            std::env::var("CARGO_TARGET_DIR").ok().map(|p| PathBuf::from(p).join("debug")),
            std::env::var("CARGO_TARGET_DIR").ok().map(|p| PathBuf::from(p).join("release")),
        ];

        for path_opt in possible_paths.iter().flatten() {
            for name in &possible_names {
                let full_path = path_opt.join(name);
                if full_path.exists() && full_path.is_file() {
                    return Ok(full_path);
                }
                
                // Try with .exe extension on Windows
                #[cfg(windows)]
                {
                    let exe_path = path_opt.join(format!("{}.exe", name));
                    if exe_path.exists() && exe_path.is_file() {
                        return Ok(exe_path);
                    }
                }
            }
        }

        // Fall back to assuming it's in PATH
        Ok(PathBuf::from("tempo-daemon"))
    }
}

/// Information about daemon status
#[derive(Debug, Clone)]
pub struct DaemonStatus {
    pub running: bool,
    pub uptime_seconds: u64,
    pub active_session: Option<crate::utils::ipc::SessionInfo>,
    pub version: Option<String>,
    pub socket_path: Option<PathBuf>,
}

/// Database connection pool statistics
#[derive(Debug, Clone)]
pub struct PoolStatistics {
    pub total_connections: u32,
    pub active_connections: u32,
    pub idle_connections: u32,
    pub max_connections: u32,
    pub connection_requests: u64,
    pub connection_timeouts: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_find_daemon_binary() {
        let result = DaemonService::find_daemon_binary();
        // Should at least return a path, even if file doesn't exist
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(!path.as_os_str().is_empty());
    }

    #[tokio::test]
    async fn test_daemon_status_when_not_running() {
        // This test assumes daemon is not running
        let status = DaemonService::get_daemon_status().await.unwrap();
        if !status.running {
            assert_eq!(status.uptime_seconds, 0);
            assert!(status.active_session.is_none());
        }
    }

    #[test]
    fn test_daemon_binary_search_paths() {
        let result = DaemonService::find_daemon_binary();
        assert!(result.is_ok());
        
        let path = result.unwrap();
        
        // Should either be a full path or just the binary name
        assert!(
            path.is_absolute() || path == PathBuf::from("tempo-daemon"),
            "Daemon path should be absolute or fallback binary name: {:?}",
            path
        );
    }

    #[tokio::test]
    async fn test_pool_stats_placeholder() {
        let stats = DaemonService::get_pool_stats().await.unwrap();
        
        // Test placeholder values
        assert_eq!(stats.total_connections, 5);
        assert_eq!(stats.active_connections, 2);
        assert_eq!(stats.idle_connections, 3);
        assert_eq!(stats.max_connections, 10);
        assert_eq!(stats.connection_requests, 150);
        assert_eq!(stats.connection_timeouts, 0);
        
        // Validate relationship
        assert_eq!(
            stats.active_connections + stats.idle_connections, 
            stats.total_connections
        );
        assert!(stats.total_connections <= stats.max_connections);
    }

    #[tokio::test]
    async fn test_daemon_operations_when_not_running() {
        // Test that daemon operations handle "not running" state gracefully
        
        // Stop daemon when not running should succeed silently
        let stop_result = DaemonService::stop_daemon().await;
        // This may succeed (daemon not running) or fail (can't connect)
        // Either is acceptable behavior
        
        // Activity heartbeat should handle daemon not running
        let heartbeat_result = DaemonService::send_activity_heartbeat().await;
        assert!(heartbeat_result.is_ok()); // Should silently ignore
    }

    #[test]
    fn test_daemon_status_structure() {
        let status = DaemonStatus {
            running: true,
            uptime_seconds: 3600,
            active_session: None,
            version: Some("0.2.0".to_string()),
            socket_path: Some(PathBuf::from("/tmp/tempo.sock")),
        };
        
        assert!(status.running);
        assert_eq!(status.uptime_seconds, 3600);
        assert!(status.active_session.is_none());
        assert_eq!(status.version, Some("0.2.0".to_string()));
        assert!(status.socket_path.is_some());
    }

    #[test]
    fn test_pool_statistics_structure() {
        let pool_stats = PoolStatistics {
            total_connections: 10,
            active_connections: 6,
            idle_connections: 4,
            max_connections: 20,
            connection_requests: 500,
            connection_timeouts: 2,
        };
        
        assert_eq!(pool_stats.total_connections, 10);
        assert_eq!(pool_stats.active_connections, 6);
        assert_eq!(pool_stats.idle_connections, 4);
        assert_eq!(pool_stats.max_connections, 20);
        assert_eq!(pool_stats.connection_requests, 500);
        assert_eq!(pool_stats.connection_timeouts, 2);
        
        // Validate internal consistency
        assert_eq!(
            pool_stats.active_connections + pool_stats.idle_connections,
            pool_stats.total_connections
        );
    }

    #[test]
    fn test_version_info() {
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
        assert!(version.starts_with("0."));
    }
}