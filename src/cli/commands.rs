use super::{Cli, Commands, ProjectAction, SessionAction, TagAction, ConfigAction};
use crate::utils::ipc::{IpcClient, IpcMessage, IpcResponse, get_socket_path, is_daemon_running};
use crate::db::queries::{ProjectQueries, SessionQueries, TagQueries};
use crate::db::{Database, get_database_path};
use crate::models::{Project, Tag};
use crate::utils::paths::{canonicalize_path, detect_project_name, get_git_hash, is_git_repository};
use crate::utils::config::{load_config, save_config};
use crate::cli::reports::ReportGenerator;
use anyhow::Result;
use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use chrono::Utc;

// Note: UI imports will be added when needed

pub async fn handle_command(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Start => {
            start_daemon().await
        }
        
        Commands::Stop => {
            stop_daemon().await
        }
        
        Commands::Restart => {
            restart_daemon().await
        }
        
        Commands::Status => {
            status_daemon().await
        }
        
        Commands::Init { name, path, description } => {
            init_project(name, path, description).await
        }
        
        Commands::List { all, tag } => {
            list_projects(all, tag).await
        }
        
        Commands::Report { project, from, to, format, group } => {
            generate_report(project, from, to, format, group).await
        }
        
        Commands::Project { action } => {
            handle_project_action(action).await
        }
        
        Commands::Session { action } => {
            handle_session_action(action).await
        }
        
        Commands::Tag { action } => {
            handle_tag_action(action).await
        }
        
        Commands::Config { action } => {
            handle_config_action(action).await
        }
        
        Commands::Dashboard => {
            println!("TUI Dashboard - Coming soon! Use 'tempo status' for basic info.");
            Ok(())
        }
        
        Commands::Tui => {
            println!("Interactive TUI - Coming soon! Use 'tempo session current' and 'tempo report' for basic info.");
            Ok(())
        }

        Commands::Completions { shell } => {
            Cli::generate_completions(shell);
            Ok(())
        }
    }
}

async fn handle_project_action(action: ProjectAction) -> Result<()> {
    match action {
        ProjectAction::Archive { project } => {
            println!("Archiving project: {}", project);
            // TODO: Implement project archiving
            Ok(())
        }
        
        ProjectAction::Unarchive { project } => {
            println!("Unarchiving project: {}", project);
            // TODO: Implement project unarchiving
            Ok(())
        }
        
        ProjectAction::UpdatePath { project, path } => {
            println!("Updating path for project {} to {:?}", project, path);
            // TODO: Implement project path update
            Ok(())
        }
        
        ProjectAction::AddTag { project, tag } => {
            println!("Adding tag '{}' to project '{}'", tag, project);
            // TODO: Implement adding tag to project
            Ok(())
        }
        
        ProjectAction::RemoveTag { project, tag } => {
            println!("Removing tag '{}' from project '{}'", tag, project);
            // TODO: Implement removing tag from project
            Ok(())
        }
    }
}

async fn handle_session_action(action: SessionAction) -> Result<()> {
    match action {
        SessionAction::Start { project, context } => {
            start_session(project, context).await
        }
        
        SessionAction::Stop => {
            stop_session().await
        }
        
        SessionAction::Pause => {
            pause_session().await
        }
        
        SessionAction::Resume => {
            resume_session().await
        }
        
        SessionAction::Current => {
            current_session().await
        }
        
        SessionAction::List { limit, project } => {
            list_sessions(limit, project).await
        }
        
        SessionAction::Edit { id, start, end, project, reason } => {
            edit_session(id, start, end, project, reason).await
        }
        
        SessionAction::Delete { id, force } => {
            delete_session(id, force).await
        }
    }
}

async fn handle_tag_action(action: TagAction) -> Result<()> {
    match action {
        TagAction::Create { name, color, description } => {
            create_tag(name, color, description).await
        }
        
        TagAction::List => {
            list_tags().await
        }
        
        TagAction::Delete { name } => {
            delete_tag(name).await
        }
    }
}

async fn handle_config_action(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => {
            show_config().await
        }
        
        ConfigAction::Set { key, value } => {
            set_config(key, value).await
        }
        
        ConfigAction::Get { key } => {
            get_config(key).await
        }
        
        ConfigAction::Reset => {
            reset_config().await
        }
    }
}

// Daemon control functions
async fn start_daemon() -> Result<()> {
    if is_daemon_running() {
        println!("Daemon is already running");
        return Ok(());
    }

    println!("Starting tempo daemon...");
    
    let current_exe = std::env::current_exe()?;
    let daemon_exe = current_exe.with_file_name("tempo-daemon");
    
    if !daemon_exe.exists() {
        return Err(anyhow::anyhow!("tempo-daemon executable not found at {:?}", daemon_exe));
    }

    let mut cmd = Command::new(&daemon_exe);
    cmd.stdout(Stdio::null())
       .stderr(Stdio::null())
       .stdin(Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }

    let child = cmd.spawn()?;
    
    // Wait a moment for daemon to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    if is_daemon_running() {
        println!("Daemon started successfully (PID: {})", child.id());
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to start daemon"))
    }
}

