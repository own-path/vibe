use clap::{Parser, Subcommand, ValueEnum, CommandFactory};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "tempo")]
#[command(about = "Automatic project time tracking CLI tool")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(author = "Tempo Contributors")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(long, short, help = "Path to config file")]
    pub config: Option<PathBuf>,

    #[arg(long, short, help = "Verbose output")]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Start the daemon")]
    Start,

    #[command(about = "Stop the daemon")]
    Stop,

    #[command(about = "Restart the daemon")]
    Restart,

    #[command(about = "Check daemon status")]
    Status,

    #[command(about = "Initialize a project for tracking")]
    Init {
        #[arg(help = "Project name")]
        name: Option<String>,
        
        #[arg(long, help = "Project path")]
        path: Option<PathBuf>,
        
        #[arg(long, help = "Project description")]
        description: Option<String>,
    },

    #[command(about = "List projects")]
    List {
        #[arg(long, help = "Include archived projects")]
        all: bool,
        
        #[arg(long, help = "Filter by tag")]
        tag: Option<String>,
    },

    #[command(about = "Generate time reports")]
    Report {
        #[arg(help = "Project name or ID")]
        project: Option<String>,
        
        #[arg(long, help = "Start date (YYYY-MM-DD)")]
        from: Option<String>,
        
        #[arg(long, help = "End date (YYYY-MM-DD)")]
        to: Option<String>,
        
        #[arg(long, help = "Export format (csv, json)")]
        format: Option<String>,
        
        #[arg(long, help = "Group by (day, week, month, project)")]
        group: Option<String>,
    },

    #[command(about = "Project management")]
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },

    #[command(about = "Session management")]
    Session {
        #[command(subcommand)]
        action: SessionAction,
    },

    #[command(about = "Tag management")]
    Tag {
        #[command(subcommand)]
        action: TagAction,
    },

    #[command(about = "Configuration management")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    #[command(about = "Interactive dashboard")]
    Dashboard,

    #[command(about = "Interactive project and session viewer")]
    Tui,
    
    #[command(about = "Interactive timer with visual progress")]
    Timer,

    #[command(about = "Browse session history")]
    History,

    #[command(about = "Goal management")]
    Goal {
        #[command(subcommand)]
        action: GoalAction,
    },

    #[command(about = "Productivity insights and analytics")]
    Insights {
        #[arg(long, help = "Period (daily, weekly, monthly)")]
        period: Option<String>,
        
        #[arg(long, help = "Project name or ID")]
        project: Option<String>,
    },

    #[command(about = "Weekly or monthly summary")]
    Summary {
        #[arg(help = "Period type (week, month)")]
        period: String,
        
        #[arg(long, help = "Start date (YYYY-MM-DD)")]
        from: Option<String>,
    },

    #[command(about = "Compare projects")]
    Compare {
        #[arg(help = "Project names or IDs (comma-separated)")]
        projects: String,
        
        #[arg(long, help = "Start date (YYYY-MM-DD)")]
        from: Option<String>,
        
        #[arg(long, help = "End date (YYYY-MM-DD)")]
        to: Option<String>,
    },

    #[command(about = "Show database pool statistics")]
    PoolStats,

    #[command(about = "Time estimation tracking")]
    Estimate {
        #[command(subcommand)]
        action: EstimateAction,
    },

    #[command(about = "Git branch tracking")]
    Branch {
        #[command(subcommand)]
        action: BranchAction,
    },

    #[command(about = "Project templates")]
    Template {
        #[command(subcommand)]
        action: TemplateAction,
    },

    #[command(about = "Workspace management")]
    Workspace {
        #[command(subcommand)]
        action: WorkspaceAction,
    },

    #[command(about = "Calendar integration")]
    Calendar {
        #[command(subcommand)]
        action: CalendarAction,
    },

    #[command(about = "Issue tracker integration")]
    Issue {
        #[command(subcommand)]
        action: IssueAction,
    },

    #[command(about = "Client reporting")]
    Client {
        #[command(subcommand)]
        action: ClientAction,
    },

    #[command(about = "Update tempo to the latest version")]
    Update {
        #[arg(long, help = "Check for updates without installing")]
        check: bool,
        
        #[arg(long, help = "Force update even if current version is latest")]
        force: bool,
        
        #[arg(long, help = "Show detailed update information")]
        verbose: bool,
    },

    #[command(about = "Generate shell completions", hide = true)]
    Completions {
        #[arg(help = "Shell to generate completions for")]
        shell: Shell,
    },
}

