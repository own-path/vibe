pub mod connection;
pub mod migrations;
pub mod queries;
pub mod advanced_queries;

use anyhow::Result;
use std::path::Path;

pub use connection::Database;

pub fn initialize_database(db_path: &Path) -> Result<Database> {
    println!("Initializing database at: {:?}", db_path);
    let db = Database::new(db_path)?;
    println!("Database connection created");
    
    // Run migrations
    println!("Running migrations...");
    migrations::run_migrations(&db.connection)?;
    println!("Migrations completed");
    
    Ok(db)
}

pub fn get_database_path() -> Result<std::path::PathBuf> {
    let data_dir = dirs::data_dir()
        .or_else(|| dirs::home_dir())
        .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;
    
    let vibe_dir = data_dir.join(".vibe");
    std::fs::create_dir_all(&vibe_dir)?;
    
    Ok(vibe_dir.join("data.db"))
}