async fn stop_daemon() -> Result<()> {
    if !is_daemon_running() {
        println!("Daemon is not running");
        return Ok(());
    }

    println!("Stopping tempo daemon...");
    
    // Try to connect and send shutdown message
    if let Ok(socket_path) = get_socket_path() {
        if let Ok(mut client) = IpcClient::connect(&socket_path).await {
            match client.send_message(&IpcMessage::Shutdown).await {
                Ok(_) => {
                    println!("Daemon stopped successfully");
                    return Ok(());
                }
                Err(e) => {
                    eprintln!("Failed to send shutdown message: {}", e);
                }
            }
        }
    }

    // Fallback: kill via PID file
    if let Ok(Some(pid)) = crate::utils::ipc::read_pid_file() {
        #[cfg(unix)]
        {
            let result = Command::new("kill")
                .arg(pid.to_string())
                .output();
            
            match result {
                Ok(_) => println!("Daemon stopped via kill signal"),
                Err(e) => eprintln!("Failed to kill daemon: {}", e),
            }
        }
        
        #[cfg(windows)]
        {
            let result = Command::new("taskkill")
                .args(&["/PID", &pid.to_string(), "/F"])
                .output();
            
            match result {
                Ok(_) => println!("Daemon stopped via taskkill"),
                Err(e) => eprintln!("Failed to kill daemon: {}", e),
            }
        }
    }

    Ok(())
}

async fn restart_daemon() -> Result<()> {
    println!("Restarting tempo daemon...");
    stop_daemon().await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    start_daemon().await
}

