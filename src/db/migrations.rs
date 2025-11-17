use anyhow::Result;
use rusqlite::Connection;
use std::collections::HashMap;

const MIGRATION_001: &str = include_str!("../../migrations/001_minimal_schema.sql");

pub fn run_migrations(conn: &Connection) -> Result<()> {
    let current_version = get_current_version(conn)?;
    let migrations = get_migrations();
    
    for (version, sql) in migrations.iter() {
        if *version > current_version {
            log::info!("Running migration {}", version);
            
            // Run migration in a transaction
            let tx = conn.unchecked_transaction()?;
            
            // Execute the SQL as a batch
            log::debug!("Executing migration SQL: {}", sql);
            tx.execute_batch(sql)?;
            
            // Update schema version
            tx.execute(
                "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
                [version]
            )?;
            
            tx.commit()?;
            log::info!("Migration {} completed", version);
        }
    }
    
    Ok(())
}

fn get_current_version(conn: &Connection) -> Result<i32> {
    // Check if schema_version table exists
    let table_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_version'",
        [],
        |row| row.get::<_, i32>(0)
    )? > 0;
    
    if !table_exists {
        return Ok(0);
    }
    
    // Get the current version
    let version = conn.query_row(
        "SELECT MAX(version) FROM schema_version",
        [],
        |row| {
            let version: Option<i32> = row.get(0)?;
            Ok(version.unwrap_or(0))
        }
    )?;
    
    Ok(version)
}

fn get_migrations() -> HashMap<i32, String> {
    let mut migrations = HashMap::new();
    migrations.insert(1, MIGRATION_001.to_string());
    migrations
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_migrations() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        
        // Verify tables exist
        let tables: Vec<String> = conn.prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap()
            .query_map([], |row| Ok(row.get::<_, String>(0)?))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        
        assert!(tables.contains(&"projects".to_string()));
        assert!(tables.contains(&"sessions".to_string()));
        assert!(tables.contains(&"tags".to_string()));
        assert!(tables.contains(&"schema_version".to_string()));
    }
}