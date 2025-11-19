use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcMessage {
    // Project tracking
    ProjectEntered { path: PathBuf, context: String },
    ProjectLeft { path: PathBuf },
    
    // Session control
    StartSession { project_path: Option<PathBuf>, context: String },
    StopSession,
    PauseSession,
    ResumeSession,
    
    // Status queries
    GetStatus,
    GetActiveSession,
    GetProject(i64),
    GetDailyStats(chrono::NaiveDate),
    GetSessionMetrics(i64),
    
    // Real-time monitoring
    SubscribeToUpdates,
    UnsubscribeFromUpdates,
    ActivityHeartbeat,
    
    // Project switching
    SwitchProject(i64),
    
    // Daemon control
    Ping,
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcResponse {
    Ok,
    Success,
    Error(String),
    Status { 
        daemon_running: bool, 
        active_session: Option<SessionInfo>,
        uptime: u64,
    },
    ActiveSession(Option<crate::models::Session>),
    Project(Option<crate::models::Project>),
    DailyStats {
        sessions_count: i64,
        total_seconds: i64,
        avg_seconds: i64,
    },
    SessionMetrics(SessionMetrics),
    SessionInfo(SessionInfo),
    SubscriptionConfirmed,
    ActivityUpdate(ActivityUpdate),
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: i64,
    pub project_name: String,
    pub project_path: PathBuf,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub context: String,
    pub duration: i64, // seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub session_id: i64,
    pub active_duration: i64, // seconds
    pub total_duration: i64,  // seconds
    pub paused_duration: i64, // seconds
    pub activity_score: f64,  // 0.0 to 1.0
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub productivity_rating: Option<u8>, // 1-5 scale
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityUpdate {
    pub session_id: i64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: ActivityEventType,
    pub duration_delta: i64, // Change in active time since last update
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityEventType {
    SessionStarted,
    SessionPaused,
    SessionResumed,
    SessionEnded,
    ActivityDetected,
    IdleDetected,
    MilestoneReached { milestone: String },
}

pub struct IpcServer {
    listener: UnixListener,
}

impl IpcServer {
    pub fn new(socket_path: &PathBuf) -> Result<Self> {
        // Remove existing socket file if it exists
        if socket_path.exists() {
            std::fs::remove_file(socket_path)?;
        }

        // Create parent directory if it doesn't exist
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let listener = UnixListener::bind(socket_path)?;
        
        // Set socket permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(socket_path, perms)?;
        }

        Ok(Self { listener })
    }

    pub async fn accept(&self) -> Result<(UnixStream, tokio::net::unix::SocketAddr)> {
        Ok(self.listener.accept().await?)
    }
}

pub struct IpcClient {
    pub stream: Option<UnixStream>,
}

impl IpcClient {
    pub async fn connect(socket_path: &PathBuf) -> Result<Self> {
        let stream = UnixStream::connect(socket_path).await?;
        Ok(Self { stream: Some(stream) })
    }

    pub fn new() -> Result<Self> {
        Ok(Self { stream: None })
    }

    pub async fn send_message(&mut self, message: &IpcMessage) -> Result<IpcResponse> {
        let stream = self.stream.as_mut().ok_or_else(|| anyhow::anyhow!("No connection established"))?;
        
        // Serialize message
        let serialized = serde_json::to_vec(message)?;
        let len = serialized.len() as u32;

        // Send length prefix + message
        stream.write_u32(len).await?;
        stream.write_all(&serialized).await?;

        // Read response
        let response_len = stream.read_u32().await?;
        let mut response_bytes = vec![0; response_len as usize];
        stream.read_exact(&mut response_bytes).await?;

        // Deserialize response
        let response: IpcResponse = serde_json::from_slice(&response_bytes)?;
        Ok(response)
    }
}

pub async fn read_ipc_message(stream: &mut UnixStream) -> Result<IpcMessage> {
    let len = stream.read_u32().await?;
    let mut buffer = vec![0; len as usize];
    stream.read_exact(&mut buffer).await?;
    
    let message: IpcMessage = serde_json::from_slice(&buffer)?;
    Ok(message)
}

pub async fn write_ipc_response(stream: &mut UnixStream, response: &IpcResponse) -> Result<()> {
    let serialized = serde_json::to_vec(response)?;
    let len = serialized.len() as u32;
    
    stream.write_u32(len).await?;
    stream.write_all(&serialized).await?;
    
    Ok(())
}

pub fn get_socket_path() -> Result<PathBuf> {
    let data_dir = crate::utils::paths::get_data_dir()?;
    Ok(data_dir.join("daemon.sock"))
}

pub fn get_pid_file_path() -> Result<PathBuf> {
    let data_dir = crate::utils::paths::get_data_dir()?;
    Ok(data_dir.join("daemon.pid"))
}

pub fn write_pid_file() -> Result<()> {
    let pid_path = get_pid_file_path()?;
    let pid = std::process::id();
    std::fs::write(pid_path, pid.to_string())?;
    Ok(())
}

pub fn read_pid_file() -> Result<Option<u32>> {
    let pid_path = get_pid_file_path()?;
    if !pid_path.exists() {
        return Ok(None);
    }
    
    let contents = std::fs::read_to_string(pid_path)?;
    let pid = contents.trim().parse::<u32>()?;
    Ok(Some(pid))
}

pub fn remove_pid_file() -> Result<()> {
    let pid_path = get_pid_file_path()?;
    if pid_path.exists() {
        std::fs::remove_file(pid_path)?;
    }
    Ok(())
}

pub fn is_daemon_running() -> bool {
    if let Ok(Some(pid)) = read_pid_file() {
        // Check if process is actually running
        #[cfg(unix)]
        {
            use std::process::Command;
            if let Ok(output) = Command::new("kill")
                .arg("-0")
                .arg(pid.to_string())
                .output()
            {
                return output.status.success();
            }
        }
        
        #[cfg(windows)]
        {
            use std::process::Command;
            if let Ok(output) = Command::new("tasklist")
                .arg("/FI")
                .arg(format!("PID eq {}", pid))
                .arg("/NH")
                .output()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                return output_str.contains(&pid.to_string());
            }
        }
    }
    
    false
}