async fn status_daemon() -> Result<()> {
    if !is_daemon_running() {
        print_daemon_not_running();
        return Ok(());
    }

    if let Ok(socket_path) = get_socket_path() {
        match IpcClient::connect(&socket_path).await {
            Ok(mut client) => {
                match client.send_message(&IpcMessage::GetStatus).await {
                    Ok(IpcResponse::Status { daemon_running: _, active_session, uptime }) => {
                        print_daemon_status(uptime, active_session.as_ref());
                    }
                    Ok(IpcResponse::Pong) => {
                        print_daemon_status(0, None); // Minimal response
                    }
                    Ok(other) => {
                        println!("Daemon is running (unexpected response: {:?})", other);
                    }
                    Err(e) => {
                        println!("Daemon is running but not responding: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("Daemon appears to be running but cannot connect: {}", e);
            }
        }
    } else {
        println!("Cannot determine socket path");
    }

    Ok(())
}

// Session control functions
async fn start_session(project: Option<String>, context: Option<String>) -> Result<()> {
    if !is_daemon_running() {
        println!("Daemon is not running. Start it with 'tempo start'");
        return Ok(());
    }

    let project_path = if let Some(proj) = project {
        PathBuf::from(proj)
    } else {
        env::current_dir()?
    };

    let context = context.unwrap_or_else(|| "manual".to_string());

    let socket_path = get_socket_path()?;
    let mut client = IpcClient::connect(&socket_path).await?;
    
    let message = IpcMessage::StartSession { 
        project_path: Some(project_path.clone()), 
        context 
    };
    
    match client.send_message(&message).await {
        Ok(IpcResponse::Ok) => {
            println!("Session started for project at {:?}", project_path);
        }
        Ok(IpcResponse::Error { message }) => {
            println!("Failed to start session: {}", message);
        }
        Ok(other) => {
            println!("Unexpected response: {:?}", other);
        }
        Err(e) => {
            println!("Failed to communicate with daemon: {}", e);
        }
    }

    Ok(())
}

async fn stop_session() -> Result<()> {
    if !is_daemon_running() {
        println!("Daemon is not running");
        return Ok(());
    }

    let socket_path = get_socket_path()?;
    let mut client = IpcClient::connect(&socket_path).await?;
    
    match client.send_message(&IpcMessage::StopSession).await {
        Ok(IpcResponse::Ok) => {
            println!("Session stopped");
        }
        Ok(IpcResponse::Error { message }) => {
            println!("Failed to stop session: {}", message);
        }
        Ok(other) => {
            println!("Unexpected response: {:?}", other);
        }
        Err(e) => {
            println!("Failed to communicate with daemon: {}", e);
        }
    }

    Ok(())
}

async fn pause_session() -> Result<()> {
    if !is_daemon_running() {
        println!("Daemon is not running");
        return Ok(());
    }

    let socket_path = get_socket_path()?;
    let mut client = IpcClient::connect(&socket_path).await?;
    
    match client.send_message(&IpcMessage::PauseSession).await {
        Ok(IpcResponse::Ok) => {
            println!("Session paused");
        }
        Ok(IpcResponse::Error { message }) => {
            println!("Failed to pause session: {}", message);
        }
        Ok(other) => {
            println!("Unexpected response: {:?}", other);
        }
        Err(e) => {
            println!("Failed to communicate with daemon: {}", e);
        }
    }

    Ok(())
}

async fn resume_session() -> Result<()> {
    if !is_daemon_running() {
        println!("Daemon is not running");
        return Ok(());
    }

    let socket_path = get_socket_path()?;
    let mut client = IpcClient::connect(&socket_path).await?;
    
    match client.send_message(&IpcMessage::ResumeSession).await {
        Ok(IpcResponse::Ok) => {
            println!("Session resumed");
        }
        Ok(IpcResponse::Error { message }) => {
            println!("Failed to resume session: {}", message);
        }
        Ok(other) => {
            println!("Unexpected response: {:?}", other);
        }
        Err(e) => {
            println!("Failed to communicate with daemon: {}", e);
        }
    }

    Ok(())
}

async fn current_session() -> Result<()> {
    if !is_daemon_running() {
        print_daemon_not_running();
        return Ok(());
    }

    let socket_path = get_socket_path()?;
    let mut client = IpcClient::connect(&socket_path).await?;
    
    match client.send_message(&IpcMessage::GetActiveSession).await {
        Ok(IpcResponse::SessionInfo(session)) => {
            print_formatted_session(&session)?;
        }
        Ok(IpcResponse::Error { message }) => {
            print_no_active_session(&message);
        }
        Ok(other) => {
            println!("Unexpected response: {:?}", other);
        }
        Err(e) => {
            println!("Failed to communicate with daemon: {}", e);
        }
    }

    Ok(())
}

// Report generation function
async fn generate_report(
    project: Option<String>,
    from: Option<String>,
    to: Option<String>,
    format: Option<String>,
    group: Option<String>,
) -> Result<()> {
    println!("Generating time report...");
    
    let generator = ReportGenerator::new()?;
    let report = generator.generate_report(project, from, to, group)?;
    
    match format.as_deref() {
        Some("csv") => {
            let output_path = PathBuf::from("vibe-report.csv");
            generator.export_csv(&report, &output_path)?;
            println!("Report exported to: {:?}", output_path);
        }
        Some("json") => {
            let output_path = PathBuf::from("vibe-report.json");
            generator.export_json(&report, &output_path)?;
            println!("Report exported to: {:?}", output_path);
        }
        _ => {
            // Print to console with formatted output
            print_formatted_report(&report)?;
        }
    }
    
    Ok(())
}

// Formatted output functions
fn print_formatted_session(session: &crate::utils::ipc::SessionInfo) -> Result<()> {
    // Color scheme definitions
    let context_color = match session.context.as_str() {
        "terminal" => "\x1b[96m",    // Bright cyan
        "ide" => "\x1b[95m",        // Bright magenta
        "linked" => "\x1b[93m",     // Bright yellow
        "manual" => "\x1b[94m",     // Bright blue
        _ => "\x1b[97m",            // Bright white (default)
    };
    
    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m           \x1b[1;37mCurrent Session\x1b[0m               \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[36m│\x1b[0m Status:   \x1b[1;32m●\x1b[0m \x1b[32mActive\x1b[0m                     \x1b[36m│\x1b[0m");
    println!("\x1b[36m│\x1b[0m Project:  \x1b[1;33m{:<25}\x1b[0m \x1b[36m│\x1b[0m", truncate_string(&session.project_name, 25));
    println!("\x1b[36m│\x1b[0m Duration: \x1b[1;32m{:<25}\x1b[0m \x1b[36m│\x1b[0m", format_duration_fancy(session.duration));
    println!("\x1b[36m│\x1b[0m Started:  \x1b[37m{:<25}\x1b[0m \x1b[36m│\x1b[0m", session.start_time.format("%H:%M:%S").to_string());
    println!("\x1b[36m│\x1b[0m Context:  {}{:<25}\x1b[0m \x1b[36m│\x1b[0m", context_color, truncate_string(&session.context, 25));
    println!("\x1b[36m│\x1b[0m Path:     \x1b[2;37m{:<25}\x1b[0m \x1b[36m│\x1b[0m", truncate_string(&session.project_path.to_string_lossy(), 25));
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    Ok(())
}

fn print_formatted_report(report: &crate::cli::reports::TimeReport) -> Result<()> {
    // Helper function to get context color
    let get_context_color = |context: &str| -> &str {
        match context {
            "terminal" => "\x1b[96m",    // Bright cyan
            "ide" => "\x1b[95m",        // Bright magenta
            "linked" => "\x1b[93m",     // Bright yellow
            "manual" => "\x1b[94m",     // Bright blue
            _ => "\x1b[97m",            // Bright white (default)
        }
    };

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m            \x1b[1;37mTime Report\x1b[0m                  \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    
    for (project_name, project_summary) in &report.projects {
        println!("\x1b[36m│\x1b[0m \x1b[1;33m{:<20}\x1b[0m \x1b[1;32m{:>15}\x1b[0m \x1b[36m│\x1b[0m", 
            truncate_string(project_name, 20),
            format_duration_fancy(project_summary.total_duration)
        );
        
        for (context, duration) in &project_summary.contexts {
            let context_color = get_context_color(context);
            println!("\x1b[36m│\x1b[0m   {}{:<15}\x1b[0m \x1b[32m{:>20}\x1b[0m \x1b[36m│\x1b[0m", 
                context_color,
                truncate_string(context, 15),
                format_duration_fancy(*duration)
            );
        }
        println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    }
    
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[36m│\x1b[0m \x1b[1;37mTotal Time:\x1b[0m \x1b[1;32m{:>26}\x1b[0m \x1b[36m│\x1b[0m", format_duration_fancy(report.total_duration));
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    Ok(())
}

fn format_duration_fancy(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        format!("{:<width$}", s, width = max_len)
    } else {
        format!("{:.width$}...", s, width = max_len.saturating_sub(3))
    }
}

// Helper functions for consistent messaging
fn print_daemon_not_running() {
    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m               \x1b[1;37mDaemon Status\x1b[0m               \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[36m│\x1b[0m Status:   \x1b[1;31m●\x1b[0m \x1b[31mOffline\x1b[0m                   \x1b[36m│\x1b[0m");
    println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    println!("\x1b[36m│\x1b[0m \x1b[33mDaemon is not running.\x1b[0m                 \x1b[36m│\x1b[0m");
    println!("\x1b[36m│\x1b[0m \x1b[37mStart it with:\x1b[0m \x1b[96mtempo start\x1b[0m         \x1b[36m│\x1b[0m");
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
}

fn print_no_active_session(message: &str) {
    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m           \x1b[1;37mCurrent Session\x1b[0m               \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[36m│\x1b[0m Status:   \x1b[1;33m○\x1b[0m \x1b[33mIdle\x1b[0m                      \x1b[36m│\x1b[0m");
    println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    println!("\x1b[36m│\x1b[0m \x1b[90m{:<37}\x1b[0m \x1b[36m│\x1b[0m", message);
    println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    println!("\x1b[36m│\x1b[0m \x1b[37mStart tracking:\x1b[0m                       \x1b[36m│\x1b[0m");
    println!("\x1b[36m│\x1b[0m   \x1b[96mtempo session start\x1b[0m                \x1b[36m│\x1b[0m");
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
}

fn print_daemon_status(uptime: u64, active_session: Option<&crate::utils::ipc::SessionInfo>) {
    let uptime_formatted = format_duration_fancy(uptime as i64);
    
    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m               \x1b[1;37mDaemon Status\x1b[0m               \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[36m│\x1b[0m Status:   \x1b[1;32m●\x1b[0m \x1b[32mOnline\x1b[0m                    \x1b[36m│\x1b[0m");
    println!("\x1b[36m│\x1b[0m Uptime:   \x1b[37m{:<25}\x1b[0m \x1b[36m│\x1b[0m", uptime_formatted);
    
    if let Some(session) = active_session {
        let context_color = match session.context.as_str() {
            "terminal" => "\x1b[96m", "ide" => "\x1b[95m", "linked" => "\x1b[93m", 
            "manual" => "\x1b[94m", _ => "\x1b[97m",
        };
        
        println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
        println!("\x1b[36m│\x1b[0m \x1b[1;37mActive Session:\x1b[0m                      \x1b[36m│\x1b[0m");
        println!("\x1b[36m│\x1b[0m   Project: \x1b[1;33m{:<23}\x1b[0m \x1b[36m│\x1b[0m", truncate_string(&session.project_name, 23));
        println!("\x1b[36m│\x1b[0m   Duration: \x1b[1;32m{:<22}\x1b[0m \x1b[36m│\x1b[0m", format_duration_fancy(session.duration));
        println!("\x1b[36m│\x1b[0m   Context: {}{:<23}\x1b[0m \x1b[36m│\x1b[0m", context_color, session.context);
    } else {
        println!("\x1b[36m│\x1b[0m Session:  \x1b[33mNo active session\x1b[0m             \x1b[36m│\x1b[0m");
    }
    
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
}

// Project management functions
async fn init_project(name: Option<String>, path: Option<PathBuf>, description: Option<String>) -> Result<()> {
    let project_path = path.unwrap_or_else(|| env::current_dir().unwrap());
    let canonical_path = canonicalize_path(&project_path)?;
    
    let project_name = name.unwrap_or_else(|| detect_project_name(&canonical_path));
    
    // Initialize database
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;
    
    // Check if project already exists
    if let Some(existing) = ProjectQueries::find_by_path(&db.connection, &canonical_path)? {
        println!("\x1b[33m⚠  Project already exists:\x1b[0m {}", existing.name);
        return Ok(());
    }
    
    // Get git hash if it's a git repository
    let git_hash = if is_git_repository(&canonical_path) {
        get_git_hash(&canonical_path)
    } else {
        None
    };
    
    // Create project
    let mut project = Project::new(project_name.clone(), canonical_path.clone())
        .with_git_hash(git_hash.clone())
        .with_description(description.clone());
    
    // Save to database
    let project_id = ProjectQueries::create(&db.connection, &project)?;
    project.id = Some(project_id);
    
    // Create .tempo marker file
    let marker_path = canonical_path.join(".tempo");
    if !marker_path.exists() {
        std::fs::write(&marker_path, format!("# Tempo time tracking project\nname: {}\n", project_name))?;
    }
    
    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m         \x1b[1;37mProject Initialized\x1b[0m               \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[36m│\x1b[0m Name:     \x1b[1;33m{:<27}\x1b[0m \x1b[36m│\x1b[0m", truncate_string(&project_name, 27));
    println!("\x1b[36m│\x1b[0m Path:     \x1b[37m{:<27}\x1b[0m \x1b[36m│\x1b[0m", truncate_string(&canonical_path.to_string_lossy(), 27));
    if let Some(desc) = &description {
        println!("\x1b[36m│\x1b[0m Desc:     \x1b[2;37m{:<27}\x1b[0m \x1b[36m│\x1b[0m", truncate_string(desc, 27));
    }
    if git_hash.is_some() {
        println!("\x1b[36m│\x1b[0m Type:     \x1b[32mGit Repository\x1b[0m              \x1b[36m│\x1b[0m");
    } else {
        println!("\x1b[36m│\x1b[0m Type:     \x1b[37mStandard Project\x1b[0m             \x1b[36m│\x1b[0m");
    }
    println!("\x1b[36m│\x1b[0m ID:       \x1b[90m{:<27}\x1b[0m \x1b[36m│\x1b[0m", project_id);
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[36m│\x1b[0m \x1b[32m✓ Project created successfully\x1b[0m          \x1b[36m│\x1b[0m");
    println!("\x1b[36m│\x1b[0m \x1b[32m✓ .tempo marker file added\x1b[0m             \x1b[36m│\x1b[0m");
    println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    println!("\x1b[36m│\x1b[0m \x1b[37mStart tracking:\x1b[0m                       \x1b[36m│\x1b[0m");
    println!("\x1b[36m│\x1b[0m   \x1b[96mtempo session start\x1b[0m                 \x1b[36m│\x1b[0m");
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    
    Ok(())
}

async fn list_projects(include_archived: bool, tag_filter: Option<String>) -> Result<()> {
    // Initialize database
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;
    
    // Get projects
    let projects = ProjectQueries::list_all(&db.connection, include_archived)?;
    
    if projects.is_empty() {
        println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
        println!("\x1b[36m│\x1b[0m              \x1b[1;37mNo Projects\x1b[0m                 \x1b[36m│\x1b[0m");
        println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
        println!("\x1b[36m│\x1b[0m No projects found.                      \x1b[36m│\x1b[0m");
        println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
        println!("\x1b[36m│\x1b[0m \x1b[37mCreate a project:\x1b[0m                      \x1b[36m│\x1b[0m");
        println!("\x1b[36m│\x1b[0m   \x1b[96mtempo init [project-name]\x1b[0m           \x1b[36m│\x1b[0m");
        println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
        return Ok(());
    }
    
    // Filter by tag if specified
    let filtered_projects = if let Some(_tag) = tag_filter {
        // TODO: Implement tag filtering when project-tag associations are implemented
        projects
    } else {
        projects
    };
    
    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m              \x1b[1;37mProjects\x1b[0m                    \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    
    for project in &filtered_projects {
        let status_icon = if project.is_archived { "📦" } else { "📁" };
        let status_color = if project.is_archived { "\x1b[90m" } else { "\x1b[37m" };
        let git_indicator = if project.git_hash.is_some() { " (git)" } else { "" };
        
        println!("\x1b[36m│\x1b[0m {} {}{:<25}\x1b[0m \x1b[36m│\x1b[0m", 
            status_icon,
            status_color,
            format!("{}{}", truncate_string(&project.name, 20), git_indicator)
        );
        
        if let Some(description) = &project.description {
            println!("\x1b[36m│\x1b[0m   \x1b[2;37m{:<35}\x1b[0m \x1b[36m│\x1b[0m", truncate_string(description, 35));
        }
        
        let path_display = project.path.to_string_lossy();
        if path_display.len() > 35 {
            let home_dir = dirs::home_dir();
            let display_path = if let Some(home) = home_dir {
                if let Ok(stripped) = project.path.strip_prefix(&home) {
                    format!("~/{}", stripped.display())
                } else {
                    path_display.to_string()
                }
            } else {
                path_display.to_string()
            };
            println!("\x1b[36m│\x1b[0m   \x1b[90m{:<35}\x1b[0m \x1b[36m│\x1b[0m", truncate_string(&display_path, 35));
        } else {
            println!("\x1b[36m│\x1b[0m   \x1b[90m{:<35}\x1b[0m \x1b[36m│\x1b[0m", truncate_string(&path_display, 35));
        }
        
        println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    }
    
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[36m│\x1b[0m \x1b[1;37mTotal:\x1b[0m {:<30} \x1b[36m│\x1b[0m", 
        format!("{} projects", filtered_projects.len())
    );
    if include_archived {
        let active_count = filtered_projects.iter().filter(|p| !p.is_archived).count();
        let archived_count = filtered_projects.iter().filter(|p| p.is_archived).count();
        println!("\x1b[36m│\x1b[0m \x1b[37mActive:\x1b[0m {:<15} \x1b[90mArchived:\x1b[0m {:<8} \x1b[36m│\x1b[0m", 
            active_count, archived_count
        );
    }
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    
    Ok(())
}

// Tag management functions
async fn create_tag(name: String, color: Option<String>, description: Option<String>) -> Result<()> {
    // Initialize database
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;
    
    // Create tag
    let mut tag = Tag::new(name.clone());
    if let Some(c) = color {
        tag = tag.with_color(c);
    }
    if let Some(d) = description {
        tag = tag.with_description(d);
    }
    
    // Validate tag
    tag.validate()?;
    
    // Check if tag already exists
    let existing_tags = TagQueries::list_all(&db.connection)?;
    if existing_tags.iter().any(|t| t.name == tag.name) {
        println!("\x1b[33m⚠  Tag already exists:\x1b[0m {}", tag.name);
        return Ok(());
    }
    
    // Save to database
    let tag_id = TagQueries::create(&db.connection, &tag)?;
    
    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m           \x1b[1;37mTag Created\x1b[0m                   \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[36m│\x1b[0m Name:     \x1b[1;33m{:<27}\x1b[0m \x1b[36m│\x1b[0m", truncate_string(&tag.name, 27));
    if let Some(color_val) = &tag.color {
        println!("\x1b[36m│\x1b[0m Color:    \x1b[37m{:<27}\x1b[0m \x1b[36m│\x1b[0m", truncate_string(color_val, 27));
    }
    if let Some(desc) = &tag.description {
        println!("\x1b[36m│\x1b[0m Desc:     \x1b[2;37m{:<27}\x1b[0m \x1b[36m│\x1b[0m", truncate_string(desc, 27));
    }
    println!("\x1b[36m│\x1b[0m ID:       \x1b[90m{:<27}\x1b[0m \x1b[36m│\x1b[0m", tag_id);
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[36m│\x1b[0m \x1b[32m✓ Tag created successfully\x1b[0m             \x1b[36m│\x1b[0m");
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    
    Ok(())
}

async fn list_tags() -> Result<()> {
    // Initialize database
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;
    
    // Get tags
    let tags = TagQueries::list_all(&db.connection)?;
    
    if tags.is_empty() {
        println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
        println!("\x1b[36m│\x1b[0m               \x1b[1;37mNo Tags\x1b[0m                    \x1b[36m│\x1b[0m");
        println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
        println!("\x1b[36m│\x1b[0m No tags found.                          \x1b[36m│\x1b[0m");
        println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
        println!("\x1b[36m│\x1b[0m \x1b[37mCreate a tag:\x1b[0m                          \x1b[36m│\x1b[0m");
        println!("\x1b[36m│\x1b[0m   \x1b[96mtempo tag create <name>\x1b[0m             \x1b[36m│\x1b[0m");
        println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
        return Ok(());
    }
    
    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m                \x1b[1;37mTags\x1b[0m                      \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    
    for tag in &tags {
        let color_indicator = if let Some(color) = &tag.color {
            format!(" ({})", color)
        } else {
            "".to_string()
        };
        
        println!("\x1b[36m│\x1b[0m 🏷️  \x1b[1;33m{:<30}\x1b[0m \x1b[36m│\x1b[0m", 
            format!("{}{}", truncate_string(&tag.name, 25), color_indicator)
        );
        
        if let Some(description) = &tag.description {
            println!("\x1b[36m│\x1b[0m     \x1b[2;37m{:<33}\x1b[0m \x1b[36m│\x1b[0m", truncate_string(description, 33));
        }
        
        println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    }
    
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[36m│\x1b[0m \x1b[1;37mTotal:\x1b[0m {:<30} \x1b[36m│\x1b[0m", 
        format!("{} tags", tags.len())
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    
    Ok(())
}

async fn delete_tag(name: String) -> Result<()> {
    println!("\x1b[33m⚠  Tag deletion not yet implemented:\x1b[0m {}", name);
    println!("This requires implementing delete functionality in TagQueries.");
    Ok(())
}

// Configuration management functions
async fn show_config() -> Result<()> {
    let config = load_config()?;
    
    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m           \x1b[1;37mConfiguration\x1b[0m                  \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[36m│\x1b[0m idle_timeout_minutes:  \x1b[33m{:<16}\x1b[0m \x1b[36m│\x1b[0m", config.idle_timeout_minutes);
    println!("\x1b[36m│\x1b[0m auto_pause_enabled:    \x1b[33m{:<16}\x1b[0m \x1b[36m│\x1b[0m", config.auto_pause_enabled);
    println!("\x1b[36m│\x1b[0m default_context:       \x1b[33m{:<16}\x1b[0m \x1b[36m│\x1b[0m", config.default_context);
    println!("\x1b[36m│\x1b[0m max_session_hours:     \x1b[33m{:<16}\x1b[0m \x1b[36m│\x1b[0m", config.max_session_hours);
    println!("\x1b[36m│\x1b[0m backup_enabled:        \x1b[33m{:<16}\x1b[0m \x1b[36m│\x1b[0m", config.backup_enabled);
    println!("\x1b[36m│\x1b[0m log_level:             \x1b[33m{:<16}\x1b[0m \x1b[36m│\x1b[0m", config.log_level);
    
    if !config.custom_settings.is_empty() {
        println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
        println!("\x1b[36m│\x1b[0m \x1b[1;37mCustom Settings:\x1b[0m                      \x1b[36m│\x1b[0m");
        for (key, value) in &config.custom_settings {
            println!("\x1b[36m│\x1b[0m {:<20} \x1b[33m{:<16}\x1b[0m \x1b[36m│\x1b[0m", 
                truncate_string(key, 20), 
                truncate_string(value, 16)
            );
        }
    }
    
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    
    Ok(())
}

async fn get_config(key: String) -> Result<()> {
    let config = load_config()?;
    
    let value = match key.as_str() {
        "idle_timeout_minutes" => Some(config.idle_timeout_minutes.to_string()),
        "auto_pause_enabled" => Some(config.auto_pause_enabled.to_string()),
        "default_context" => Some(config.default_context),
        "max_session_hours" => Some(config.max_session_hours.to_string()),
        "backup_enabled" => Some(config.backup_enabled.to_string()),
        "log_level" => Some(config.log_level),
        _ => config.custom_settings.get(&key).cloned(),
    };
    
    match value {
        Some(val) => {
            println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
            println!("\x1b[36m│\x1b[0m          \x1b[1;37mConfiguration Value\x1b[0m             \x1b[36m│\x1b[0m");
            println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
            println!("\x1b[36m│\x1b[0m {:<20} \x1b[33m{:<16}\x1b[0m \x1b[36m│\x1b[0m", 
                truncate_string(&key, 20), 
                truncate_string(&val, 16)
            );
            println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
        }
        None => {
            println!("\x1b[31m✗ Configuration key not found:\x1b[0m {}", key);
        }
    }
    
    Ok(())
}

async fn set_config(key: String, value: String) -> Result<()> {
    let mut config = load_config()?;
    
    let display_value = value.clone(); // Clone for display purposes
    
    match key.as_str() {
        "idle_timeout_minutes" => {
            config.idle_timeout_minutes = value.parse()?;
        }
        "auto_pause_enabled" => {
            config.auto_pause_enabled = value.parse()?;
        }
        "default_context" => {
            config.default_context = value;
        }
        "max_session_hours" => {
            config.max_session_hours = value.parse()?;
        }
        "backup_enabled" => {
            config.backup_enabled = value.parse()?;
        }
        "log_level" => {
            config.log_level = value;
        }
        _ => {
            config.set_custom(key.clone(), value);
        }
    }
    
    config.validate()?;
    save_config(&config)?;
    
    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m        \x1b[1;37mConfiguration Updated\x1b[0m             \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[36m│\x1b[0m {:<20} \x1b[32m{:<16}\x1b[0m \x1b[36m│\x1b[0m", 
        truncate_string(&key, 20), 
        truncate_string(&display_value, 16)
    );
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[36m│\x1b[0m \x1b[32m✓ Configuration saved successfully\x1b[0m      \x1b[36m│\x1b[0m");
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    
    Ok(())
}

async fn reset_config() -> Result<()> {
    let default_config = crate::models::Config::default();
    save_config(&default_config)?;
    
    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m         \x1b[1;37mConfiguration Reset\x1b[0m              \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[36m│\x1b[0m \x1b[32m✓ Configuration reset to defaults\x1b[0m       \x1b[36m│\x1b[0m");
    println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    println!("\x1b[36m│\x1b[0m \x1b[37mView current config:\x1b[0m                   \x1b[36m│\x1b[0m");
    println!("\x1b[36m│\x1b[0m   \x1b[96mtempo config show\x1b[0m                   \x1b[36m│\x1b[0m");
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    
    Ok(())
}

// Session management functions
async fn list_sessions(limit: Option<usize>, project_filter: Option<String>) -> Result<()> {
    // Initialize database
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;
    
    let session_limit = limit.unwrap_or(10);
    let sessions = SessionQueries::list_recent(&db.connection, session_limit)?;
    
    if sessions.is_empty() {
        println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
        println!("\x1b[36m│\x1b[0m             \x1b[1;37mNo Sessions\x1b[0m                  \x1b[36m│\x1b[0m");
        println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
        println!("\x1b[36m│\x1b[0m No sessions found.                      \x1b[36m│\x1b[0m");
        println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
        println!("\x1b[36m│\x1b[0m \x1b[37mStart a session:\x1b[0m                      \x1b[36m│\x1b[0m");
        println!("\x1b[36m│\x1b[0m   \x1b[96mtempo session start\x1b[0m                 \x1b[36m│\x1b[0m");
        println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
        return Ok(());
    }
    
    // Filter by project if specified
    let filtered_sessions = if let Some(_project) = project_filter {
        // TODO: Implement project filtering when we have project relationships
        sessions
    } else {
        sessions
    };
    
    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m         \x1b[1;37mRecent Sessions\x1b[0m                 \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    
    for session in &filtered_sessions {
        let status_icon = if session.end_time.is_some() { "✅" } else { "🔄" };
        let duration = if let Some(end) = session.end_time {
            (end - session.start_time).num_seconds() - session.paused_duration.num_seconds()
        } else {
            (Utc::now() - session.start_time).num_seconds() - session.paused_duration.num_seconds()
        };
        
        let context_color = match session.context.to_string().as_str() {
            "terminal" => "\x1b[96m",
            "ide" => "\x1b[95m", 
            "linked" => "\x1b[93m",
            "manual" => "\x1b[94m",
            _ => "\x1b[97m",
        };
        
        println!("\x1b[36m│\x1b[0m {} \x1b[1;37m{:<32}\x1b[0m \x1b[36m│\x1b[0m", 
            status_icon,
            format!("Session {}", session.id.unwrap_or(0))
        );
        println!("\x1b[36m│\x1b[0m    Duration: \x1b[32m{:<24}\x1b[0m \x1b[36m│\x1b[0m", format_duration_fancy(duration));
        println!("\x1b[36m│\x1b[0m    Context:  {}{:<24}\x1b[0m \x1b[36m│\x1b[0m", 
            context_color, 
            session.context.to_string()
        );
        println!("\x1b[36m│\x1b[0m    Started:  \x1b[37m{:<24}\x1b[0m \x1b[36m│\x1b[0m", 
            session.start_time.format("%Y-%m-%d %H:%M:%S").to_string()
        );
        println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    }
    
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[36m│\x1b[0m \x1b[1;37mShowing:\x1b[0m {:<28} \x1b[36m│\x1b[0m", 
        format!("{} recent sessions", filtered_sessions.len())
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    
    Ok(())
}

async fn edit_session(id: i64, start: Option<String>, end: Option<String>, project: Option<String>, reason: Option<String>) -> Result<()> {
    println!("\x1b[33m⚠  Session editing not yet implemented:\x1b[0m session {}", id);
    println!("This requires implementing update functionality in SessionQueries.");
    if let Some(s) = start {
        println!("  New start: {}", s);
    }
    if let Some(e) = end {
        println!("  New end: {}", e);
    }
    if let Some(p) = project {
        println!("  New project: {}", p);
    }
    if let Some(r) = reason {
        println!("  Reason: {}", r);
    }
    Ok(())
}

async fn delete_session(id: i64, force: bool) -> Result<()> {
    println!("\x1b[33m⚠  Session deletion not yet implemented:\x1b[0m session {}", id);
    println!("This requires implementing delete functionality in SessionQueries.");
    if force {
        println!("  Force deletion was requested");
    }
    Ok(())
}

// TUI functions will be implemented later when the library integration is fully working