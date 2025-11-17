use clap::{Parser, Subcommand, ValueEnum, CommandFactory};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "vibe")]
#[command(about = "Automatic project time tracking CLI tool")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(author = "Vibe Contributors")]
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
            Shell::Bash => generate(shells::Bash, &mut cmd, "vibe", &mut io::stdout()),
            Shell::Zsh => generate(shells::Zsh, &mut cmd, "vibe", &mut io::stdout()),
            Shell::Fish => generate(shells::Fish, &mut cmd, "vibe", &mut io::stdout()),
            Shell::PowerShell => generate(shells::PowerShell, &mut cmd, "vibe", &mut io::stdout()),
        }
    }
}