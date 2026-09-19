//! Database migration system

use crate::db::{DatabaseConnection, SynchronousMode};
use crate::error::{StorageError, StorageResult};
use rusqlite::params;
use std::path::Path;

/// Current database schema version
pub const CURRENT_SCHEMA_VERSION: i32 = 1;

/// Migration function type
pub type MigrationFunction = Box<dyn Fn(&DatabaseConnection) -> StorageResult<()> + Send + Sync>;

/// Database migration
#[derive(Debug, Clone)]
pub struct Migration {
    pub version: i32,
    pub description: String,
    pub up: MigrationFunction,
    pub down: Option<MigrationFunction>,
}

impl Migration {
    pub fn new(
        version: i32,
        description: impl Into<String>,
        up: MigrationFunction,
    ) -> Self {
        Migration {
            version,
            description: description.into(),
            up,
            down: None,
        }
    }

    pub fn with_down(mut self, down: MigrationFunction) -> Self {
        self.down = Some(down);
        self
    }
}

/// Migration manager
pub struct MigrationManager {
    migrations: Vec<Migration>,
}

impl MigrationManager {
    pub fn new() -> Self {
        MigrationManager {
            migrations: Vec::new(),
        }
    }

    /// Add a migration
    pub fn add_migration(&mut self, migration: Migration) {
        self.migrations.push(migration);
    }

    /// Sort migrations by version
    pub fn sort_migrations(&mut self) {
        self.migrations.sort_by_key(|m| m.version);
    }

    /// Get migrations to apply (migrations with version > current_version)
    pub fn get_migrations_to_apply(&self, current_version: i32) -> Vec<&Migration> {
        self.migrations
            .iter()
            .filter(|m| m.version > current_version)
            .collect()
    }

    /// Get migrations to rollback (migrations with version <= target_version)
    pub fn get_migrations_to_rollback(&self, target_version: i32) -> Vec<&Migration> {
        self.migrations
            .iter()
            .filter(|m| m.version > target_version && m.version <= CURRENT_SCHEMA_VERSION)
            .collect()
    }

    /// Apply all pending migrations
    pub fn apply_migrations(&self, conn: &DatabaseConnection) -> StorageResult<Vec<i32>> {
        let current_version = self.get_current_version(conn)?;
        let migrations_to_apply = self.get_migrations_to_apply(current_version);
        
        let mut applied = Vec::new();

        for migration in migrations_to_apply {
            log::info!("Applying migration {}: {}", migration.version, migration.description);
            
            let tx = conn.transaction()?;
            
            match (migration.up)(conn) {
                Ok(_) => {
                    // Record the migration
                    conn.execute_params(
                        "INSERT INTO schema_version (version, description, applied_at) VALUES (?1, ?2, ?3)",
                        params![migration.version, &migration.description, chrono::Utc::now().to_rfc3339()],
                    )?;
                    tx.commit()?;
                    applied.push(migration.version);
                    log::info!("Migration {} applied successfully", migration.version);
                }
                Err(e) => {
                    tx.rollback()?;
                    log::error!("Migration {} failed: {}", migration.version, e);
                    return Err(StorageError::migration_failed(format!(
                        "Migration {} failed: {}",
                        migration.version, e
                    )));
                }
            }
        }

        Ok(applied)
    }

    /// Rollback migrations to a specific version
    pub fn rollback_to(&self, conn: &DatabaseConnection, target_version: i32) -> StorageResult<Vec<i32>> {
        let current_version = self.get_current_version(conn)?;
        let migrations_to_rollback = self.get_migrations_to_rollback(target_version);
        
        let mut rolled_back = Vec::new();

        // Rollback in reverse order
        let mut sorted_migrations: Vec<&Migration> = migrations_to_rollback;
        sorted_migrations.sort_by(|a, b| b.version.cmp(&a.version));

        for migration in sorted_migrations {
            log::info!("Rolling back migration {}: {}", migration.version, migration.description);
            
            if let Some(down) = &migration.down {
                let tx = conn.transaction()?;
                
                match down(conn) {
                    Ok(_) => {
                        // Remove the migration record
                        conn.execute_params(
                            "DELETE FROM schema_version WHERE version = ?1",
                            params![migration.version],
                        )?;
                        tx.commit()?;
                        rolled_back.push(migration.version);
                        log::info!("Migration {} rolled back successfully", migration.version);
                    }
                    Err(e) => {
                        tx.rollback()?;
                        log::error!("Migration {} rollback failed: {}", migration.version, e);
                        return Err(StorageError::migration_failed(format!(
                            "Migration {} rollback failed: {}",
                            migration.version, e
                        )));
                    }
                }
            } else {
                log::warn!("Migration {} has no down function, cannot rollback", migration.version);
            }
        }

        Ok(rolled_back)
    }