#[derive(Subcommand)]
pub enum ProjectAction {
    #[command(about = "Archive a project")]
    Archive {
        #[arg(help = "Project name or ID")]
        project: String,
    },
    
    #[command(about = "Unarchive a project")]
    Unarchive {
        #[arg(help = "Project name or ID")]
        project: String,
    },
    
    #[command(about = "Update project path")]
    UpdatePath {
        #[arg(help = "Project name or ID")]
        project: String,
        
        #[arg(help = "New path")]
        path: PathBuf,
    },
    
    #[command(about = "Add tag to project")]
    AddTag {
        #[arg(help = "Project name or ID")]
        project: String,
        
        #[arg(help = "Tag name")]
        tag: String,
    },
    
    #[command(about = "Remove tag from project")]
    RemoveTag {
        #[arg(help = "Project name or ID")]
        project: String,
        
        #[arg(help = "Tag name")]
        tag: String,
    },
}

#[derive(Subcommand)]
pub enum SessionAction {
    #[command(about = "Start tracking time for current project")]
    Start {
        #[arg(long, help = "Project name or path")]
        project: Option<String>,
        
        #[arg(long, help = "Session context")]
        context: Option<String>,
    },
    
    #[command(about = "Stop current session")]
    Stop,
    
    #[command(about = "Pause current session")]
    Pause,
    
    #[command(about = "Resume paused session")]
    Resume,
    
    #[command(about = "Show current session")]
    Current,
    
    #[command(about = "List recent sessions")]
    List {
        #[arg(long, help = "Number of sessions to show")]
        limit: Option<usize>,
        
        #[arg(long, help = "Project filter")]
        project: Option<String>,
    },
    
    #[command(about = "Edit a session")]
    Edit {
        #[arg(help = "Session ID")]
        id: i64,
        
        #[arg(long, help = "New start time")]
        start: Option<String>,
        
        #[arg(long, help = "New end time")]
        end: Option<String>,
        
        #[arg(long, help = "New project")]
        project: Option<String>,
        
        #[arg(long, help = "Edit reason")]
        reason: Option<String>,
    },
    
    #[command(about = "Delete a session")]
    Delete {
        #[arg(help = "Session ID")]
        id: i64,
        
        #[arg(long, help = "Force deletion without confirmation")]
        force: bool,
    },
    
    #[command(about = "Merge multiple sessions into one")]
    Merge {
        #[arg(help = "Session IDs to merge (comma-separated)")]
        session_ids: String,
        
        #[arg(long, help = "Target project for merged session")]
        project: Option<String>,
        
        #[arg(long, help = "Notes for the merged session")]
        notes: Option<String>,
    },
    
