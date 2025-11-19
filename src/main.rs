mod cli;
mod db;
mod models;
mod ui;
mod utils;

use cli::{commands::handle_command, Cli, Parser};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    env_logger::init();

    // Parse command line arguments
    let cli = Cli::parse();

    // Handle the command
    handle_command(cli).await
}