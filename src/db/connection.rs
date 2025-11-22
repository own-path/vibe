use anyhow::Result;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::path::Path;

pub struct Database {
    pub connection: Connection,
}

impl Database {
    pub fn new(db_path: &Path) -> Result<Self> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let connection = Connection::open_with_flags(
            db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;

        // Enable foreign key constraints
        connection.pragma_update(None, "foreign_keys", "ON")?;

        // Set WAL mode for better concurrent access
        connection.pragma_update(None, "journal_mode", "WAL")?;

        // Set synchronous to NORMAL for better performance
        connection.pragma_update(None, "synchronous", "NORMAL")?;

        // Set cache size (negative value means KB)
        connection.pragma_update(None, "cache_size", "-64000")?;

        let db = Self { connection };

        // Run migrations automatically
        crate::db::migrations::run_migrations(&db.connection)?;

        Ok(db)
    }

    pub fn in_memory() -> Result<Self> {
        let connection = Connection::open_in_memory()?;
        connection.execute("PRAGMA foreign_keys = ON", [])?;
        Ok(Self { connection })
    }

    pub fn backup_to(&self, backup_path: &Path) -> Result<()> {
        let mut backup_conn = Connection::open(backup_path)?;
        let backup = rusqlite::backup::Backup::new(&self.connection, &mut backup_conn)?;
        backup.run_to_completion(5, std::time::Duration::from_millis(250), None)?;
        Ok(())
    }

    pub fn vacuum(&self) -> Result<()> {
        self.connection.execute("VACUUM", [])?;
        Ok(())
    }

    pub fn analyze(&self) -> Result<()> {
        self.connection.execute("ANALYZE", [])?;
        Ok(())
    }

    pub fn get_schema_version(&self) -> Result<Option<i32>> {
        let mut stmt = self
            .connection
            .prepare("SELECT version FROM schema_version ORDER BY version DESC LIMIT 1")?;

        let version = stmt.query_row([], |row| row.get::<_, i32>(0)).optional()?;

        Ok(version)
    }
}
