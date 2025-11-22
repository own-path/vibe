pub mod advanced_queries;
pub mod connection;
pub mod migrations;
pub mod pool;
pub mod queries;

use anyhow::Result;
use std::path::Path;
use std::sync::OnceLock;

pub use connection::Database;
pub use pool::{DatabasePool, PoolConfig, PooledConnectionGuard};

/// Global database pool instance
static DB_POOL: OnceLock<DatabasePool> = OnceLock::new();

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
        .or_else(dirs::home_dir)
        .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?;

    let tempo_dir = data_dir.join(".tempo");
    std::fs::create_dir_all(&tempo_dir)?;

    Ok(tempo_dir.join("data.db"))
}

/// Initialize the global database pool
pub fn initialize_pool() -> Result<()> {
    let db_path = get_database_path()?;
    let pool = DatabasePool::new_with_defaults(db_path)?;

    DB_POOL
        .set(pool)
        .map_err(|_| anyhow::anyhow!("Database pool already initialized"))?;

    Ok(())
}

/// Initialize the global database pool with custom configuration
pub fn initialize_pool_with_config(config: PoolConfig) -> Result<()> {
    let db_path = get_database_path()?;
    let pool = DatabasePool::new(db_path, config)?;

    DB_POOL
        .set(pool)
        .map_err(|_| anyhow::anyhow!("Database pool already initialized"))?;

    Ok(())
}

/// Get a connection from the global pool
pub async fn get_connection() -> Result<PooledConnectionGuard> {
    let pool = DB_POOL.get().ok_or_else(|| {
        anyhow::anyhow!("Database pool not initialized. Call initialize_pool() first.")
    })?;

    pool.get_connection().await
}

/// Get pool statistics
pub fn get_pool_stats() -> Result<pool::PoolStats> {
    let pool = DB_POOL
        .get()
        .ok_or_else(|| anyhow::anyhow!("Database pool not initialized"))?;

    pool.stats()
}

/// Close the global pool
pub fn close_pool() -> Result<()> {
    if let Some(pool) = DB_POOL.get() {
        pool.close()?;
    }
    Ok(())
}
