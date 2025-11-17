use super::{Cli, Commands, ProjectAction, SessionAction, TagAction, ConfigAction, Shell};
use crate::utils::ipc::{IpcClient, IpcMessage, IpcResponse, get_socket_path, is_daemon_running};
use crate::db::queries::{ProjectQueries, SessionQueries, TagQueries};
use crate::cli::reports::{ReportGenerator, print_report};
use anyhow::Result;
use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};

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
            let project_path = path.unwrap_or_else(|| env::current_dir().unwrap());
            let project_name = name.unwrap_or_else(|| {
                project_path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            });
            
            println!("Initializing project '{}' at {:?}", project_name, project_path);
            if let Some(desc) = description {
                println!("Description: {}", desc);
            }
            
            // TODO: Implement project initialization
            Ok(())
        }
        
        Commands::List { all, tag } => {
            println!("Listing projects...");
            if all {
                println!("Including archived projects");
            }
            if let Some(tag_filter) = tag {
                println!("Filtering by tag: {}", tag_filter);
            }
            
            // TODO: Implement project listing
            Ok(())
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
            println!("TUI Dashboard - Coming soon! Use 'vibe status' for basic info.");
            Ok(())
        }
        
        Commands::Tui => {
            println!("Interactive TUI - Coming soon! Use 'vibe session current' and 'vibe report' for basic info.");
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
            println!("Listing sessions...");
            if let Some(lim) = limit {
                println!("Limit: {}", lim);
            }
            if let Some(proj) = project {
                println!("Project filter: {}", proj);
            }
            // TODO: Implement session listing
            Ok(())
        }
        
        SessionAction::Edit { id, start, end, project, reason } => {
            println!("Editing session {}...", id);
            if let Some(s) = start {
                println!("New start time: {}", s);
            }
            if let Some(e) = end {
                println!("New end time: {}", e);
            }
            if let Some(p) = project {
                println!("New project: {}", p);
            }
            if let Some(r) = reason {
                println!("Reason: {}", r);
            }
            // TODO: Implement session editing
            Ok(())
        }
        
        SessionAction::Delete { id, force } => {
            println!("Deleting session {}...", id);
            if force {
                println!("Force deletion enabled");
            }
            // TODO: Implement session deletion
            Ok(())
        }
    }
}

async fn handle_tag_action(action: TagAction) -> Result<()> {
    match action {
        TagAction::Create { name, color, description } => {
            println!("Creating tag '{}'", name);
            if let Some(c) = color {
                println!("Color: {}", c);
            }
            if let Some(d) = description {
                println!("Description: {}", d);
            }
            // TODO: Implement tag creation
            Ok(())
        }
        
        TagAction::List => {
            println!("Listing all tags...");
            // TODO: Implement tag listing
            Ok(())
        }
        
        TagAction::Delete { name } => {
            println!("Deleting tag '{}'", name);
            // TODO: Implement tag deletion
            Ok(())
        }
    }
}

async fn handle_config_action(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => {
            println!("Current configuration:");
            // TODO: Implement config display
            Ok(())
        }
        
        ConfigAction::Set { key, value } => {
            println!("Setting {} = {}", key, value);
            // TODO: Implement config setting
            Ok(())
        }
        
        ConfigAction::Get { key } => {
            println!("Getting configuration for key: {}", key);
            // TODO: Implement config getting
            Ok(())
        }
        
        ConfigAction::Reset => {
            println!("Resetting configuration to defaults...");
            // TODO: Implement config reset
            Ok(())
        }
    }
}

// Daemon control functions
async fn start_daemon() -> Result<()> {
    if is_daemon_running() {
        println!("Daemon is already running");
        return Ok(());
    }

    println!("Starting vibe daemon...");
    
    let current_exe = std::env::current_exe()?;
    let daemon_exe = current_exe.with_file_name("vibe-daemon");
    
    if !daemon_exe.exists() {
        return Err(anyhow::anyhow!("vibe-daemon executable not found at {:?}", daemon_exe));
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

    println!("Stopping vibe daemon...");
    
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
    println!("Restarting vibe daemon...");
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
        println!("Daemon is not running. Start it with 'vibe start'");
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
    println!("\x1b[36m│\x1b[0m \x1b[37mStart it with:\x1b[0m \x1b[96mvibe start\x1b[0m          \x1b[36m│\x1b[0m");
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
    println!("\x1b[36m│\x1b[0m   \x1b[96mvibe session start\x1b[0m                  \x1b[36m│\x1b[0m");
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

// TUI functions will be implemented later when the library integration is fully working