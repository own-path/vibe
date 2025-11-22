use tempo_cli::{
    cli::{commands::handle_command, Cli, Parser},
    db,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    env_logger::init();

    // Initialize database pool early
    if let Err(e) = db::initialize_pool() {
        eprintln!("Warning: Failed to initialize database pool: {}. Falling back to individual connections.", e);
    }

    // Parse command line arguments
    let cli = Cli::parse();

    // Handle the command
    let result = handle_command(cli).await;

    // Clean up pool on exit
    let _ = db::close_pool();

    result
}
