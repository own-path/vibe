use anyhow::Result;
use rusqlite::{Connection, OpenFlags};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A database connection with metadata
#[derive(Debug)]
pub struct PooledConnection {
    pub connection: Connection,
    created_at: Instant,
    last_used: Instant,
    use_count: usize,
}

impl PooledConnection {
    fn new(connection: Connection) -> Self {
        let now = Instant::now();
        Self {
            connection,
            created_at: now,
            last_used: now,
            use_count: 0,
        }
    }

    fn mark_used(&mut self) {
        self.last_used = Instant::now();
        self.use_count += 1;
    }

    fn is_expired(&self, max_lifetime: Duration) -> bool {
        self.created_at.elapsed() > max_lifetime
    }

    fn is_idle_too_long(&self, max_idle: Duration) -> bool {
        self.last_used.elapsed() > max_idle
    }
}

/// Configuration for the database pool
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_connections: usize,
    pub min_connections: usize,
    pub max_lifetime: Duration,
    pub max_idle_time: Duration,
    pub connection_timeout: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 2,
            max_lifetime: Duration::from_secs(3600), // 1 hour
            max_idle_time: Duration::from_secs(600), // 10 minutes
            connection_timeout: Duration::from_secs(30),
        }
    }
}

/// A connection pool for SQLite databases
pub struct DatabasePool {
    db_path: PathBuf,
    pool: Arc<Mutex<VecDeque<PooledConnection>>>,
    config: PoolConfig,
    stats: Arc<Mutex<PoolStats>>,
}

#[derive(Debug, Default)]
pub struct PoolStats {
    pub total_connections_created: usize,
    pub active_connections: usize,
    pub connections_in_pool: usize,
    pub connection_requests: usize,
    pub connection_timeouts: usize,
}

impl DatabasePool {
    /// Create a new database pool
    pub fn new<P: AsRef<Path>>(db_path: P, config: PoolConfig) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();

        // Create parent directory if it doesn't exist
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let pool = Self {
            db_path,
            pool: Arc::new(Mutex::new(VecDeque::new())),
            config,
            stats: Arc::new(Mutex::new(PoolStats::default())),
        };

        // Pre-populate with minimum connections
        pool.ensure_min_connections()?;

