use super::{
    BranchAction, CalendarAction, Cli, ClientAction, Commands, ConfigAction, EstimateAction,
    GoalAction, IssueAction, ProjectAction, SessionAction, TagAction, TemplateAction,
    WorkspaceAction,
};
use crate::cli::reports::ReportGenerator;
use crate::db::advanced_queries::{
    GitBranchQueries, GoalQueries, InsightQueries, TemplateQueries, TimeEstimateQueries,
    WorkspaceQueries,
};
use crate::db::queries::{ProjectQueries, SessionEditQueries, SessionQueries, TagQueries};
use crate::db::{get_connection, get_database_path, get_pool_stats, Database};
use crate::models::{Goal, Project, ProjectTemplate, Tag, TimeEstimate, Workspace};
use crate::utils::config::{load_config, save_config};
use crate::utils::ipc::{get_socket_path, is_daemon_running, IpcClient, IpcMessage, IpcResponse};
use crate::utils::paths::{
    canonicalize_path, detect_project_name, get_git_hash, is_git_repository, validate_project_path,
};
use crate::utils::validation::{validate_project_description, validate_project_name};
use anyhow::{Context, Result};
use chrono::{TimeZone, Utc};
use serde::Deserialize;
use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::ui::dashboard::Dashboard;
use crate::ui::history::SessionHistoryBrowser;
use crate::ui::timer::InteractiveTimer;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tokio::runtime::Handle;

pub async fn handle_command(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Start => start_daemon().await,

        Commands::Stop => stop_daemon().await,

        Commands::Restart => restart_daemon().await,

        Commands::Status => status_daemon().await,

        Commands::Init {
            name,
            path,
            description,
        } => init_project(name, path, description).await,

        Commands::List { all, tag } => list_projects(all, tag).await,

        Commands::Report {
            project,
            from,
            to,
            format,
            group,
        } => generate_report(project, from, to, format, group).await,

        Commands::Project { action } => handle_project_action(action).await,

        Commands::Session { action } => handle_session_action(action).await,

        Commands::Tag { action } => handle_tag_action(action).await,

        Commands::Config { action } => handle_config_action(action).await,

        Commands::Dashboard => launch_dashboard().await,

        Commands::Tui => launch_dashboard().await,

        Commands::Timer => launch_timer().await,

        Commands::History => launch_history().await,

        Commands::Goal { action } => handle_goal_action(action).await,

        Commands::Insights { period, project } => show_insights(period, project).await,

        Commands::Summary { period, from } => show_summary(period, from).await,

        Commands::Compare { projects, from, to } => compare_projects(projects, from, to).await,

        Commands::PoolStats => show_pool_stats().await,

        Commands::Estimate { action } => handle_estimate_action(action).await,

        Commands::Branch { action } => handle_branch_action(action).await,

        Commands::Template { action } => handle_template_action(action).await,

        Commands::Workspace { action } => handle_workspace_action(action).await,

        Commands::Calendar { action } => handle_calendar_action(action).await,

        Commands::Issue { action } => handle_issue_action(action).await,

        Commands::Client { action } => handle_client_action(action).await,

        Commands::Update {
            check,
            force,
            verbose,
        } => handle_update(check, force, verbose).await,

        Commands::Completions { shell } => {
            Cli::generate_completions(shell);
            Ok(())
        }
    }
}

async fn handle_project_action(action: ProjectAction) -> Result<()> {
    match action {
        ProjectAction::Archive { project } => archive_project(project).await,

        ProjectAction::Unarchive { project } => unarchive_project(project).await,

        ProjectAction::UpdatePath { project, path } => update_project_path(project, path).await,

        ProjectAction::AddTag { project, tag } => add_tag_to_project(project, tag).await,

        ProjectAction::RemoveTag { project, tag } => remove_tag_from_project(project, tag).await,
    }
}

async fn handle_session_action(action: SessionAction) -> Result<()> {
    match action {
        SessionAction::Start { project, context } => start_session(project, context).await,

        SessionAction::Stop => stop_session().await,

        SessionAction::Pause => pause_session().await,

        SessionAction::Resume => resume_session().await,

        SessionAction::Current => current_session().await,

        SessionAction::List { limit, project } => list_sessions(limit, project).await,

        SessionAction::Edit {
            id,
            start,
            end,
            project,
            reason,
        } => edit_session(id, start, end, project, reason).await,

        SessionAction::Delete { id, force } => delete_session(id, force).await,

        SessionAction::Merge {
            session_ids,
            project,
            notes,
        } => merge_sessions(session_ids, project, notes).await,

        SessionAction::Split {
            session_id,
            split_times,
            notes,
        } => split_session(session_id, split_times, notes).await,
    }
}

async fn handle_tag_action(action: TagAction) -> Result<()> {
    match action {
        TagAction::Create {
            name,
            color,
            description,
        } => create_tag(name, color, description).await,

        TagAction::List => list_tags().await,

        TagAction::Delete { name } => delete_tag(name).await,
    }
}