    /// Get current schema version
    pub fn get_current_version(&self, conn: &DatabaseConnection) -> StorageResult<i32> {
        match conn.version() {
            Ok(v) => Ok(v),
            Err(_) => Ok(0), // If schema_version table doesn't exist, version is 0
        }
    }

    /// Initialize the schema version table
    pub fn init_schema_version_table(&self, conn: &DatabaseConnection) -> StorageResult<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                description TEXT NOT NULL,
                applied_at TEXT NOT NULL
            )",
        )?;
        Ok(())
    }

    /// Check if database is up to date
    pub fn is_up_to_date(&self, conn: &DatabaseConnection) -> StorageResult<bool> {
        let current_version = self.get_current_version(conn)?;
        let migrations_to_apply = self.get_migrations_to_apply(current_version);
        Ok(migrations_to_apply.is_empty())
    }

    /// Get all migrations
    pub fn all_migrations(&self) -> &[Migration] {
        &self.migrations
    }
}

impl Default for MigrationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Create the schema version table migration
pub fn create_schema_version_migration() -> Migration {
    Migration::new(
        0,
        "Create schema_version table",
        Box::new(|conn| {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS schema_version (
                    version INTEGER PRIMARY KEY,
                    description TEXT NOT NULL,
                    applied_at TEXT NOT NULL
                )",
            )?;
            Ok(())
        }),
    )
}

/// Initialize the database with all required tables
pub fn initialize_database(conn: &DatabaseConnection) -> StorageResult<()> {
    // Enable WAL mode and foreign keys
    conn.execute("PRAGMA journal_mode=WAL")?;
    conn.execute("PRAGMA foreign_keys=ON")?;
    conn.execute("PRAGMA synchronous=NORMAL")?;

    // Create schema version table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            description TEXT NOT NULL,
            applied_at TEXT NOT NULL
        )",
    )?;

    Ok(())
}

