//! Database connection and configuration

use crate::error::{StorageError, StorageResult};
use rusqlite::{Connection, OpenFlags, OptionalTransaction};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub path: PathBuf,
    pub wal_mode: bool,
    pub synchronous: SynchronousMode,
    pub journal_mode: JournalMode,
    pub temp_store: TempStoreMode,
    pub cache_size: i32,
    pub foreign_keys: bool,
    pub max_connections: usize,
}

impl DatabaseConfig {
    pub fn new(path: impl AsRef<Path>) -> Self {
        DatabaseConfig {
            path: path.as_ref().to_path_buf(),
            wal_mode: true,
            synchronous: SynchronousMode::Normal,
            journal_mode: JournalMode::Wal,
            temp_store: TempStoreMode::Memory,
            cache_size: -2000, // 2MB negative cache size
            foreign_keys: true,
            max_connections: 10,
        }
    }

    pub fn with_wal_mode(mut self, enabled: bool) -> Self {
        self.wal_mode = enabled;
        self
    }

    pub fn with_synchronous(mut self, mode: SynchronousMode) -> Self {
        self.synchronous = mode;
        self
    }

    pub fn with_journal_mode(mut self, mode: JournalMode) -> Self {
        self.journal_mode = mode;
        self
    }

    pub fn with_cache_size(mut self, size_kb: i32) -> Self {
        self.cache_size = size_kb;
        self
    }

    pub fn with_foreign_keys(mut self, enabled: bool) -> Self {
        self.foreign_keys = enabled;
        self
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        let path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from(".malverde"))
            .join("malverde.db");
        DatabaseConfig::new(path)
    }
}

/// SQLite synchronous mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynchronousMode {
    Off,
    Normal,
    Full,
    Extra,
}

impl std::fmt::Display for SynchronousMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SynchronousMode::Off => write!(f, "OFF"),
            SynchronousMode::Normal => write!(f, "NORMAL"),
            SynchronousMode::Full => write!(f, "FULL"),
            SynchronousMode::Extra => write!(f, "EXTRA"),
        }
    }
}

/// SQLite journal mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalMode {
    Delete,
    Truncate,
    Persist,
    Memory,
    Wal,
    Off,
}

impl std::fmt::Display for JournalMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JournalMode::Delete => write!(f, "DELETE"),
            JournalMode::Truncate => write!(f, "TRUNCATE"),
            JournalMode::Persist => write!(f, "PERSIST"),
            JournalMode::Memory => write!(f, "MEMORY"),
            JournalMode::Wal => write!(f, "WAL"),
            JournalMode::Off => write!(f, "OFF"),
        }
    }
}

/// SQLite temp store mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempStoreMode {
    Default,
    File,
    Memory,
}

impl std::fmt::Display for TempStoreMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TempStoreMode::Default => write!(f, "DEFAULT"),
            TempStoreMode::File => write!(f, "FILE"),
            TempStoreMode::Memory => write!(f, "MEMORY"),
        }
    }
}

/// Database connection wrapper
pub struct DatabaseConnection {
    conn: Connection,
    config: DatabaseConfig,
}

impl DatabaseConnection {
    /// Open a new database connection
    pub fn open(config: &DatabaseConfig) -> StorageResult<Self> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = config.path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open connection with appropriate flags
        let flags = if config.wal_mode {
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_URI
        } else {
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
        };

        let mut conn = Connection::open_with_flags(&config.path, flags)?;

        // Configure the connection
        conn.pragma_update(None, "synchronous", &config.synchronous.to_string())?;
        conn.pragma_update(None, "journal_mode", &config.journal_mode.to_string())?;
        conn.pragma_update(None, "temp_store", &config.temp_store.to_string())?;
        conn.pragma_update(None, "cache_size", &config.cache_size.to_string())?;
        
        if config.foreign_keys {
            conn.pragma_update(None, "foreign_keys", "ON")?;
        }

        // Enable WAL mode if requested
        if config.wal_mode {
            conn.pragma_update(None, "journal_mode", "WAL")?;
        }