        Ok(pool)
    }

    /// Create a new database pool with default configuration
    pub fn new_with_defaults<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        Self::new(db_path, PoolConfig::default())
    }

    /// Get a connection from the pool
    pub async fn get_connection(&self) -> Result<PooledConnectionGuard> {
        let start = Instant::now();

        // Update stats
        {
            let mut stats = self
                .stats
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to acquire stats lock: {}", e))?;
            stats.connection_requests += 1;
        }

        loop {
            // Try to get a connection from the pool
            if let Some(mut conn) = self.try_get_from_pool()? {
                conn.mark_used();

                // Update stats
                {
                    let mut stats = self
                        .stats
                        .lock()
                        .map_err(|e| anyhow::anyhow!("Failed to acquire stats lock: {}", e))?;
                    stats.active_connections += 1;
                    stats.connections_in_pool = stats.connections_in_pool.saturating_sub(1);
                }

                return Ok(PooledConnectionGuard::new(
                    conn,
                    self.pool.clone(),
                    self.stats.clone(),
                ));
            }

            // If no connection available, try to create a new one
            if self.can_create_new_connection()? {
                let conn = self.create_connection()?;

                // Update stats
                {
                    let mut stats = self
                        .stats
                        .lock()
                        .map_err(|e| anyhow::anyhow!("Failed to acquire stats lock: {}", e))?;
                    stats.total_connections_created += 1;
                    stats.active_connections += 1;
                }

                return Ok(PooledConnectionGuard::new(
                    conn,
                    self.pool.clone(),
                    self.stats.clone(),
                ));
            }

            // Check for timeout
            if start.elapsed() > self.config.connection_timeout {
                let mut stats = self
                    .stats
                    .lock()
                    .map_err(|e| anyhow::anyhow!("Failed to acquire stats lock: {}", e))?;
                stats.connection_timeouts += 1;
                return Err(anyhow::anyhow!(
                    "Connection timeout after {:?}",
                    self.config.connection_timeout
                ));
            }

            // Wait a bit before retrying
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }

    /// Try to get a connection from the existing pool
    fn try_get_from_pool(&self) -> Result<Option<PooledConnection>> {
        let mut pool = self
            .pool
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire pool lock: {}", e))?;

        // Clean up expired/idle connections first
        self.cleanup_connections(&mut pool)?;

        // Try to get a connection
        Ok(pool.pop_front())
    }

    /// Check if we can create a new connection
    fn can_create_new_connection(&self) -> Result<bool> {
        let stats = self
            .stats
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire stats lock: {}", e))?;
        Ok(stats.active_connections + stats.connections_in_pool < self.config.max_connections)
    }

    /// Create a new database connection
    fn create_connection(&self) -> Result<PooledConnection> {
        let connection = Connection::open_with_flags(
            &self.db_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;

        // Configure the connection
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
        connection.pragma_update(None, "cache_size", "-64000")?;

        // Run migrations
        crate::db::migrations::run_migrations(&connection)?;

        Ok(PooledConnection::new(connection))
    }

    /// Clean up expired and idle connections
    fn cleanup_connections(&self, pool: &mut VecDeque<PooledConnection>) -> Result<()> {
        let mut to_remove = Vec::new();

        for (index, conn) in pool.iter().enumerate() {
            if conn.is_expired(self.config.max_lifetime)
                || conn.is_idle_too_long(self.config.max_idle_time)
            {
                to_remove.push(index);
            }
        }

        // Remove connections in reverse order to maintain indices
        for index in to_remove.iter().rev() {
            pool.remove(*index);
        }

        Ok(())
    }

    /// Ensure minimum number of connections are available
    fn ensure_min_connections(&self) -> Result<()> {
        let mut pool = self
            .pool
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire pool lock: {}", e))?;

        while pool.len() < self.config.min_connections {
            let conn = self.create_connection()?;
            pool.push_back(conn);

            // Update stats
            let mut stats = self
                .stats
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to acquire stats lock: {}", e))?;
            stats.total_connections_created += 1;
            stats.connections_in_pool += 1;
        }

        Ok(())
    }

    /// Return a connection to the pool
    fn return_connection(&self, conn: PooledConnection) -> Result<()> {
        let mut pool = self
            .pool
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire pool lock: {}", e))?;

        // Check if we should keep this connection
        if !conn.is_expired(self.config.max_lifetime) && pool.len() < self.config.max_connections {
            pool.push_back(conn);

            // Update stats
            let mut stats = self
                .stats
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to acquire stats lock: {}", e))?;
            stats.connections_in_pool += 1;
            stats.active_connections = stats.active_connections.saturating_sub(1);
        } else {
            // Update stats - connection is being dropped
            let mut stats = self
                .stats
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to acquire stats lock: {}", e))?;
            stats.active_connections = stats.active_connections.saturating_sub(1);
        }

        Ok(())
    }

    /// Get current pool statistics
    pub fn stats(&self) -> Result<PoolStats> {
        let stats = self
            .stats
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire stats lock: {}", e))?;
        Ok(PoolStats {
            total_connections_created: stats.total_connections_created,
            active_connections: stats.active_connections,
            connections_in_pool: stats.connections_in_pool,
            connection_requests: stats.connection_requests,
            connection_timeouts: stats.connection_timeouts,
        })
    }

    /// Close all connections in the pool
    pub fn close(&self) -> Result<()> {
        let mut pool = self
            .pool
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire pool lock: {}", e))?;
        pool.clear();

        let mut stats = self
            .stats
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire stats lock: {}", e))?;
        stats.connections_in_pool = 0;

        Ok(())
    }
}

/// A guard that automatically returns connections to the pool when dropped
pub struct PooledConnectionGuard {
    connection: Option<PooledConnection>,
    pool: Arc<Mutex<VecDeque<PooledConnection>>>,
    stats: Arc<Mutex<PoolStats>>,
}

impl PooledConnectionGuard {
    fn new(
        connection: PooledConnection,
        pool: Arc<Mutex<VecDeque<PooledConnection>>>,
        stats: Arc<Mutex<PoolStats>>,
    ) -> Self {
        Self {
            connection: Some(connection),
            pool,
            stats,
        }
    }

    /// Get a reference to the underlying connection
    pub fn connection(&self) -> &Connection {
        &self.connection.as_ref().unwrap().connection
    }
}

impl Drop for PooledConnectionGuard {
    fn drop(&mut self) {
        if let Some(conn) = self.connection.take() {
            // Try to return connection to pool
            let mut pool = match self.pool.lock() {
                Ok(pool) => pool,
                Err(_) => {
                    // Pool lock is poisoned, just update stats
                    if let Ok(mut stats) = self.stats.lock() {
                        stats.active_connections = stats.active_connections.saturating_sub(1);
                    }
                    return;
                }
            };

            // Check if we should keep this connection
            if !conn.is_expired(Duration::from_secs(3600)) && pool.len() < 10 {
                pool.push_back(conn);
                if let Ok(mut stats) = self.stats.lock() {
                    stats.connections_in_pool += 1;
                    stats.active_connections = stats.active_connections.saturating_sub(1);
                }
            } else {
                // Connection is being dropped
                if let Ok(mut stats) = self.stats.lock() {
                    stats.active_connections = stats.active_connections.saturating_sub(1);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_pool_creation() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let pool = DatabasePool::new_with_defaults(&db_path).unwrap();
        let stats = pool.stats().unwrap();

        // Should have minimum connections created
        assert!(stats.total_connections_created >= 2);
        assert_eq!(stats.connections_in_pool, 2);
    }

    #[tokio::test]
    async fn test_get_connection() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let pool = DatabasePool::new_with_defaults(&db_path).unwrap();
        let conn = pool.get_connection().await.unwrap();

        // Should be able to use the connection
        conn.connection()
            .execute("CREATE TABLE test (id INTEGER)", [])
            .unwrap();

        let stats = pool.stats().unwrap();
        assert_eq!(stats.active_connections, 1);
    }

    #[tokio::test]
    async fn test_connection_return() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let pool = DatabasePool::new_with_defaults(&db_path).unwrap();

        {
            let _conn = pool.get_connection().await.unwrap();
            let stats = pool.stats().unwrap();
            assert_eq!(stats.active_connections, 1);
        }

        // Connection should be returned to pool
        let stats = pool.stats().unwrap();
        assert_eq!(stats.active_connections, 0);
        assert!(stats.connections_in_pool > 0);
    }
}