/// Define all database tables as migrations
pub fn create_all_migrations() -> MigrationManager {
    let mut manager = MigrationManager::new();

    // Migration 0: Schema version table (already created by initialize_database)
    manager.add_migration(create_schema_version_migration());

    // Migration 1: Core tables
    manager.add_migration(Migration::new(
        1,
        "Create core tables",
        Box::new(|conn| {
            // Projects table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS projects (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    description TEXT,
                    owner TEXT NOT NULL,
                    state TEXT NOT NULL DEFAULT 'Active',
                    metadata TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    accessed_at TEXT
                )",
            )?;

            // Actors table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS actors (
                    id TEXT PRIMARY KEY,
                    actor_type TEXT NOT NULL,
                    name TEXT NOT NULL,
                    description TEXT,
                    trust_level TEXT NOT NULL DEFAULT 'Neutral',
                    metadata TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                )",
            )?;

            // Knowledge table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS knowledge (
                    id TEXT PRIMARY KEY,
                    knowledge_type TEXT NOT NULL,
                    content TEXT NOT NULL,
                    context TEXT,
                    state TEXT NOT NULL DEFAULT 'Unknown',
                    confidence TEXT NOT NULL DEFAULT 'Medium',
                    source TEXT,
                    provenance TEXT,
                    relations TEXT,
                    evidence TEXT,
                    tags TEXT,
                    metadata TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    version INTEGER NOT NULL DEFAULT 1,
                    content_hash TEXT NOT NULL
                )",
            )?;

            // Knowledge patterns table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS knowledge_patterns (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    description TEXT,
                    pattern_type TEXT NOT NULL,
                    pattern TEXT NOT NULL,
                    examples TEXT,
                    confidence TEXT NOT NULL DEFAULT 'Medium',
                    provenance TEXT,
                    evidence TEXT,
                    tags TEXT,
                    metadata TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    version INTEGER NOT NULL DEFAULT 1
                )",
            )?;

            // Memory items table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS memory_items (
                    id TEXT PRIMARY KEY,
                    memory_type TEXT NOT NULL,
                    content TEXT NOT NULL,
                    context TEXT,
                    confidence TEXT NOT NULL DEFAULT 'Medium',
                    provenance TEXT,
                    relations TEXT,
                    evidence TEXT,
                    tags TEXT,
                    metadata TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    accessed_at TEXT,
                    version INTEGER NOT NULL DEFAULT 1,
                    content_hash TEXT NOT NULL
                )",
            )?;

            // Events table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS events (
                    id TEXT PRIMARY KEY,
                    event_type TEXT NOT NULL,
                    timestamp TEXT NOT NULL,
                    actor TEXT NOT NULL,
                    actor_type TEXT NOT NULL,
                    project_id TEXT,
                    operation_id TEXT,
                    input_hash TEXT,
                    output_hash TEXT,
                    evidence TEXT,
                    confidence TEXT NOT NULL DEFAULT 'Medium',
                    provenance TEXT,
                    parent_event_id TEXT,
                    payload TEXT NOT NULL,
                    metadata TEXT,
                    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE SET NULL,
                    FOREIGN KEY (parent_event_id) REFERENCES events(id) ON DELETE SET NULL
                )",
            )?;

            // Jobs table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS job_instances (
                    id TEXT PRIMARY KEY,
                    job_type TEXT NOT NULL,
                    state TEXT NOT NULL DEFAULT 'Pending',
                    actor TEXT NOT NULL,
                    project_id TEXT,
                    description TEXT,
                    input TEXT,
                    output TEXT,
                    errors TEXT,
                    progress INTEGER NOT NULL DEFAULT 0,
                    checkpoint_id TEXT,
                    metadata TEXT,
                    created_at TEXT NOT NULL,
                    started_at TEXT,
                    completed_at TEXT,
                    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE SET NULL,
                    FOREIGN KEY (checkpoint_id) REFERENCES checkpoints(id) ON DELETE SET NULL
                )",
            )?;

            // Checkpoints table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS checkpoints (
                    id TEXT PRIMARY KEY,
                    job_id TEXT NOT NULL,
                    metadata TEXT,
                    created_at TEXT NOT NULL,
                    is_valid BOOLEAN NOT NULL DEFAULT 1,
                    FOREIGN KEY (job_id) REFERENCES job_instances(id) ON DELETE CASCADE
                )",
            )?;

            // Brain checkpoints table (for learning system)
            conn.execute(
                "CREATE TABLE IF NOT EXISTS brain_checkpoints (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    description TEXT,
                    metadata TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    is_valid BOOLEAN NOT NULL DEFAULT 1
                )",
            )?;

            // Audit log table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS audit_log (
                    id TEXT PRIMARY KEY,
                    event_id TEXT,
                    timestamp TEXT NOT NULL,
                    actor TEXT NOT NULL,
                    actor_type TEXT NOT NULL,
                    operation TEXT NOT NULL,
                    input_hash TEXT,
                    output_hash TEXT,
                    evidence TEXT,
                    confidence TEXT NOT NULL DEFAULT 'Medium',
                    errors TEXT,
                    related_events TEXT,
                    project_id TEXT,
                    metadata TEXT,
                    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE SET NULL,
                    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE SET NULL
                )",
            )?;

            // Evidence table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS evidence (
                    id TEXT PRIMARY KEY,
                    evidence_type TEXT NOT NULL,
                    data TEXT NOT NULL,
                    confidence TEXT NOT NULL DEFAULT 'Medium',
                    provenance TEXT NOT NULL,
                    collected_at TEXT NOT NULL,
                    description TEXT
                )",
            )?;

            // Plugins table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS plugins (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    description TEXT,
                    path TEXT NOT NULL,
                    version TEXT,
                    enabled BOOLEAN NOT NULL DEFAULT 1,
                    loaded_at TEXT,
                    metadata TEXT
                )",
            )?;

            // Configuration table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS configuration (
                    key TEXT PRIMARY KEY,
                    value TEXT NOT NULL,
                    description TEXT,
                    updated_at TEXT NOT NULL
                )",
            )?;

            // Create indexes for performance
            conn.execute("CREATE INDEX IF NOT EXISTS idx_knowledge_type ON knowledge(knowledge_type)")?;
            conn.execute("CREATE INDEX IF NOT EXISTS idx_knowledge_state ON knowledge(state)")?;
            conn.execute("CREATE INDEX IF NOT EXISTS idx_knowledge_content_hash ON knowledge(content_hash)")?;
            conn.execute("CREATE INDEX IF NOT EXISTS idx_memory_type ON memory_items(memory_type)")?;
            conn.execute("CREATE INDEX IF NOT EXISTS idx_memory_content_hash ON memory_items(content_hash)")?;
            conn.execute("CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type)")?;
            conn.execute("CREATE INDEX IF NOT EXISTS idx_events_actor ON events(actor)")?;
            conn.execute("CREATE INDEX IF NOT EXISTS idx_events_project ON events(project_id)")?;
            conn.execute("CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp)")?;
            conn.execute("CREATE INDEX IF NOT EXISTS idx_jobs_state ON job_instances(state)")?;
            conn.execute("CREATE INDEX IF NOT EXISTS idx_jobs_project ON job_instances(project_id)")?;
            conn.execute("CREATE INDEX IF NOT EXISTS idx_checkpoints_job ON checkpoints(job_id)")?;
            conn.execute("CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp)")?;
            conn.execute("CREATE INDEX IF NOT EXISTS idx_audit_actor ON audit_log(actor)")?;
            conn.execute("CREATE INDEX IF NOT EXISTS idx_audit_project ON audit_log(project_id)")?;

            Ok(())
        }),
    ));

    // Migration 2: Add triggers if needed
    // (SQLite doesn't support triggers in the same way as other databases,
    // but we can add application-level logic)

    manager.sort_migrations();
    manager
}