async fn handle_config_action(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Show => show_config().await,

        ConfigAction::Set { key, value } => set_config(key, value).await,

        ConfigAction::Get { key } => get_config(key).await,

        ConfigAction::Reset => reset_config().await,
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
        return Err(anyhow::anyhow!(
            "tempo-daemon executable not found at {:?}",
            daemon_exe
        ));
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
            let result = Command::new("kill").arg(pid.to_string()).output();

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
                    Ok(IpcResponse::Status {
                        daemon_running: _,
                        active_session,
                        uptime,
                    }) => {
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
        context,
    };

    match client.send_message(&message).await {
        Ok(IpcResponse::Ok) => {
            println!("Session started for project at {:?}", project_path);
        }
        Ok(IpcResponse::Error(message)) => {
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
        Ok(IpcResponse::Error(message)) => {
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
        Ok(IpcResponse::Error(message)) => {
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
        Ok(IpcResponse::Error(message)) => {
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
        Ok(IpcResponse::Error(message)) => {
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
            let output_path = PathBuf::from("tempo-report.csv");
            generator.export_csv(&report, &output_path)?;
            println!("Report exported to: {:?}", output_path);
        }
        Some("json") => {
            let output_path = PathBuf::from("tempo-report.json");
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
        "terminal" => "\x1b[96m", // Bright cyan
        "ide" => "\x1b[95m",      // Bright magenta
        "linked" => "\x1b[93m",   // Bright yellow
        "manual" => "\x1b[94m",   // Bright blue
        _ => "\x1b[97m",          // Bright white (default)
    };

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m           \x1b[1;37mCurrent Session\x1b[0m               \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[36m│\x1b[0m Status:   \x1b[1;32m*\x1b[0m \x1b[32mActive\x1b[0m                     \x1b[36m│\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m Project:  \x1b[1;33m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
        truncate_string(&session.project_name, 25)
    );
    println!(
        "\x1b[36m│\x1b[0m Duration: \x1b[1;32m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
        format_duration_fancy(session.duration)
    );
    println!(
        "\x1b[36m│\x1b[0m Started:  \x1b[37m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
        session.start_time.format("%H:%M:%S").to_string()
    );
    println!(
        "\x1b[36m│\x1b[0m Context:  {}{:<25}\x1b[0m \x1b[36m│\x1b[0m",
        context_color,
        truncate_string(&session.context, 25)
    );
    println!(
        "\x1b[36m│\x1b[0m Path:     \x1b[2;37m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
        truncate_string(&session.project_path.to_string_lossy(), 25)
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    Ok(())
}

fn print_formatted_report(report: &crate::cli::reports::TimeReport) -> Result<()> {
    // Helper function to get context color
    let get_context_color = |context: &str| -> &str {
        match context {
            "terminal" => "\x1b[96m", // Bright cyan
            "ide" => "\x1b[95m",      // Bright magenta
            "linked" => "\x1b[93m",   // Bright yellow
            "manual" => "\x1b[94m",   // Bright blue
            _ => "\x1b[97m",          // Bright white (default)
        }
    };

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m            \x1b[1;37mTime Report\x1b[0m                  \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");

    for (project_name, project_summary) in &report.projects {
        println!(
            "\x1b[36m│\x1b[0m \x1b[1;33m{:<20}\x1b[0m \x1b[1;32m{:>15}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(project_name, 20),
            format_duration_fancy(project_summary.total_duration)
        );

        for (context, duration) in &project_summary.contexts {
            let context_color = get_context_color(context);
            println!(
                "\x1b[36m│\x1b[0m   {}{:<15}\x1b[0m \x1b[32m{:>20}\x1b[0m \x1b[36m│\x1b[0m",
                context_color,
                truncate_string(context, 15),
                format_duration_fancy(*duration)
            );
        }
        println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    }

    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m \x1b[1;37mTotal Time:\x1b[0m \x1b[1;32m{:>26}\x1b[0m \x1b[36m│\x1b[0m",
        format_duration_fancy(report.total_duration)
    );
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
    println!("\x1b[36m│\x1b[0m Status:   \x1b[1;31m*\x1b[0m \x1b[31mOffline\x1b[0m                   \x1b[36m│\x1b[0m");
    println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m \x1b[33mDaemon is not running.\x1b[0m                 \x1b[36m│\x1b[0m"
    );
    println!("\x1b[36m│\x1b[0m \x1b[37mStart it with:\x1b[0m \x1b[96mtempo start\x1b[0m         \x1b[36m│\x1b[0m");
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
}

fn print_no_active_session(message: &str) {
    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m           \x1b[1;37mCurrent Session\x1b[0m               \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[36m│\x1b[0m Status:   \x1b[1;33m-\x1b[0m \x1b[33mIdle\x1b[0m                      \x1b[36m│\x1b[0m");
    println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m \x1b[90m{:<37}\x1b[0m \x1b[36m│\x1b[0m",
        message
    );
    println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m \x1b[37mStart tracking:\x1b[0m                       \x1b[36m│\x1b[0m"
    );
    println!(
        "\x1b[36m│\x1b[0m   \x1b[96mtempo session start\x1b[0m                \x1b[36m│\x1b[0m"
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
}

fn print_daemon_status(uptime: u64, active_session: Option<&crate::utils::ipc::SessionInfo>) {
    let uptime_formatted = format_duration_fancy(uptime as i64);

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m               \x1b[1;37mDaemon Status\x1b[0m               \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!("\x1b[36m│\x1b[0m Status:   \x1b[1;32m*\x1b[0m \x1b[32mOnline\x1b[0m                    \x1b[36m│\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m Uptime:   \x1b[37m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
        uptime_formatted
    );

    if let Some(session) = active_session {
        let context_color = match session.context.as_str() {
            "terminal" => "\x1b[96m",
            "ide" => "\x1b[95m",
            "linked" => "\x1b[93m",
            "manual" => "\x1b[94m",
            _ => "\x1b[97m",
        };

        println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
        println!("\x1b[36m│\x1b[0m \x1b[1;37mActive Session:\x1b[0m                      \x1b[36m│\x1b[0m");
        println!(
            "\x1b[36m│\x1b[0m   Project: \x1b[1;33m{:<23}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(&session.project_name, 23)
        );
        println!(
            "\x1b[36m│\x1b[0m   Duration: \x1b[1;32m{:<22}\x1b[0m \x1b[36m│\x1b[0m",
            format_duration_fancy(session.duration)
        );
        println!(
            "\x1b[36m│\x1b[0m   Context: {}{:<23}\x1b[0m \x1b[36m│\x1b[0m",
            context_color, session.context
        );
    } else {
        println!("\x1b[36m│\x1b[0m Session:  \x1b[33mNo active session\x1b[0m             \x1b[36m│\x1b[0m");
    }

    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
}

// Project management functions
async fn init_project(
    name: Option<String>,
    path: Option<PathBuf>,
    description: Option<String>,
) -> Result<()> {
    // Validate inputs early
    let validated_name = if let Some(n) = name.as_ref() {
        Some(validate_project_name(n).with_context(|| format!("Invalid project name '{}'", n))?)
    } else {
        None
    };

    let validated_description = if let Some(d) = description.as_ref() {
        Some(validate_project_description(d).with_context(|| "Invalid project description")?)
    } else {
        None
    };

    let project_path =
        path.unwrap_or_else(|| env::current_dir().expect("Failed to get current directory"));

    // Use secure path validation
    let canonical_path = validate_project_path(&project_path)
        .with_context(|| format!("Invalid project path: {}", project_path.display()))?;

    let project_name = validated_name.clone().unwrap_or_else(|| {
        let detected = detect_project_name(&canonical_path);
        validate_project_name(&detected).unwrap_or_else(|_| "project".to_string())
    });

    // Get database connection from pool
    let conn = match get_connection().await {
        Ok(conn) => conn,
        Err(_) => {
            // Fallback to direct connection
            let db_path = get_database_path()?;
            let db = Database::new(&db_path)?;
            return init_project_with_db(
                validated_name,
                Some(canonical_path),
                validated_description,
                &db.connection,
            )
            .await;
        }
    };

    // Check if project already exists
    if let Some(existing) = ProjectQueries::find_by_path(conn.connection(), &canonical_path)? {
        eprintln!(
            "\x1b[33m! Warning:\x1b[0m A project named '{}' already exists at this path.",
            existing.name
        );
        eprintln!("Use 'tempo list' to see all projects or choose a different location.");
        return Ok(());
    }

    // Use the pooled connection to complete initialization
    init_project_with_db(
        Some(project_name.clone()),
        Some(canonical_path.clone()),
        validated_description,
        conn.connection(),
    )
    .await?;

    println!(
        "\x1b[32m+ Success:\x1b[0m Project '{}' initialized at {}",
        project_name,
        canonical_path.display()
    );
    println!("Start tracking time with: \x1b[36mtempo start\x1b[0m");

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
        let status_icon = if project.is_archived { "[A]" } else { "[P]" };
        let status_color = if project.is_archived {
            "\x1b[90m"
        } else {
            "\x1b[37m"
        };
        let git_indicator = if project.git_hash.is_some() {
            " (git)"
        } else {
            ""
        };

        println!(
            "\x1b[36m│\x1b[0m {} {}{:<25}\x1b[0m \x1b[36m│\x1b[0m",
            status_icon,
            status_color,
            format!("{}{}", truncate_string(&project.name, 20), git_indicator)
        );

        if let Some(description) = &project.description {
            println!(
                "\x1b[36m│\x1b[0m   \x1b[2;37m{:<35}\x1b[0m \x1b[36m│\x1b[0m",
                truncate_string(description, 35)
            );
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
            println!(
                "\x1b[36m│\x1b[0m   \x1b[90m{:<35}\x1b[0m \x1b[36m│\x1b[0m",
                truncate_string(&display_path, 35)
            );
        } else {
            println!(
                "\x1b[36m│\x1b[0m   \x1b[90m{:<35}\x1b[0m \x1b[36m│\x1b[0m",
                truncate_string(&path_display, 35)
            );
        }

        println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    }

    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m \x1b[1;37mTotal:\x1b[0m {:<30} \x1b[36m│\x1b[0m",
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
async fn create_tag(
    name: String,
    color: Option<String>,
    description: Option<String>,
) -> Result<()> {
    // Initialize database
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    // Create tag - use builder pattern to avoid cloning
    let mut tag = Tag::new(name);
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
    println!(
        "\x1b[36m│\x1b[0m Name:     \x1b[1;33m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
        truncate_string(&tag.name, 27)
    );
    if let Some(color_val) = &tag.color {
        println!(
            "\x1b[36m│\x1b[0m Color:    \x1b[37m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(color_val, 27)
        );
    }
    if let Some(desc) = &tag.description {
        println!(
            "\x1b[36m│\x1b[0m Desc:     \x1b[2;37m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(desc, 27)
        );
    }
    println!(
        "\x1b[36m│\x1b[0m ID:       \x1b[90m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
        tag_id
    );
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m \x1b[32m✓ Tag created successfully\x1b[0m             \x1b[36m│\x1b[0m"
    );
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
            String::new()
        };

        println!(
            "\x1b[36m│\x1b[0m 🏷️  \x1b[1;33m{:<30}\x1b[0m \x1b[36m│\x1b[0m",
            format!("{}{}", truncate_string(&tag.name, 25), color_indicator)
        );

        if let Some(description) = &tag.description {
            println!(
                "\x1b[36m│\x1b[0m     \x1b[2;37m{:<33}\x1b[0m \x1b[36m│\x1b[0m",
                truncate_string(description, 33)
            );
        }

        println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    }

    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m \x1b[1;37mTotal:\x1b[0m {:<30} \x1b[36m│\x1b[0m",
        format!("{} tags", tags.len())
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");

    Ok(())
}

async fn delete_tag(name: String) -> Result<()> {
    // Initialize database
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    // Check if tag exists
    if TagQueries::find_by_name(&db.connection, &name)?.is_none() {
        println!("\x1b[31m✗ Tag '{}' not found\x1b[0m", name);
        return Ok(());
    }

    // Delete the tag
    let deleted = TagQueries::delete_by_name(&db.connection, &name)?;

    if deleted {
        println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
        println!("\x1b[36m│\x1b[0m           \x1b[1;37mTag Deleted\x1b[0m                   \x1b[36m│\x1b[0m");
        println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
        println!(
            "\x1b[36m│\x1b[0m Name:     \x1b[1;33m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(&name, 27)
        );
        println!(
            "\x1b[36m│\x1b[0m Status:   \x1b[32mDeleted\x1b[0m                   \x1b[36m│\x1b[0m"
        );
        println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
        println!("\x1b[36m│\x1b[0m \x1b[32m✓ Tag deleted successfully\x1b[0m             \x1b[36m│\x1b[0m");
        println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    } else {
        println!("\x1b[31m✗ Failed to delete tag '{}'\x1b[0m", name);
    }

    Ok(())
}

// Configuration management functions
async fn show_config() -> Result<()> {
    let config = load_config()?;

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m           \x1b[1;37mConfiguration\x1b[0m                  \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m idle_timeout_minutes:  \x1b[33m{:<16}\x1b[0m \x1b[36m│\x1b[0m",
        config.idle_timeout_minutes
    );
    println!(
        "\x1b[36m│\x1b[0m auto_pause_enabled:    \x1b[33m{:<16}\x1b[0m \x1b[36m│\x1b[0m",
        config.auto_pause_enabled
    );
    println!(
        "\x1b[36m│\x1b[0m default_context:       \x1b[33m{:<16}\x1b[0m \x1b[36m│\x1b[0m",
        config.default_context
    );
    println!(
        "\x1b[36m│\x1b[0m max_session_hours:     \x1b[33m{:<16}\x1b[0m \x1b[36m│\x1b[0m",
        config.max_session_hours
    );
    println!(
        "\x1b[36m│\x1b[0m backup_enabled:        \x1b[33m{:<16}\x1b[0m \x1b[36m│\x1b[0m",
        config.backup_enabled
    );
    println!(
        "\x1b[36m│\x1b[0m log_level:             \x1b[33m{:<16}\x1b[0m \x1b[36m│\x1b[0m",
        config.log_level
    );

    if !config.custom_settings.is_empty() {
        println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
        println!("\x1b[36m│\x1b[0m \x1b[1;37mCustom Settings:\x1b[0m                      \x1b[36m│\x1b[0m");
        for (key, value) in &config.custom_settings {
            println!(
                "\x1b[36m│\x1b[0m {:<20} \x1b[33m{:<16}\x1b[0m \x1b[36m│\x1b[0m",
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
            println!(
                "\x1b[36m│\x1b[0m {:<20} \x1b[33m{:<16}\x1b[0m \x1b[36m│\x1b[0m",
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
    println!(
        "\x1b[36m│\x1b[0m {:<20} \x1b[32m{:<16}\x1b[0m \x1b[36m│\x1b[0m",
        truncate_string(&key, 20),
        truncate_string(&display_value, 16)
    );
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m \x1b[32m✓ Configuration saved successfully\x1b[0m      \x1b[36m│\x1b[0m"
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");

    Ok(())
}

async fn reset_config() -> Result<()> {
    let default_config = crate::models::Config::default();
    save_config(&default_config)?;

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m         \x1b[1;37mConfiguration Reset\x1b[0m              \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m \x1b[32m✓ Configuration reset to defaults\x1b[0m       \x1b[36m│\x1b[0m"
    );
    println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m \x1b[37mView current config:\x1b[0m                   \x1b[36m│\x1b[0m"
    );
    println!(
        "\x1b[36m│\x1b[0m   \x1b[96mtempo config show\x1b[0m                   \x1b[36m│\x1b[0m"
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");

    Ok(())
}

// Session management functions
async fn list_sessions(limit: Option<usize>, project_filter: Option<String>) -> Result<()> {
    // Initialize database
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    let session_limit = limit.unwrap_or(10);

    // Handle project filtering
    let project_id = if let Some(project_name) = &project_filter {
        match ProjectQueries::find_by_name(&db.connection, project_name)? {
            Some(project) => Some(project.id.unwrap()),
            None => {
                println!("\x1b[31m✗ Project '{}' not found\x1b[0m", project_name);
                return Ok(());
            }
        }
    } else {
        None
    };

    let sessions = SessionQueries::list_with_filter(
        &db.connection,
        project_id,
        None,
        None,
        Some(session_limit),
    )?;

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
        let status_icon = if session.end_time.is_some() {
            "✅"
        } else {
            "🔄"
        };
        let duration = if let Some(end) = session.end_time {
            (end - session.start_time).num_seconds() - session.paused_duration.num_seconds()
        } else {
            (Utc::now() - session.start_time).num_seconds() - session.paused_duration.num_seconds()
        };

        let context_color = match session.context {
            crate::models::SessionContext::Terminal => "\x1b[96m",
            crate::models::SessionContext::IDE => "\x1b[95m",
            crate::models::SessionContext::Linked => "\x1b[93m",
            crate::models::SessionContext::Manual => "\x1b[94m",
        };

        println!(
            "\x1b[36m│\x1b[0m {} \x1b[1;37m{:<32}\x1b[0m \x1b[36m│\x1b[0m",
            status_icon,
            format!("Session {}", session.id.unwrap_or(0))
        );
        println!(
            "\x1b[36m│\x1b[0m    Duration: \x1b[32m{:<24}\x1b[0m \x1b[36m│\x1b[0m",
            format_duration_fancy(duration)
        );
        println!(
            "\x1b[36m│\x1b[0m    Context:  {}{:<24}\x1b[0m \x1b[36m│\x1b[0m",
            context_color, session.context
        );
        println!(
            "\x1b[36m│\x1b[0m    Started:  \x1b[37m{:<24}\x1b[0m \x1b[36m│\x1b[0m",
            session.start_time.format("%Y-%m-%d %H:%M:%S")
        );
        println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    }

    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m \x1b[1;37mShowing:\x1b[0m {:<28} \x1b[36m│\x1b[0m",
        format!("{} recent sessions", filtered_sessions.len())
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");

    Ok(())
}

async fn edit_session(
    id: i64,
    start: Option<String>,
    end: Option<String>,
    project: Option<String>,
    reason: Option<String>,
) -> Result<()> {
    // Initialize database
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    // Find the session
    let session = SessionQueries::find_by_id(&db.connection, id)?;
    let session = match session {
        Some(s) => s,
        None => {
            println!("\x1b[31m✗ Session {} not found\x1b[0m", id);
            return Ok(());
        }
    };

    let original_start = session.start_time;
    let original_end = session.end_time;

    // Parse new values
    let mut new_start = original_start;
    let mut new_end = original_end;
    let mut new_project_id = session.project_id;

    // Parse start time if provided
    if let Some(start_str) = &start {
        new_start = match chrono::DateTime::parse_from_rfc3339(start_str) {
            Ok(dt) => dt.with_timezone(&chrono::Utc),
            Err(_) => match chrono::NaiveDateTime::parse_from_str(start_str, "%Y-%m-%d %H:%M:%S") {
                Ok(dt) => chrono::Utc.from_utc_datetime(&dt),
                Err(_) => {
                    return Err(anyhow::anyhow!(
                        "Invalid start time format. Use RFC3339 or 'YYYY-MM-DD HH:MM:SS'"
                    ))
                }
            },
        };
    }

    // Parse end time if provided
    if let Some(end_str) = &end {
        if end_str.to_lowercase() == "null" || end_str.to_lowercase() == "none" {
            new_end = None;
        } else {
            new_end = Some(match chrono::DateTime::parse_from_rfc3339(end_str) {
                Ok(dt) => dt.with_timezone(&chrono::Utc),
                Err(_) => {
                    match chrono::NaiveDateTime::parse_from_str(end_str, "%Y-%m-%d %H:%M:%S") {
                        Ok(dt) => chrono::Utc.from_utc_datetime(&dt),
                        Err(_) => {
                            return Err(anyhow::anyhow!(
                                "Invalid end time format. Use RFC3339 or 'YYYY-MM-DD HH:MM:SS'"
                            ))
                        }
                    }
                }
            });
        }
    }

    // Find project by name if provided
    if let Some(project_name) = &project {
        if let Some(proj) = ProjectQueries::find_by_name(&db.connection, project_name)? {
            new_project_id = proj.id.unwrap();
        } else {
            println!("\x1b[31m✗ Project '{}' not found\x1b[0m", project_name);
            return Ok(());
        }
    }

    // Validate the edit
    if new_start >= new_end.unwrap_or(chrono::Utc::now()) {
        println!("\x1b[31m✗ Start time must be before end time\x1b[0m");
        return Ok(());
    }

    // Create audit trail record
    SessionEditQueries::create_edit_record(
        &db.connection,
        id,
        original_start,
        original_end,
        new_start,
        new_end,
        reason.clone(),
    )?;

    // Update the session
    SessionQueries::update_session(
        &db.connection,
        id,
        if start.is_some() {
            Some(new_start)
        } else {
            None
        },
        if end.is_some() { Some(new_end) } else { None },
        if project.is_some() {
            Some(new_project_id)
        } else {
            None
        },
        None,
    )?;

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m         \x1b[1;37mSession Updated\x1b[0m                 \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m Session:  \x1b[1;37m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
        id
    );

    if start.is_some() {
        println!(
            "\x1b[36m│\x1b[0m Start:    \x1b[32m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(&new_start.format("%Y-%m-%d %H:%M:%S").to_string(), 27)
        );
    }

    if end.is_some() {
        let end_str = if let Some(e) = new_end {
            e.format("%Y-%m-%d %H:%M:%S").to_string()
        } else {
            "Ongoing".to_string()
        };
        println!(
            "\x1b[36m│\x1b[0m End:      \x1b[32m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(&end_str, 27)
        );
    }

    if let Some(r) = &reason {
        println!(
            "\x1b[36m│\x1b[0m Reason:   \x1b[2;37m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(r, 27)
        );
    }

    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m \x1b[32m✓ Session updated with audit trail\x1b[0m     \x1b[36m│\x1b[0m"
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");

    Ok(())
}

async fn delete_session(id: i64, force: bool) -> Result<()> {
    // Initialize database
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    // Check if session exists
    let session = SessionQueries::find_by_id(&db.connection, id)?;
    let session = match session {
        Some(s) => s,
        None => {
            println!("\x1b[31m✗ Session {} not found\x1b[0m", id);
            return Ok(());
        }
    };

    // Check if it's an active session and require force flag
    if session.end_time.is_none() && !force {
        println!("\x1b[33m⚠  Cannot delete active session without --force flag\x1b[0m");
        println!("  Use: tempo session delete {} --force", id);
        return Ok(());
    }

    // Delete the session
    SessionQueries::delete_session(&db.connection, id)?;

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m         \x1b[1;37mSession Deleted\x1b[0m                 \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m Session:  \x1b[1;37m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
        id
    );
    println!(
        "\x1b[36m│\x1b[0m Status:   \x1b[32mDeleted\x1b[0m                   \x1b[36m│\x1b[0m"
    );

    if session.end_time.is_none() {
        println!("\x1b[36m│\x1b[0m Type:     \x1b[33mActive session (forced)\x1b[0m      \x1b[36m│\x1b[0m");
    } else {
        println!("\x1b[36m│\x1b[0m Type:     \x1b[37mCompleted session\x1b[0m           \x1b[36m│\x1b[0m");
    }

    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m \x1b[32m✓ Session and audit trail removed\x1b[0m      \x1b[36m│\x1b[0m"
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");

    Ok(())
}

// Project management functions
async fn archive_project(project_name: String) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    let project = match ProjectQueries::find_by_name(&db.connection, &project_name)? {
        Some(p) => p,
        None => {
            println!("\x1b[31m✗ Project '{}' not found\x1b[0m", project_name);
            return Ok(());
        }
    };

    if project.is_archived {
        println!(
            "\x1b[33m⚠  Project '{}' is already archived\x1b[0m",
            project_name
        );
        return Ok(());
    }

    let success = ProjectQueries::archive_project(&db.connection, project.id.unwrap())?;

    if success {
        println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
        println!("\x1b[36m│\x1b[0m        \x1b[1;37mProject Archived\x1b[0m                \x1b[36m│\x1b[0m");
        println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
        println!(
            "\x1b[36m│\x1b[0m Name:     \x1b[1;33m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(&project_name, 27)
        );
        println!(
            "\x1b[36m│\x1b[0m Status:   \x1b[90mArchived\x1b[0m                  \x1b[36m│\x1b[0m"
        );
        println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
        println!("\x1b[36m│\x1b[0m \x1b[32m✓ Project archived successfully\x1b[0m        \x1b[36m│\x1b[0m");
        println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    } else {
        println!(
            "\x1b[31m✗ Failed to archive project '{}'\x1b[0m",
            project_name
        );
    }

    Ok(())
}

async fn unarchive_project(project_name: String) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    let project = match ProjectQueries::find_by_name(&db.connection, &project_name)? {
        Some(p) => p,
        None => {
            println!("\x1b[31m✗ Project '{}' not found\x1b[0m", project_name);
            return Ok(());
        }
    };

    if !project.is_archived {
        println!(
            "\x1b[33m⚠  Project '{}' is not archived\x1b[0m",
            project_name
        );
        return Ok(());
    }

    let success = ProjectQueries::unarchive_project(&db.connection, project.id.unwrap())?;

    if success {
        println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
        println!("\x1b[36m│\x1b[0m       \x1b[1;37mProject Unarchived\x1b[0m               \x1b[36m│\x1b[0m");
        println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
        println!(
            "\x1b[36m│\x1b[0m Name:     \x1b[1;33m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(&project_name, 27)
        );
        println!(
            "\x1b[36m│\x1b[0m Status:   \x1b[32mActive\x1b[0m                    \x1b[36m│\x1b[0m"
        );
        println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
        println!("\x1b[36m│\x1b[0m \x1b[32m✓ Project unarchived successfully\x1b[0m      \x1b[36m│\x1b[0m");
        println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    } else {
        println!(
            "\x1b[31m✗ Failed to unarchive project '{}'\x1b[0m",
            project_name
        );
    }

    Ok(())
}

async fn update_project_path(project_name: String, new_path: PathBuf) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    let project = match ProjectQueries::find_by_name(&db.connection, &project_name)? {
        Some(p) => p,
        None => {
            println!("\x1b[31m✗ Project '{}' not found\x1b[0m", project_name);
            return Ok(());
        }
    };

    let canonical_path = canonicalize_path(&new_path)?;
    let success =
        ProjectQueries::update_project_path(&db.connection, project.id.unwrap(), &canonical_path)?;

    if success {
        println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
        println!("\x1b[36m│\x1b[0m       \x1b[1;37mProject Path Updated\x1b[0m              \x1b[36m│\x1b[0m");
        println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
        println!(
            "\x1b[36m│\x1b[0m Name:     \x1b[1;33m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(&project_name, 27)
        );
        println!(
            "\x1b[36m│\x1b[0m Old Path: \x1b[2;37m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(&project.path.to_string_lossy(), 27)
        );
        println!(
            "\x1b[36m│\x1b[0m New Path: \x1b[32m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(&canonical_path.to_string_lossy(), 27)
        );
        println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
        println!("\x1b[36m│\x1b[0m \x1b[32m✓ Path updated successfully\x1b[0m            \x1b[36m│\x1b[0m");
        println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    } else {
        println!(
            "\x1b[31m✗ Failed to update path for project '{}'\x1b[0m",
            project_name
        );
    }

    Ok(())
}

async fn add_tag_to_project(project_name: String, tag_name: String) -> Result<()> {
    println!("\x1b[33m⚠  Project-tag associations not yet implemented\x1b[0m");
    println!("Would add tag '{}' to project '{}'", tag_name, project_name);
    println!("This requires implementing project_tags table operations.");
    Ok(())
}

async fn remove_tag_from_project(project_name: String, tag_name: String) -> Result<()> {
    println!("\x1b[33m⚠  Project-tag associations not yet implemented\x1b[0m");
    println!(
        "Would remove tag '{}' from project '{}'",
        tag_name, project_name
    );
    println!("This requires implementing project_tags table operations.");
    Ok(())
}

// Bulk session operations
async fn bulk_update_sessions_project(
    session_ids: Vec<i64>,
    new_project_name: String,
) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    // Find the target project
    let project = match ProjectQueries::find_by_name(&db.connection, &new_project_name)? {
        Some(p) => p,
        None => {
            println!("\x1b[31m✗ Project '{}' not found\x1b[0m", new_project_name);
            return Ok(());
        }
    };

    let updated =
        SessionQueries::bulk_update_project(&db.connection, &session_ids, project.id.unwrap())?;

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m      \x1b[1;37mBulk Session Update\x1b[0m               \x1b[36m│\x1b[0m"
    );
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m Sessions: \x1b[1;37m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
        updated
    );
    println!(
        "\x1b[36m│\x1b[0m Project:  \x1b[1;33m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
        truncate_string(&new_project_name, 27)
    );
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m \x1b[32m✓ {} sessions updated\x1b[0m {:<12} \x1b[36m│\x1b[0m",
        updated, ""
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");

    Ok(())
}

async fn bulk_delete_sessions(session_ids: Vec<i64>) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    let deleted = SessionQueries::bulk_delete(&db.connection, &session_ids)?;

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m      \x1b[1;37mBulk Session Delete\x1b[0m               \x1b[36m│\x1b[0m"
    );
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m Requested: \x1b[1;37m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
        session_ids.len()
    );
    println!(
        "\x1b[36m│\x1b[0m Deleted:   \x1b[32m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
        deleted
    );
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m \x1b[32m✓ {} sessions deleted\x1b[0m {:<10} \x1b[36m│\x1b[0m",
        deleted, ""
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");

    Ok(())
}

async fn launch_dashboard() -> Result<()> {
    // Check if we have a TTY first
    if !is_tty() {
        return show_dashboard_fallback().await;
    }

    // Setup terminal with better error handling
    enable_raw_mode().context("Failed to enable raw mode - terminal may not support interactive features")?;
    let mut stdout = io::stdout();
    
    execute!(stdout, EnterAlternateScreen)
        .context("Failed to enter alternate screen - terminal may not support full-screen mode")?;
    
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)
        .context("Failed to initialize terminal backend")?;

    // Clear the screen first
    terminal.clear().context("Failed to clear terminal")?;

    // Create dashboard instance and run it
    let result = async {
        let mut dashboard = Dashboard::new().await?;
        dashboard.run(&mut terminal).await
    };

    let result = tokio::task::block_in_place(|| Handle::current().block_on(result));

    // Always restore terminal, even if there was an error
    let cleanup_result = cleanup_terminal(&mut terminal);
    
    // Return the original result, but log cleanup errors
    if let Err(e) = cleanup_result {
        eprintln!("Warning: Failed to restore terminal: {}", e);
    }

    result
}

fn is_tty() -> bool {
    use std::os::unix::io::AsRawFd;
    unsafe { libc::isatty(std::io::stdin().as_raw_fd()) == 1 }
}

async fn show_dashboard_fallback() -> Result<()> {
    println!("📊 Tempo Dashboard (Text Mode)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    
    // Get basic status information
    if is_daemon_running() {
        println!("🟢 Daemon Status: Running");
    } else {
        println!("🔴 Daemon Status: Offline");
        println!("   Start with: tempo start");
        println!();
        return Ok(());
    }

    // Show current session info
    let socket_path = get_socket_path()?;
    if let Ok(mut client) = IpcClient::connect(&socket_path).await {
        match client.send_message(&IpcMessage::GetActiveSession).await {
            Ok(IpcResponse::ActiveSession(Some(session))) => {
                println!("⏱️  Active Session:");
                println!("   Started: {}", session.start_time.format("%H:%M:%S"));
                println!("   Duration: {}", format_duration_simple((chrono::Utc::now().timestamp() - session.start_time.timestamp()) - session.paused_duration.num_seconds()));
                println!("   Context: {}", session.context);
                println!();
                
                // Get project info
                match client.send_message(&IpcMessage::GetProject(session.project_id)).await {
                    Ok(IpcResponse::Project(Some(project))) => {
                        println!("📁 Current Project: {}", project.name);
                        println!("   Path: {}", project.path.display());
                        println!();
                    }
                    _ => {
                        println!("📁 Project: Unknown");
                        println!();
                    }
                }
            }
            _ => {
                println!("⏸️  No active session");
                println!("   Start tracking with: tempo session start");
                println!();
            }
        }

        // Get daily stats
        let today = chrono::Local::now().date_naive();
        match client.send_message(&IpcMessage::GetDailyStats(today)).await {
            Ok(IpcResponse::DailyStats { sessions_count, total_seconds, avg_seconds }) => {
                println!("📈 Today's Summary:");
                println!("   Sessions: {}", sessions_count);
                println!("   Total time: {}", format_duration_simple(total_seconds));
                if sessions_count > 0 {
                    println!("   Average session: {}", format_duration_simple(avg_seconds));
                }
                let progress = (total_seconds as f64 / (8.0 * 3600.0)) * 100.0;
                println!("   Daily goal (8h): {:.1}%", progress);
                println!();
            }
            _ => {
                println!("📈 Today's Summary: No data available");
                println!();
            }
        }
    } else {
        println!("❌ Unable to connect to daemon");
        println!("   Try: tempo restart");
        println!();
    }

    println!("💡 For interactive dashboard, run in a terminal:");
    println!("   • Terminal.app, iTerm2, or other terminal emulators");
    println!("   • SSH sessions with TTY allocation (ssh -t)");
    println!("   • Interactive shell environments");
    
    Ok(())
}

fn format_duration_simple(seconds: i64) -> String {
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

fn cleanup_terminal<B>(terminal: &mut Terminal<B>) -> Result<()> 
where 
    B: ratatui::backend::Backend + std::io::Write,
{
    // Restore terminal
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("Failed to leave alternate screen")?;
    terminal.show_cursor().context("Failed to show cursor")?;
    Ok(())
}

async fn launch_timer() -> Result<()> {
    // Check if we have a TTY first
    if !is_tty() {
        return Err(anyhow::anyhow!(
            "Interactive timer requires an interactive terminal (TTY).\n\
            \n\
            This command needs to run in a proper terminal environment.\n\
            Try running this command directly in your terminal application."
        ));
    }

    // Setup terminal with better error handling
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to initialize terminal")?;
    terminal.clear().context("Failed to clear terminal")?;

    // Create timer instance and run it
    let result = async {
        let mut timer = InteractiveTimer::new().await?;
        timer.run(&mut terminal).await
    };

    let result = tokio::task::block_in_place(|| Handle::current().block_on(result));

    // Always restore terminal
    let cleanup_result = cleanup_terminal(&mut terminal);
    if let Err(e) = cleanup_result {
        eprintln!("Warning: Failed to restore terminal: {}", e);
    }

    result
}

async fn merge_sessions(
    session_ids_str: String,
    project_name: Option<String>,
    notes: Option<String>,
) -> Result<()> {
    // Parse session IDs
    let session_ids: Result<Vec<i64>, _> = session_ids_str
        .split(',')
        .map(|s| s.trim().parse::<i64>())
        .collect();

    let session_ids = session_ids.map_err(|_| {
        anyhow::anyhow!("Invalid session IDs format. Use comma-separated numbers like '1,2,3'")
    })?;

    if session_ids.len() < 2 {
        return Err(anyhow::anyhow!(
            "At least 2 sessions are required for merging"
        ));
    }

    // Get target project ID if specified
    let mut target_project_id = None;
    if let Some(project) = project_name {
        let db_path = get_database_path()?;
        let db = Database::new(&db_path)?;

        // Try to find project by name first, then by ID
        if let Ok(project_id) = project.parse::<i64>() {
            if ProjectQueries::find_by_id(&db.connection, project_id)?.is_some() {
                target_project_id = Some(project_id);
            }
        } else if let Some(proj) = ProjectQueries::find_by_name(&db.connection, &project)? {
            target_project_id = proj.id;
        }

        if target_project_id.is_none() {
            return Err(anyhow::anyhow!("Project '{}' not found", project));
        }
    }

    // Perform the merge
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    let merged_id =
        SessionQueries::merge_sessions(&db.connection, &session_ids, target_project_id, notes)?;

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m        \x1b[1;37mSession Merge Complete\x1b[0m            \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m Merged sessions: \x1b[33m{:<22}\x1b[0m \x1b[36m│\x1b[0m",
        session_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "\x1b[36m│\x1b[0m New session ID:  \x1b[32m{:<22}\x1b[0m \x1b[36m│\x1b[0m",
        merged_id
    );
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m \x1b[32m✓ Sessions successfully merged\x1b[0m        \x1b[36m│\x1b[0m"
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");

    Ok(())
}

async fn split_session(
    session_id: i64,
    split_times_str: String,
    notes: Option<String>,
) -> Result<()> {
    // Parse split times
    let split_time_strings: Vec<&str> = split_times_str.split(',').map(|s| s.trim()).collect();
    let mut split_times = Vec::new();

    for time_str in split_time_strings {
        // Try to parse as time (HH:MM or HH:MM:SS)
        let datetime = if time_str.contains(':') {
            // Parse as time and combine with today's date
            let today = chrono::Local::now().date_naive();
            let time = chrono::NaiveTime::parse_from_str(time_str, "%H:%M")
                .or_else(|_| chrono::NaiveTime::parse_from_str(time_str, "%H:%M:%S"))
                .map_err(|_| {
                    anyhow::anyhow!("Invalid time format '{}'. Use HH:MM or HH:MM:SS", time_str)
                })?;
            today.and_time(time).and_utc()
        } else {
            // Try to parse as full datetime
            chrono::DateTime::parse_from_rfc3339(time_str)
                .map_err(|_| {
                    anyhow::anyhow!(
                        "Invalid datetime format '{}'. Use HH:MM or RFC3339 format",
                        time_str
                    )
                })?
                .to_utc()
        };

        split_times.push(datetime);
    }

    if split_times.is_empty() {
        return Err(anyhow::anyhow!("No valid split times provided"));
    }

    // Parse notes if provided
    let notes_list = notes.map(|n| {
        n.split(',')
            .map(|s| s.trim().to_string())
            .collect::<Vec<String>>()
    });

    // Perform the split
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    let new_session_ids =
        SessionQueries::split_session(&db.connection, session_id, &split_times, notes_list)?;

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m        \x1b[1;37mSession Split Complete\x1b[0m            \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m Original session: \x1b[33m{:<20}\x1b[0m \x1b[36m│\x1b[0m",
        session_id
    );
    println!(
        "\x1b[36m│\x1b[0m Split points:     \x1b[90m{:<20}\x1b[0m \x1b[36m│\x1b[0m",
        split_times.len()
    );
    println!(
        "\x1b[36m│\x1b[0m New sessions:     \x1b[32m{:<20}\x1b[0m \x1b[36m│\x1b[0m",
        new_session_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m \x1b[32m✓ Session successfully split\x1b[0m          \x1b[36m│\x1b[0m"
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");

    Ok(())
}

async fn launch_history() -> Result<()> {
    // Check if we have a TTY first
    if !is_tty() {
        return Err(anyhow::anyhow!(
            "Session history browser requires an interactive terminal (TTY).\n\
            \n\
            This command needs to run in a proper terminal environment.\n\
            Try running this command directly in your terminal application."
        ));
    }

    // Setup terminal with better error handling
    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("Failed to initialize terminal")?;
    terminal.clear().context("Failed to clear terminal")?;

    let result = async {
        let mut browser = SessionHistoryBrowser::new().await?;
        browser.run(&mut terminal).await
    };

    let result = tokio::task::block_in_place(|| Handle::current().block_on(result));

    // Always restore terminal
    let cleanup_result = cleanup_terminal(&mut terminal);
    if let Err(e) = cleanup_result {
        eprintln!("Warning: Failed to restore terminal: {}", e);
    }

    result
}

async fn handle_goal_action(action: GoalAction) -> Result<()> {
    match action {
        GoalAction::Create {
            name,
            target_hours,
            project,
            description,
            start_date,
            end_date,
        } => {
            create_goal(
                name,
                target_hours,
                project,
                description,
                start_date,
                end_date,
            )
            .await
        }
        GoalAction::List { project } => list_goals(project).await,
        GoalAction::Update { id, hours } => update_goal_progress(id, hours).await,
    }
}

async fn create_goal(
    name: String,
    target_hours: f64,
    project: Option<String>,
    description: Option<String>,
    start_date: Option<String>,
    end_date: Option<String>,
) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    let project_id = if let Some(proj_name) = project {
        match ProjectQueries::find_by_name(&db.connection, &proj_name)? {
            Some(p) => p.id,
            None => {
                println!("\x1b[31m✗ Project '{}' not found\x1b[0m", proj_name);
                return Ok(());
            }
        }
    } else {
        None
    };

    let start = start_date.and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());
    let end = end_date.and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());

    let mut goal = Goal::new(name.clone(), target_hours);
    if let Some(pid) = project_id {
        goal = goal.with_project(pid);
    }
    if let Some(desc) = description {
        goal = goal.with_description(desc);
    }
    goal = goal.with_dates(start, end);

    goal.validate()?;
    let goal_id = GoalQueries::create(&db.connection, &goal)?;

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m           \x1b[1;37mGoal Created\x1b[0m                   \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m Name:     \x1b[1;33m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
        truncate_string(&name, 27)
    );
    println!(
        "\x1b[36m│\x1b[0m Target:   \x1b[32m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
        format!("{} hours", target_hours)
    );
    println!(
        "\x1b[36m│\x1b[0m ID:       \x1b[90m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
        goal_id
    );
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m \x1b[32m✓ Goal created successfully\x1b[0m             \x1b[36m│\x1b[0m"
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");

    Ok(())
}

async fn list_goals(project: Option<String>) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    let project_id = if let Some(proj_name) = &project {
        match ProjectQueries::find_by_name(&db.connection, proj_name)? {
            Some(p) => p.id,
            None => {
                println!("\x1b[31m✗ Project '{}' not found\x1b[0m", proj_name);
                return Ok(());
            }
        }
    } else {
        None
    };

    let goals = GoalQueries::list_by_project(&db.connection, project_id)?;

    if goals.is_empty() {
        println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
        println!("\x1b[36m│\x1b[0m              \x1b[1;37mNo Goals\x1b[0m                    \x1b[36m│\x1b[0m");
        println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
        return Ok(());
    }

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m                \x1b[1;37mGoals\x1b[0m                      \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");

    for goal in &goals {
        let progress_pct = goal.progress_percentage();
        println!(
            "\x1b[36m│\x1b[0m 🎯 \x1b[1;33m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(&goal.name, 25)
        );
        println!("\x1b[36m│\x1b[0m    Progress: \x1b[32m{:.1}%\x1b[0m ({:.1}h / {:.1}h)     \x1b[36m│\x1b[0m", 
            progress_pct, goal.current_progress, goal.target_hours);
        println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    }

    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    Ok(())
}

async fn update_goal_progress(id: i64, hours: f64) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    GoalQueries::update_progress(&db.connection, id, hours)?;
    println!(
        "\x1b[32m✓ Updated goal {} progress by {} hours\x1b[0m",
        id, hours
    );
    Ok(())
}

async fn show_insights(period: Option<String>, project: Option<String>) -> Result<()> {
    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m        \x1b[1;37mProductivity Insights\x1b[0m              \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m Period:   \x1b[33m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
        period.as_deref().unwrap_or("all")
    );
    if let Some(proj) = project {
        println!(
            "\x1b[36m│\x1b[0m Project:  \x1b[33m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(&proj, 27)
        );
    }
    println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m \x1b[33m⚠  Insights calculation in progress...\x1b[0m  \x1b[36m│\x1b[0m"
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    Ok(())
}

async fn show_summary(period: String, from: Option<String>) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    let start_date = if let Some(from_str) = from {
        chrono::NaiveDate::parse_from_str(&from_str, "%Y-%m-%d")?
    } else {
        match period.as_str() {
            "week" => chrono::Local::now().date_naive() - chrono::Duration::days(7),
            "month" => chrono::Local::now().date_naive() - chrono::Duration::days(30),
            _ => chrono::Local::now().date_naive(),
        }
    };

    let insight_data = match period.as_str() {
        "week" => InsightQueries::calculate_weekly_summary(&db.connection, start_date)?,
        "month" => InsightQueries::calculate_monthly_summary(&db.connection, start_date)?,
        _ => return Err(anyhow::anyhow!("Invalid period. Use 'week' or 'month'")),
    };

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m         \x1b[1;37m{} Summary\x1b[0m                  \x1b[36m│\x1b[0m",
        period
    );
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m Total Hours:  \x1b[32m{:<23}\x1b[0m \x1b[36m│\x1b[0m",
        format!("{:.1}h", insight_data.total_hours)
    );
    println!(
        "\x1b[36m│\x1b[0m Sessions:     \x1b[33m{:<23}\x1b[0m \x1b[36m│\x1b[0m",
        insight_data.sessions_count
    );
    println!(
        "\x1b[36m│\x1b[0m Avg Session:  \x1b[33m{:<23}\x1b[0m \x1b[36m│\x1b[0m",
        format!("{:.1}h", insight_data.avg_session_duration)
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    Ok(())
}

async fn compare_projects(
    projects: String,
    _from: Option<String>,
    _to: Option<String>,
) -> Result<()> {
    let _project_names: Vec<&str> = projects.split(',').map(|s| s.trim()).collect();

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m        \x1b[1;37mProject Comparison\x1b[0m                \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m Projects: \x1b[33m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
        truncate_string(&projects, 27)
    );
    println!(
        "\x1b[36m│\x1b[0m \x1b[33m⚠  Comparison feature in development\x1b[0m    \x1b[36m│\x1b[0m"
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    Ok(())
}

async fn handle_estimate_action(action: EstimateAction) -> Result<()> {
    match action {
        EstimateAction::Create {
            project,
            task,
            hours,
            due_date,
        } => create_estimate(project, task, hours, due_date).await,
        EstimateAction::Record { id, hours } => record_actual_time(id, hours).await,
        EstimateAction::List { project } => list_estimates(project).await,
    }
}

async fn create_estimate(
    project: String,
    task: String,
    hours: f64,
    due_date: Option<String>,
) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    let project_obj = ProjectQueries::find_by_name(&db.connection, &project)?
        .ok_or_else(|| anyhow::anyhow!("Project '{}' not found", project))?;

    let due = due_date.and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());

    let mut estimate = TimeEstimate::new(project_obj.id.unwrap(), task.clone(), hours);
    estimate.due_date = due;

    let estimate_id = TimeEstimateQueries::create(&db.connection, &estimate)?;

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m      \x1b[1;37mTime Estimate Created\x1b[0m              \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m Task:      \x1b[1;33m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
        truncate_string(&task, 27)
    );
    println!(
        "\x1b[36m│\x1b[0m Estimate:  \x1b[32m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
        format!("{} hours", hours)
    );
    println!(
        "\x1b[36m│\x1b[0m ID:        \x1b[90m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
        estimate_id
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    Ok(())
}

async fn record_actual_time(id: i64, hours: f64) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    TimeEstimateQueries::record_actual(&db.connection, id, hours)?;
    println!(
        "\x1b[32m✓ Recorded {} hours for estimate {}\x1b[0m",
        hours, id
    );
    Ok(())
}

async fn list_estimates(project: String) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    let project_obj = ProjectQueries::find_by_name(&db.connection, &project)?
        .ok_or_else(|| anyhow::anyhow!("Project '{}' not found", project))?;

    let estimates = TimeEstimateQueries::list_by_project(&db.connection, project_obj.id.unwrap())?;

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m          \x1b[1;37mTime Estimates\x1b[0m                  \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");

    for est in &estimates {
        let variance = est.variance();
        let variance_str = if let Some(v) = variance {
            if v > 0.0 {
                format!("\x1b[31m+{:.1}h over\x1b[0m", v)
            } else {
                format!("\x1b[32m{:.1}h under\x1b[0m", v.abs())
            }
        } else {
            "N/A".to_string()
        };

        println!(
            "\x1b[36m│\x1b[0m 📋 \x1b[1;33m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(&est.task_name, 25)
        );
        let actual_str = est
            .actual_hours
            .map(|h| format!("{:.1}h", h))
            .unwrap_or_else(|| "N/A".to_string());
        println!(
            "\x1b[36m│\x1b[0m    Est: {}h | Actual: {} | {}  \x1b[36m│\x1b[0m",
            est.estimated_hours, actual_str, variance_str
        );
        println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    }

    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    Ok(())
}

async fn handle_branch_action(action: BranchAction) -> Result<()> {
    match action {
        BranchAction::List { project } => list_branches(project).await,
        BranchAction::Stats { project, branch } => show_branch_stats(project, branch).await,
    }
}

async fn list_branches(project: String) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    let project_obj = ProjectQueries::find_by_name(&db.connection, &project)?
        .ok_or_else(|| anyhow::anyhow!("Project '{}' not found", project))?;

    let branches = GitBranchQueries::list_by_project(&db.connection, project_obj.id.unwrap())?;

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m          \x1b[1;37mGit Branches\x1b[0m                   \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");

    for branch in &branches {
        println!(
            "\x1b[36m│\x1b[0m 🌿 \x1b[1;33m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(&branch.branch_name, 25)
        );
        println!(
            "\x1b[36m│\x1b[0m    Time: \x1b[32m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
            format!("{:.1}h", branch.total_hours())
        );
        println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
    }

    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    Ok(())
}

async fn show_branch_stats(project: String, branch: Option<String>) -> Result<()> {
    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m        \x1b[1;37mBranch Statistics\x1b[0m                \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m Project:  \x1b[33m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
        truncate_string(&project, 27)
    );
    if let Some(b) = branch {
        println!(
            "\x1b[36m│\x1b[0m Branch:   \x1b[33m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(&b, 27)
        );
    }
    println!(
        "\x1b[36m│\x1b[0m \x1b[33m⚠  Branch stats in development\x1b[0m         \x1b[36m│\x1b[0m"
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    Ok(())
}

// Template management functions
async fn handle_template_action(action: TemplateAction) -> Result<()> {
    match action {
        TemplateAction::Create {
            name,
            description,
            tags,
            workspace_path,
        } => create_template(name, description, tags, workspace_path).await,
        TemplateAction::List => list_templates().await,
        TemplateAction::Delete { template } => delete_template(template).await,
        TemplateAction::Use {
            template,
            project_name,
            path,
        } => use_template(template, project_name, path).await,
    }
}

async fn create_template(
    name: String,
    description: Option<String>,
    tags: Option<String>,
    workspace_path: Option<PathBuf>,
) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    let default_tags = tags
        .map(|t| t.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    let mut template = ProjectTemplate::new(name.clone()).with_tags(default_tags);

    let desc_clone = description.clone();
    if let Some(desc) = description {
        template = template.with_description(desc);
    }
    if let Some(path) = workspace_path {
        template = template.with_workspace_path(path);
    }

    let _template_id = TemplateQueries::create(&db.connection, &template)?;

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m         \x1b[1;37mTemplate Created\x1b[0m                  \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m Name:     \x1b[1;33m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
        truncate_string(&name, 27)
    );
    if let Some(desc) = &desc_clone {
        println!(
            "\x1b[36m│\x1b[0m Desc:     \x1b[2;37m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(desc, 27)
        );
    }
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    Ok(())
}

async fn list_templates() -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    let templates = TemplateQueries::list_all(&db.connection)?;

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m          \x1b[1;37mTemplates\x1b[0m                      \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");

    if templates.is_empty() {
        println!("\x1b[36m│\x1b[0m No templates found.                      \x1b[36m│\x1b[0m");
    } else {
        for template in &templates {
            println!(
                "\x1b[36m│\x1b[0m 📋 \x1b[1;33m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
                truncate_string(&template.name, 25)
            );
            if let Some(desc) = &template.description {
                println!(
                    "\x1b[36m│\x1b[0m    \x1b[2;37m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
                    truncate_string(desc, 27)
                );
            }
            println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
        }
    }

    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    Ok(())
}

async fn delete_template(_template: String) -> Result<()> {
    println!("\x1b[33m⚠  Template deletion not yet implemented\x1b[0m");
    Ok(())
}

async fn use_template(template: String, project_name: String, path: Option<PathBuf>) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    let templates = TemplateQueries::list_all(&db.connection)?;
    let selected_template = templates
        .iter()
        .find(|t| t.name == template || t.id.map(|id| id.to_string()) == Some(template.clone()))
        .ok_or_else(|| anyhow::anyhow!("Template '{}' not found", template))?;

    // Initialize project with template
    let project_path = path.unwrap_or_else(|| env::current_dir().unwrap());
    let canonical_path = canonicalize_path(&project_path)?;

    // Check if project already exists
    if ProjectQueries::find_by_path(&db.connection, &canonical_path)?.is_some() {
        return Err(anyhow::anyhow!("Project already exists at this path"));
    }

    let git_hash = if is_git_repository(&canonical_path) {
        get_git_hash(&canonical_path)
    } else {
        None
    };

    let template_desc = selected_template.description.clone();
    let mut project = Project::new(project_name.clone(), canonical_path.clone())
        .with_git_hash(git_hash)
        .with_description(template_desc);

    let project_id = ProjectQueries::create(&db.connection, &project)?;
    project.id = Some(project_id);

    // Apply template tags (project-tag associations not yet implemented)
    // TODO: Implement project_tags table operations

    // Apply template goals
    for goal_def in &selected_template.default_goals {
        let mut goal =
            Goal::new(goal_def.name.clone(), goal_def.target_hours).with_project(project_id);
        if let Some(desc) = &goal_def.description {
            goal = goal.with_description(desc.clone());
        }
        GoalQueries::create(&db.connection, &goal)?;
    }

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m    \x1b[1;37mProject Created from Template\x1b[0m          \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m Template: \x1b[33m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
        truncate_string(&selected_template.name, 27)
    );
    println!(
        "\x1b[36m│\x1b[0m Project:   \x1b[33m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
        truncate_string(&project_name, 27)
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    Ok(())
}

// Workspace management functions
async fn handle_workspace_action(action: WorkspaceAction) -> Result<()> {
    match action {
        WorkspaceAction::Create {
            name,
            description,
            path,
        } => create_workspace(name, description, path).await,
        WorkspaceAction::List => list_workspaces().await,
        WorkspaceAction::AddProject { workspace, project } => {
            add_project_to_workspace(workspace, project).await
        }
        WorkspaceAction::RemoveProject { workspace, project } => {
            remove_project_from_workspace(workspace, project).await
        }
        WorkspaceAction::Projects { workspace } => list_workspace_projects(workspace).await,
        WorkspaceAction::Delete { workspace } => delete_workspace(workspace).await,
    }
}

async fn create_workspace(
    name: String,
    description: Option<String>,
    path: Option<PathBuf>,
) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    let mut workspace = Workspace::new(name.clone());
    let desc_clone = description.clone();
    if let Some(desc) = description {
        workspace = workspace.with_description(desc);
    }
    if let Some(p) = path {
        workspace = workspace.with_path(p);
    }

    let _workspace_id = WorkspaceQueries::create(&db.connection, &workspace)?;

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m        \x1b[1;37mWorkspace Created\x1b[0m                  \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m Name:     \x1b[1;33m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
        truncate_string(&name, 27)
    );
    if let Some(desc) = &desc_clone {
        println!(
            "\x1b[36m│\x1b[0m Desc:     \x1b[2;37m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(desc, 27)
        );
    }
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    Ok(())
}

async fn list_workspaces() -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    let workspaces = WorkspaceQueries::list_all(&db.connection)?;

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m          \x1b[1;37mWorkspaces\x1b[0m                      \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");

    if workspaces.is_empty() {
        println!("\x1b[36m│\x1b[0m No workspaces found.                     \x1b[36m│\x1b[0m");
    } else {
        for workspace in &workspaces {
            println!(
                "\x1b[36m│\x1b[0m 📁 \x1b[1;33m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
                truncate_string(&workspace.name, 25)
            );
            if let Some(desc) = &workspace.description {
                println!(
                    "\x1b[36m│\x1b[0m    \x1b[2;37m{:<27}\x1b[0m \x1b[36m│\x1b[0m",
                    truncate_string(desc, 27)
                );
            }
            println!("\x1b[36m│\x1b[0m                                         \x1b[36m│\x1b[0m");
        }
    }

    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    Ok(())
}

async fn add_project_to_workspace(workspace: String, project: String) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    // Find workspace by name
    let workspace_obj = WorkspaceQueries::find_by_name(&db.connection, &workspace)?
        .ok_or_else(|| anyhow::anyhow!("Workspace '{}' not found", workspace))?;

    // Find project by name
    let project_obj = ProjectQueries::find_by_name(&db.connection, &project)?
        .ok_or_else(|| anyhow::anyhow!("Project '{}' not found", project))?;

    let workspace_id = workspace_obj
        .id
        .ok_or_else(|| anyhow::anyhow!("Workspace ID is missing"))?;
    let project_id = project_obj
        .id
        .ok_or_else(|| anyhow::anyhow!("Project ID is missing"))?;

    if WorkspaceQueries::add_project(&db.connection, workspace_id, project_id)? {
        println!(
            "\x1b[32m✓\x1b[0m Added project '\x1b[33m{}\x1b[0m' to workspace '\x1b[33m{}\x1b[0m'",
            project, workspace
        );
    } else {
        println!("\x1b[33m⚠\x1b[0m Project '\x1b[33m{}\x1b[0m' is already in workspace '\x1b[33m{}\x1b[0m'", project, workspace);
    }

    Ok(())
}

async fn remove_project_from_workspace(workspace: String, project: String) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    // Find workspace by name
    let workspace_obj = WorkspaceQueries::find_by_name(&db.connection, &workspace)?
        .ok_or_else(|| anyhow::anyhow!("Workspace '{}' not found", workspace))?;

    // Find project by name
    let project_obj = ProjectQueries::find_by_name(&db.connection, &project)?
        .ok_or_else(|| anyhow::anyhow!("Project '{}' not found", project))?;

    let workspace_id = workspace_obj
        .id
        .ok_or_else(|| anyhow::anyhow!("Workspace ID is missing"))?;
    let project_id = project_obj
        .id
        .ok_or_else(|| anyhow::anyhow!("Project ID is missing"))?;

    if WorkspaceQueries::remove_project(&db.connection, workspace_id, project_id)? {
        println!("\x1b[32m✓\x1b[0m Removed project '\x1b[33m{}\x1b[0m' from workspace '\x1b[33m{}\x1b[0m'", project, workspace);
    } else {
        println!(
            "\x1b[33m⚠\x1b[0m Project '\x1b[33m{}\x1b[0m' was not in workspace '\x1b[33m{}\x1b[0m'",
            project, workspace
        );
    }

    Ok(())
}

async fn list_workspace_projects(workspace: String) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    // Find workspace by name
    let workspace_obj = WorkspaceQueries::find_by_name(&db.connection, &workspace)?
        .ok_or_else(|| anyhow::anyhow!("Workspace '{}' not found", workspace))?;

    let workspace_id = workspace_obj
        .id
        .ok_or_else(|| anyhow::anyhow!("Workspace ID is missing"))?;
    let projects = WorkspaceQueries::list_projects(&db.connection, workspace_id)?;

    if projects.is_empty() {
        println!(
            "\x1b[33m⚠\x1b[0m No projects found in workspace '\x1b[33m{}\x1b[0m'",
            workspace
        );
        return Ok(());
    }

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m        \x1b[1;37mWorkspace Projects\x1b[0m               \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m Workspace: \x1b[33m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
        truncate_string(&workspace, 25)
    );
    println!(
        "\x1b[36m│\x1b[0m Projects:  \x1b[32m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
        format!("{} projects", projects.len())
    );
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");

    for project in &projects {
        let status_indicator = if !project.is_archived {
            "\x1b[32m●\x1b[0m"
        } else {
            "\x1b[31m○\x1b[0m"
        };
        println!(
            "\x1b[36m│\x1b[0m {} \x1b[37m{:<33}\x1b[0m \x1b[36m│\x1b[0m",
            status_indicator,
            truncate_string(&project.name, 33)
        );
    }

    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
    Ok(())
}

async fn delete_workspace(workspace: String) -> Result<()> {
    let db_path = get_database_path()?;
    let db = Database::new(&db_path)?;

    // Find workspace by name
    let workspace_obj = WorkspaceQueries::find_by_name(&db.connection, &workspace)?
        .ok_or_else(|| anyhow::anyhow!("Workspace '{}' not found", workspace))?;

    let workspace_id = workspace_obj
        .id
        .ok_or_else(|| anyhow::anyhow!("Workspace ID is missing"))?;

    // Check if workspace has projects
    let projects = WorkspaceQueries::list_projects(&db.connection, workspace_id)?;
    if !projects.is_empty() {
        println!("\x1b[33m⚠\x1b[0m Cannot delete workspace '\x1b[33m{}\x1b[0m' - it contains {} project(s). Remove projects first.", 
                workspace, projects.len());
        return Ok(());
    }

    if WorkspaceQueries::delete(&db.connection, workspace_id)? {
        println!(
            "\x1b[32m✓\x1b[0m Deleted workspace '\x1b[33m{}\x1b[0m'",
            workspace
        );
    } else {
        println!(
            "\x1b[31m✗\x1b[0m Failed to delete workspace '\x1b[33m{}\x1b[0m'",
            workspace
        );
    }

    Ok(())
}

// Calendar integration functions
async fn handle_calendar_action(action: CalendarAction) -> Result<()> {
    match action {
        CalendarAction::Add {
            name,
            start,
            end,
            event_type,
            project,
            description,
        } => add_calendar_event(name, start, end, event_type, project, description).await,
        CalendarAction::List { from, to, project } => list_calendar_events(from, to, project).await,
        CalendarAction::Delete { id } => delete_calendar_event(id).await,
    }
}

async fn add_calendar_event(
    _name: String,
    _start: String,
    _end: Option<String>,
    _event_type: Option<String>,
    _project: Option<String>,
    _description: Option<String>,
) -> Result<()> {
    println!("\x1b[33m⚠  Calendar integration in development\x1b[0m");
    Ok(())
}

async fn list_calendar_events(
    _from: Option<String>,
    _to: Option<String>,
    _project: Option<String>,
) -> Result<()> {
    println!("\x1b[33m⚠  Calendar integration in development\x1b[0m");
    Ok(())
}

async fn delete_calendar_event(_id: i64) -> Result<()> {
    println!("\x1b[33m⚠  Calendar integration in development\x1b[0m");
    Ok(())
}

// Issue tracker integration functions
async fn handle_issue_action(action: IssueAction) -> Result<()> {
    match action {
        IssueAction::Sync {
            project,
            tracker_type,
        } => sync_issues(project, tracker_type).await,
        IssueAction::List { project, status } => list_issues(project, status).await,
        IssueAction::Link {
            session_id,
            issue_id,
        } => link_session_to_issue(session_id, issue_id).await,
    }
}

async fn sync_issues(_project: String, _tracker_type: Option<String>) -> Result<()> {
    println!("\x1b[33m⚠  Issue tracker integration in development\x1b[0m");
    Ok(())
}

async fn list_issues(_project: String, _status: Option<String>) -> Result<()> {
    println!("\x1b[33m⚠  Issue tracker integration in development\x1b[0m");
    Ok(())
}

async fn link_session_to_issue(_session_id: i64, _issue_id: String) -> Result<()> {
    println!("\x1b[33m⚠  Issue tracker integration in development\x1b[0m");
    Ok(())
}

// Client reporting functions
async fn handle_client_action(action: ClientAction) -> Result<()> {
    match action {
        ClientAction::Generate {
            client,
            from,
            to,
            projects,
            format,
        } => generate_client_report(client, from, to, projects, format).await,
        ClientAction::List { client } => list_client_reports(client).await,
        ClientAction::View { id } => view_client_report(id).await,
    }
}

async fn generate_client_report(
    _client: String,
    _from: String,
    _to: String,
    _projects: Option<String>,
    _format: Option<String>,
) -> Result<()> {
    println!("\x1b[33m⚠  Client reporting in development\x1b[0m");
    Ok(())
}

async fn list_client_reports(_client: Option<String>) -> Result<()> {
    println!("\x1b[33m⚠  Client reporting in development\x1b[0m");
    Ok(())
}

async fn view_client_report(_id: i64) -> Result<()> {
    println!("\x1b[33m⚠  Client reporting in development\x1b[0m");
    Ok(())
}

fn should_quit(event: crossterm::event::Event) -> bool {
    match event {
        crossterm::event::Event::Key(key) if key.kind == crossterm::event::KeyEventKind::Press => {
            matches!(
                key.code,
                crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Esc
            )
        }
        _ => false,
    }
}

// Helper function for init_project with database connection
async fn init_project_with_db(
    name: Option<String>,
    canonical_path: Option<PathBuf>,
    description: Option<String>,
    conn: &rusqlite::Connection,
) -> Result<()> {
    let canonical_path =
        canonical_path.ok_or_else(|| anyhow::anyhow!("Canonical path required"))?;
    let project_name = name.unwrap_or_else(|| detect_project_name(&canonical_path));

    // Check if project already exists
    if let Some(existing) = ProjectQueries::find_by_path(conn, &canonical_path)? {
        println!(
            "\x1b[33m⚠  Project already exists:\x1b[0m {}",
            existing.name
        );
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
    let project_id = ProjectQueries::create(conn, &project)?;
    project.id = Some(project_id);

    // Create .tempo marker file
    let marker_path = canonical_path.join(".tempo");
    if !marker_path.exists() {
        std::fs::write(
            &marker_path,
            format!("# Tempo time tracking project\nname: {}\n", project_name),
        )?;
    }

    println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
    println!("\x1b[36m│\x1b[0m         \x1b[1;37mProject Initialized\x1b[0m               \x1b[36m│\x1b[0m");
    println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
    println!(
        "\x1b[36m│\x1b[0m Name:        \x1b[33m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
        truncate_string(&project_name, 25)
    );
    println!(
        "\x1b[36m│\x1b[0m Path:        \x1b[37m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
        truncate_string(&canonical_path.display().to_string(), 25)
    );

    if let Some(desc) = &description {
        println!(
            "\x1b[36m│\x1b[0m Description: \x1b[37m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
            truncate_string(desc, 25)
        );
    }

    if is_git_repository(&canonical_path) {
        println!(
            "\x1b[36m│\x1b[0m Git:         \x1b[32m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
            "Repository detected"
        );
        if let Some(hash) = &git_hash {
            println!(
                "\x1b[36m│\x1b[0m Git Hash:    \x1b[37m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
                truncate_string(hash, 25)
            );
        }
    }

    println!(
        "\x1b[36m│\x1b[0m ID:          \x1b[37m{:<25}\x1b[0m \x1b[36m│\x1b[0m",
        project_id
    );
    println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");

    Ok(())
}

// Show database connection pool statistics
async fn show_pool_stats() -> Result<()> {
    match get_pool_stats() {
        Ok(stats) => {
            println!("\x1b[36m┌─────────────────────────────────────────┐\x1b[0m");
            println!("\x1b[36m│\x1b[0m        \x1b[1;37mDatabase Pool Statistics\x1b[0m          \x1b[36m│\x1b[0m");
            println!("\x1b[36m├─────────────────────────────────────────┤\x1b[0m");
            println!(
                "\x1b[36m│\x1b[0m Total Created:    \x1b[32m{:<19}\x1b[0m \x1b[36m│\x1b[0m",
                stats.total_connections_created
            );
            println!(
                "\x1b[36m│\x1b[0m Active:           \x1b[33m{:<19}\x1b[0m \x1b[36m│\x1b[0m",
                stats.active_connections
            );
            println!(
                "\x1b[36m│\x1b[0m Available in Pool:\x1b[37m{:<19}\x1b[0m \x1b[36m│\x1b[0m",
                stats.connections_in_pool
            );
            println!(
                "\x1b[36m│\x1b[0m Total Requests:   \x1b[37m{:<19}\x1b[0m \x1b[36m│\x1b[0m",
                stats.connection_requests
            );
            println!(
                "\x1b[36m│\x1b[0m Timeouts:         \x1b[31m{:<19}\x1b[0m \x1b[36m│\x1b[0m",
                stats.connection_timeouts
            );
            println!("\x1b[36m└─────────────────────────────────────────┘\x1b[0m");
        }
        Err(_) => {
            println!("\x1b[33m⚠  Database pool not initialized or not available\x1b[0m");
            println!("   Using direct database connections as fallback");
        }
    }
    Ok(())
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: String,
    body: String,
    published_at: String,
    prerelease: bool,
}

async fn handle_update(check: bool, force: bool, verbose: bool) -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    
    if verbose {
        println!("🔍 Current version: v{}", current_version);
        println!("📡 Checking for updates...");
    } else {
        println!("🔍 Checking for updates...");
    }

    // Fetch latest release information from GitHub
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/repos/own-path/vibe/releases/latest")
        .header("User-Agent", format!("tempo-cli/{}", current_version))
        .send()
        .await
        .context("Failed to fetch release information")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to fetch release information: HTTP {}",
            response.status()
        ));
    }

    let release: GitHubRelease = response
        .json()
        .await
        .context("Failed to parse release information")?;

    let latest_version = release.tag_name.trim_start_matches('v');
    
    if verbose {
        println!("📦 Latest version: v{}", latest_version);
        println!("📅 Released: {}", release.published_at);
    }

    // Compare versions
    let current_semver = semver::Version::parse(current_version)
        .context("Failed to parse current version")?;
    let latest_semver = semver::Version::parse(latest_version)
        .context("Failed to parse latest version")?;

    if current_semver >= latest_semver && !force {
        println!("✅ You're already running the latest version (v{})", current_version);
        if check {
            return Ok(());
        }
        
        if !force {
            println!("💡 Use --force to reinstall the current version");
            return Ok(());
        }
    }

    if check {
        if current_semver < latest_semver {
            println!("📦 Update available: v{} → v{}", current_version, latest_version);
            println!("🔗 Run `tempo update` to install the latest version");
            
            if verbose && !release.body.is_empty() {
                println!("\n📝 Release Notes:");
                println!("{}", release.body);
            }
        }
        return Ok(());
    }

    if current_semver < latest_semver || force {
        println!("⬇️  Updating tempo from v{} to v{}", current_version, latest_version);
        
        if verbose {
            println!("🔧 Installing via cargo...");
        }
        
        // Update using cargo install
        let mut cmd = Command::new("cargo");
        cmd.args(&["install", "tempo-cli", "--force"]);
        
        if verbose {
            cmd.stdout(Stdio::inherit())
               .stderr(Stdio::inherit());
        } else {
            cmd.stdout(Stdio::null())
               .stderr(Stdio::null());
        }

        let status = cmd.status()
            .context("Failed to run cargo install command")?;

        if status.success() {
            println!("✅ Successfully updated tempo to v{}", latest_version);
            println!("🎉 You can now use the latest features!");
            
            if !release.body.is_empty() && verbose {
                println!("\n📝 What's new in v{}:", latest_version);
                println!("{}", release.body);
            }
        } else {
            return Err(anyhow::anyhow!("Failed to install update. Try running manually: cargo install tempo-cli --force"));
        }
    }

    Ok(())
}