    #[command(about = "Split a session into multiple sessions")]
    Split {
        #[arg(help = "Session ID to split")]
        session_id: i64,
        
        #[arg(help = "Split points (comma-separated times like '10:30,11:45')")]
        split_times: String,
        
        #[arg(long, help = "Notes for each split session (comma-separated)")]
        notes: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum TagAction {
    #[command(about = "Create a new tag")]
    Create {
        #[arg(help = "Tag name")]
        name: String,
        
        #[arg(long, help = "Tag color")]
        color: Option<String>,
        
        #[arg(long, help = "Tag description")]
        description: Option<String>,
    },
    
    #[command(about = "List all tags")]
    List,
    
    #[command(about = "Delete a tag")]
    Delete {
        #[arg(help = "Tag name")]
        name: String,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    #[command(about = "Show current configuration")]
    Show,
    
    #[command(about = "Set configuration value")]
    Set {
        #[arg(help = "Configuration key")]
        key: String,
        
        #[arg(help = "Configuration value")]
        value: String,
    },
    
    #[command(about = "Get configuration value")]
    Get {
        #[arg(help = "Configuration key")]
        key: String,
    },
    
    #[command(about = "Reset configuration to defaults")]
    Reset,
}

#[derive(Subcommand)]
pub enum GoalAction {
    #[command(about = "Create a new goal")]
    Create {
        #[arg(help = "Goal name")]
        name: String,
        
        #[arg(help = "Target hours")]
        target_hours: f64,
        
        #[arg(long, help = "Project name or ID")]
        project: Option<String>,
        
        #[arg(long, help = "Goal description")]
        description: Option<String>,
        
        #[arg(long, help = "Start date (YYYY-MM-DD)")]
        start_date: Option<String>,
        
        #[arg(long, help = "End date (YYYY-MM-DD)")]
        end_date: Option<String>,
    },
    
    #[command(about = "List goals")]
    List {
        #[arg(long, help = "Project name or ID")]
        project: Option<String>,
    },
    
    #[command(about = "Update goal progress")]
    Update {
        #[arg(help = "Goal ID")]
        id: i64,
        
        #[arg(help = "Hours to add")]
        hours: f64,
    },
}

#[derive(Subcommand)]
pub enum EstimateAction {
    #[command(about = "Create a time estimate")]
    Create {
        #[arg(help = "Project name or ID")]
        project: String,
        
        #[arg(help = "Task name")]
        task: String,
        
        #[arg(help = "Estimated hours")]
        hours: f64,
        
        #[arg(long, help = "Due date (YYYY-MM-DD)")]
        due_date: Option<String>,
    },
    
    #[command(about = "Record actual time")]
    Record {
        #[arg(help = "Estimate ID")]
        id: i64,
        
        #[arg(help = "Actual hours")]
        hours: f64,
    },
    
    #[command(about = "List estimates")]
    List {
        #[arg(help = "Project name or ID")]
        project: String,
    },
}

#[derive(Subcommand)]
pub enum BranchAction {
    #[command(about = "List git branches for a project")]
    List {
        #[arg(help = "Project name or ID")]
        project: String,
    },
    
    #[command(about = "Show branch statistics")]
    Stats {
        #[arg(help = "Project name or ID")]
        project: String,
        
        #[arg(long, help = "Branch name")]
        branch: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum TemplateAction {
    #[command(about = "Create a new project template")]
    Create {
        #[arg(help = "Template name")]
        name: String,
        
        #[arg(long, help = "Template description")]
        description: Option<String>,
        
        #[arg(long, help = "Default tags (comma-separated)")]
        tags: Option<String>,
        
        #[arg(long, help = "Workspace path for template")]
        workspace_path: Option<PathBuf>,
    },
    
    #[command(about = "List all templates")]
    List,
    
    #[command(about = "Delete a template")]
    Delete {
        #[arg(help = "Template name or ID")]
        template: String,
    },
    
    #[command(about = "Use a template to initialize a project")]
    Use {
        #[arg(help = "Template name or ID")]
        template: String,
        
        #[arg(help = "Project name")]
        project_name: String,
        
        #[arg(long, help = "Project path")]
        path: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum WorkspaceAction {
    #[command(about = "Create a new workspace")]
    Create {
        #[arg(help = "Workspace name")]
        name: String,
        
        #[arg(long, help = "Workspace description")]
        description: Option<String>,
        
        #[arg(long, help = "Workspace path")]
        path: Option<PathBuf>,
    },
    
    #[command(about = "List all workspaces")]
    List,
    
    #[command(about = "Add project to workspace")]
    AddProject {
        #[arg(help = "Workspace name or ID")]
        workspace: String,
        
        #[arg(help = "Project name or ID")]
        project: String,
    },
    
    #[command(about = "Remove project from workspace")]
    RemoveProject {
        #[arg(help = "Workspace name or ID")]
        workspace: String,
        
        #[arg(help = "Project name or ID")]
        project: String,
    },
    
    #[command(about = "List projects in workspace")]
    Projects {
        #[arg(help = "Workspace name or ID")]
        workspace: String,
    },
    
    #[command(about = "Delete a workspace")]
    Delete {
        #[arg(help = "Workspace name or ID")]
        workspace: String,
    },
}

#[derive(Subcommand)]
pub enum CalendarAction {
    #[command(about = "Add a calendar event")]
    Add {
        #[arg(help = "Event name")]
        name: String,
        
        #[arg(help = "Start time (YYYY-MM-DD HH:MM)")]
        start: String,
        
        #[arg(long, help = "End time (YYYY-MM-DD HH:MM)")]
        end: Option<String>,
        
        #[arg(long, help = "Event type (meeting, focus_block, deadline)")]
        event_type: Option<String>,
        
        #[arg(long, help = "Project name or ID")]
        project: Option<String>,
        
        #[arg(long, help = "Event description")]
        description: Option<String>,
    },
    
    #[command(about = "List calendar events")]
    List {
        #[arg(long, help = "Start date (YYYY-MM-DD)")]
        from: Option<String>,
        
        #[arg(long, help = "End date (YYYY-MM-DD)")]
        to: Option<String>,
        
        #[arg(long, help = "Project name or ID")]
        project: Option<String>,
    },
    
    #[command(about = "Delete a calendar event")]
    Delete {
        #[arg(help = "Event ID")]
        id: i64,
    },
}

#[derive(Subcommand)]
pub enum IssueAction {
    #[command(about = "Sync issues from external tracker")]
    Sync {
        #[arg(help = "Project name or ID")]
        project: String,
        
        #[arg(long, help = "Issue tracker type (jira, github, gitlab)")]
        tracker_type: Option<String>,
    },
    
    #[command(about = "List issues for a project")]
    List {
        #[arg(help = "Project name or ID")]
        project: String,
        
        #[arg(long, help = "Filter by status")]
        status: Option<String>,
    },
    
    #[command(about = "Link session to issue")]
    Link {
        #[arg(help = "Session ID")]
        session_id: i64,
        
        #[arg(help = "Issue ID (external ID like JIRA-123)")]
        issue_id: String,
    },
}

#[derive(Subcommand)]
pub enum ClientAction {
    #[command(about = "Generate a client report")]
    Generate {
        #[arg(help = "Client name")]
        client: String,
        
        #[arg(help = "Start date (YYYY-MM-DD)")]
        from: String,
        
        #[arg(help = "End date (YYYY-MM-DD)")]
        to: String,
        
        #[arg(long, help = "Project filter (comma-separated)")]
        projects: Option<String>,
        
        #[arg(long, help = "Output format (json, csv, markdown)")]
        format: Option<String>,
    },
    
    #[command(about = "List all client reports")]
    List {
        #[arg(long, help = "Client name filter")]
        client: Option<String>,
    },
    
    #[command(about = "View a specific report")]
    View {
        #[arg(help = "Report ID")]
        id: i64,
    },
}

#[derive(ValueEnum, Clone, Debug)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
}

impl Cli {
    pub fn generate_completions(shell: Shell) {
        use clap_complete::{generate, shells};
        use std::io;

        let mut cmd = Self::command();
        match shell {
            Shell::Bash => generate(shells::Bash, &mut cmd, "tempo", &mut io::stdout()),
            Shell::Zsh => generate(shells::Zsh, &mut cmd, "tempo", &mut io::stdout()),
            Shell::Fish => generate(shells::Fish, &mut cmd, "tempo", &mut io::stdout()),
            Shell::PowerShell => generate(shells::PowerShell, &mut cmd, "tempo", &mut io::stdout()),
        }
    }
}