        Ok(DatabaseConnection {
            conn,
            config: config.clone(),
        })
    }

    /// Get the underlying connection
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Get mutable access to the connection
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Get the configuration
    pub fn config(&self) -> &DatabaseConfig {
        &self.config
    }

    /// Execute a query
    pub fn execute(&self, query: &str) -> StorageResult<()> {
        self.conn.execute(query, [])?;
        Ok(())
    }

    /// Execute a query with parameters
    pub fn execute_params(&self, query: &str, params: &[&dyn rusqlite::ToSql]) -> StorageResult<()> {
        self.conn.execute(query, params)?;
        Ok(())
    }

    /// Query and get results
    pub fn query<T, F>(&self, query: &str, f: F) -> StorageResult<T>
    where
        F: FnOnce(&mut rusqlite::Statement) -> StorageResult<T>,
    {
        let mut stmt = self.conn.prepare(query)?;
        f(&mut stmt)
    }

    /// Query with parameters
    pub fn query_params<T, F>(&self, query: &str, params: &[&dyn rusqlite::ToSql], f: F) -> StorageResult<T>
    where
        F: FnOnce(&mut rusqlite::Statement) -> StorageResult<T>,
    {
        let mut stmt = self.conn.prepare(query)?;
        f(&mut stmt)
    }

    /// Start a transaction
    pub fn transaction(&self) -> StorageResult<rusqlite::Transaction> {
        Ok(self.conn.transaction()?)
    }

    /// Check if a table exists
    pub fn table_exists(&self, table_name: &str) -> StorageResult<bool> {
        let result: i32 = self.conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            [table_name],
            |row| row.get(0),
        )?;
        Ok(result > 0)
    }

    /// Get the database version
    pub fn version(&self) -> StorageResult<i32> {
        let result: i32 = self.conn.query_row(
            "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1",
            [],
            |row| row.get(0),
        )?;
        Ok(result)
    }

    /// Check database integrity
    pub fn check_integrity(&self) -> StorageResult<()> {
        self.conn.execute("PRAGMA integrity_check", [])?;
        Ok(())
    }

    /// Vacuum the database
    pub fn vacuum(&self) -> StorageResult<()> {
        self.conn.execute("VACUUM", [])?;
        Ok(())
    }

    /// Close the connection
    pub fn close(self) -> StorageResult<()> {
        Ok(self.conn.close()?)
    }
}

/// Database connection pool
#[derive(Debug, Clone)]
pub struct ConnectionPool {
    config: DatabaseConfig,
    connections: Arc<RwLock<Vec<Option<DatabaseConnection>>>>,
}

impl ConnectionPool {
    pub fn new(config: DatabaseConfig, max_connections: usize) -> StorageResult<Self> {
        let mut connections = Vec::with_capacity(max_connections);
        for _ in 0..max_connections {
            connections.push(None);
        }

        Ok(ConnectionPool {
            config: config.clone(),
            connections: Arc::new(RwLock::new(connections)),
        })
    }

    /// Get a connection from the pool
    pub fn get(&self) -> StorageResult<DatabaseConnection> {
        let mut connections = self.connections.write().unwrap();
        
        // Try to find an available connection
        for conn in connections.iter_mut() {
            if let Some(c) = conn.take() {
                return Ok(c);
            }
        }

        // If none available, create a new one
        DatabaseConnection::open(&self.config)
    }

    /// Return a connection to the pool
    pub fn return_conn(&self, conn: DatabaseConnection) {
        let mut connections = self.connections.write().unwrap();
        for c in connections.iter_mut() {
            if c.is_none() {
                *c = Some(conn);
                return;
            }
        }
        // If pool is full, just drop it (will be closed when dropped)
    }
}

/// Default database path
pub fn default_db_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from(".malverde"))
        .join("malverde.db")
}

/// Initialize the database directory
pub fn init_db_directory(path: &Path) -> StorageResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_database_config() {
        let config = DatabaseConfig::new("/tmp/test.db")
            .with_wal_mode(true)
            .with_synchronous(SynchronousMode::Normal)
            .with_journal_mode(JournalMode::Wal)
            .with_cache_size(-2000);

        assert_eq!(config.path, PathBuf::from("/tmp/test.db"));
        assert!(config.wal_mode);
    }

    #[test]
    fn test_database_connection() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        let config = DatabaseConfig::new(path);
        let conn = DatabaseConnection::open(&config).unwrap();

        assert!(conn.conn().is_autocommit());
        
        // Test table existence
        assert!(!conn.table_exists("nonexistent").unwrap());
        
        // Create a test table
        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)").unwrap();
        assert!(conn.table_exists("test").unwrap());

        conn.close().unwrap();
    }

    #[test]
    fn test_connection_pool() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        let config = DatabaseConfig::new(path);
        let pool = ConnectionPool::new(config, 2).unwrap();

        let conn1 = pool.get().unwrap();
        let conn2 = pool.get().unwrap();

        assert!(pool.get().is_err()); // Pool is exhausted

        pool.return_conn(conn1);
        pool.return_conn(conn2);

        // Now we should be able to get connections again
        assert!(pool.get().is_ok());
    }

    #[test]
    fn test_default_db_path() {
        let path = default_db_path();
        assert!(path.to_string_lossy().contains("malverde.db"));
    }
}