/// Apply all migrations to a database
pub fn apply_all_migrations(conn: &mut DatabaseConnection) -> StorageResult<Vec<i32>> {
    let manager = create_all_migrations();
    
    // Initialize database
    initialize_database(conn)?;
    
    // Apply migrations
    manager.apply_migrations(conn)
}

/// Check if migrations are needed
pub fn migrations_needed(conn: &DatabaseConnection) -> StorageResult<bool> {
    let manager = create_all_migrations();
    manager.is_up_to_date(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DatabaseConfig;
    use tempfile::NamedTempFile;

    #[test]
    fn test_migration_manager() {
        let mut manager = MigrationManager::new();
        
        manager.add_migration(Migration::new(
            1,
            "Test migration",
            Box::new(|_conn| Ok(())),
        ));
        manager.add_migration(Migration::new(
            3,
            "Another test",
            Box::new(|_conn| Ok(())),
        ));
        manager.add_migration(Migration::new(
            2,
            "Middle migration",
            Box::new(|_conn| Ok(())),
        ));

        manager.sort_migrations();
        
        assert_eq!(manager.all_migrations().len(), 3);
        assert_eq!(manager.all_migrations()[0].version, 1);
        assert_eq!(manager.all_migrations()[1].version, 2);
        assert_eq!(manager.all_migrations()[2].version, 3);
    }

    #[test]
    fn test_apply_migrations() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        let config = DatabaseConfig::new(path);
        let mut conn = DatabaseConnection::open(&config).unwrap();
        
        // Initialize database
        initialize_database(&conn).unwrap();
        
        // Apply migrations
        let manager = create_all_migrations();
        let applied = manager.apply_migrations(&conn).unwrap();
        
        assert!(!applied.is_empty());
        
        // Check that tables were created
        assert!(conn.table_exists("projects").unwrap());
        assert!(conn.table_exists("knowledge").unwrap());
        assert!(conn.table_exists("events").unwrap());
        assert!(conn.table_exists("job_instances").unwrap());
    }

    #[test]
    fn test_migrations_needed() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        let config = DatabaseConfig::new(path);
        let mut conn = DatabaseConnection::open(&config).unwrap();
        
        // Before migrations, they should be needed
        assert!(migrations_needed(&conn).unwrap());
        
        // Apply migrations
        apply_all_migrations(&mut conn).unwrap();
        
        // Now they should not be needed
        assert!(!migrations_needed(&conn).unwrap());
    }

    #[test]
    fn test_create_all_migrations() {
        let manager = create_all_migrations();
        assert!(!manager.all_migrations().is_empty());
        
        // Check that we have migrations for all major tables
        let versions: Vec<i32> = manager.all_migrations().iter().map(|m| m.version).collect();
        assert!(versions.contains(&0));
        assert!(versions.contains(&1));
    }
}
