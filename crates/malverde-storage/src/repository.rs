//! Repository implementations for database entities

use crate::db::DatabaseConnection;
use crate::error::{StorageError, StorageResult};
use malverde_core::ids::{
    ActorId, AuditId, CheckpointId, EvidenceId, JobId, KnowledgeId, MemoryId, OperationId, PatternId,
    PluginId, ProjectId,
};
use malverde_core::models::{Actor, Command, CommandResult, KnowledgeItem, MemoryItem, Operation, Pattern};
use malverde_core::states::{ActorType, JobState, KnowledgeState, MemoryType, OperationState, ProjectState};
use malverde_core::timestamps::{AccessedAt, CreatedAt, UpdatedAt};
use malverde_core::trust::{Confidence, Evidence, Provenance, TrustLevel};
use malverde_events::event::{Event, EventMetadata, EventPayload};
use malverde_events::timeline::Timeline;
use malverde_events::EventType;
use rusqlite::params;
use serde_json;
use std::collections::HashMap;

/// Trait for repository operations
pub trait Repository<T, Id>:
    Send + Sync
where
    T: Send + Sync,
    Id: Send + Sync,
{
    fn save(&self, conn: &DatabaseConnection, entity: &T) -> StorageResult<Id>;
    fn find_by_id(&self, conn: &DatabaseConnection, id: &Id) -> StorageResult<Option<T>>;
    fn find_all(&self, conn: &DatabaseConnection) -> StorageResult<Vec<T>>;
    fn update(&self, conn: &DatabaseConnection, entity: &T) -> StorageResult<bool>;
    fn delete(&self, conn: &DatabaseConnection, id: &Id) -> StorageResult<bool>;
    fn exists(&self, conn: &DatabaseConnection, id: &Id) -> StorageResult<bool>;
}

/// Project Repository
#[derive(Debug, Clone)]
pub struct ProjectRepository;

impl ProjectRepository {
    pub fn new() -> Self {
        ProjectRepository
    }

    pub fn find_by_name(&self, conn: &DatabaseConnection, name: &str) -> StorageResult<Option<Project>> {
        let result: Option<Project> = conn.query_params(
            "SELECT id, name, description, owner, state, metadata, created_at, updated_at, accessed_at FROM projects WHERE name = ?1 LIMIT 1",
            [name],
            |row| {
                let metadata: serde_json::Value = row.get::<_, String>(5).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(serde_json::Value::Null);
                Ok(Project {
                    id: ProjectId::from_string(&row.get::<_, String>(0)?).unwrap(),
                    name: row.get(1)?,
                    description: row.get::<_, Option<String>>(2)?,
                    owner: ActorId::from_string(&row.get::<_, String>(3)?).unwrap(),
                    state: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or(ProjectState::Active),
                    metadata,
                    created_at: CreatedAt::from(chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?).unwrap()),
                    updated_at: UpdatedAt::from(chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?).unwrap()),
                    accessed_at: row.get::<_, Option<String>>(8)?.map(|s| AccessedAt::from(chrono::DateTime::parse_from_rfc3339(&s).unwrap())),
                })
            },
        )?;
        Ok(result)
    }

    pub fn find_by_owner(&self, conn: &DatabaseConnection, owner: &ActorId) -> StorageResult<Vec<Project>> {
        let results: Vec<Project> = conn.query_params(
            "SELECT id, name, description, owner, state, metadata, created_at, updated_at, accessed_at FROM projects WHERE owner = ?1",
            [owner.as_str()],
            |row| {
                let metadata: serde_json::Value = row.get::<_, String>(5).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(serde_json::Value::Null);
                Ok(Project {
                    id: ProjectId::from_string(&row.get::<_, String>(0)?).unwrap(),
                    name: row.get(1)?,
                    description: row.get::<_, Option<String>>(2)?,
                    owner: ActorId::from_string(&row.get::<_, String>(3)?).unwrap(),
                    state: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or(ProjectState::Active),
                    metadata,
                    created_at: CreatedAt::from(chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?).unwrap()),
                    updated_at: UpdatedAt::from(chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?).unwrap()),
                    accessed_at: row.get::<_, Option<String>>(8)?.map(|s| AccessedAt::from(chrono::DateTime::parse_from_rfc3339(&s).unwrap())),
                })
            },
        )?;
        Ok(results)
    }
}

impl Repository<Project, ProjectId> for ProjectRepository {
    fn save(&self, conn: &DatabaseConnection, entity: &Project) -> StorageResult<ProjectId> {
        conn.execute_params(
            "INSERT INTO projects (id, name, description, owner, state, metadata, created_at, updated_at, accessed_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                entity.id.as_str(),
                &entity.name,
                entity.description.as_deref().unwrap_or(""),
                entity.owner.as_str(),
                serde_json::to_string(&entity.state).unwrap(),
                serde_json::to_string(&entity.metadata).unwrap(),
                entity.created_at.to_rfc3339(),
                entity.updated_at.to_rfc3339(),
                entity.accessed_at.as_ref().map(|a| a.to_rfc3339()).unwrap_or(""),
            ],
        )?;
        Ok(entity.id.clone())
    }

    fn find_by_id(&self, conn: &DatabaseConnection, id: &ProjectId) -> StorageResult<Option<Project>> {
        let result: Option<Project> = conn.query_params(
            "SELECT id, name, description, owner, state, metadata, created_at, updated_at, accessed_at FROM projects WHERE id = ?1 LIMIT 1",
            [id.as_str()],
            |row| {
                let metadata: serde_json::Value = row.get::<_, String>(5).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(serde_json::Value::Null);
                Ok(Project {
                    id: ProjectId::from_string(&row.get::<_, String>(0)?).unwrap(),
                    name: row.get(1)?,
                    description: row.get::<_, Option<String>>(2)?,
                    owner: ActorId::from_string(&row.get::<_, String>(3)?).unwrap(),
                    state: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or(ProjectState::Active),
                    metadata,
                    created_at: CreatedAt::from(chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?).unwrap()),
                    updated_at: UpdatedAt::from(chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?).unwrap()),
                    accessed_at: row.get::<_, Option<String>>(8)?.map(|s| AccessedAt::from(chrono::DateTime::parse_from_rfc3339(&s).unwrap())),
                })
            },
        )?;
        Ok(result)
    }

    fn find_all(&self, conn: &DatabaseConnection) -> StorageResult<Vec<Project>> {
        let results: Vec<Project> = conn.query(
            "SELECT id, name, description, owner, state, metadata, created_at, updated_at, accessed_at FROM projects",
            |row| {
                let metadata: serde_json::Value = row.get::<_, String>(5).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(serde_json::Value::Null);
                Ok(Project {
                    id: ProjectId::from_string(&row.get::<_, String>(0)?).unwrap(),
                    name: row.get(1)?,
                    description: row.get::<_, Option<String>>(2)?,
                    owner: ActorId::from_string(&row.get::<_, String>(3)?).unwrap(),
                    state: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or(ProjectState::Active),
                    metadata,
                    created_at: CreatedAt::from(chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?).unwrap()),
                    updated_at: UpdatedAt::from(chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?).unwrap()),
                    accessed_at: row.get::<_, Option<String>>(8)?.map(|s| AccessedAt::from(chrono::DateTime::parse_from_rfc3339(&s).unwrap())),
                })
            },
        )?;
        Ok(results)
    }

    fn update(&self, conn: &DatabaseConnection, entity: &Project) -> StorageResult<bool> {
        let rows_affected = conn.execute_params(
            "UPDATE projects SET name = ?1, description = ?2, state = ?3, metadata = ?4, updated_at = ?5, accessed_at = ?6 WHERE id = ?7",
            params![
                &entity.name,
                entity.description.as_deref().unwrap_or(""),
                serde_json::to_string(&entity.state).unwrap(),
                serde_json::to_string(&entity.metadata).unwrap(),
                entity.updated_at.to_rfc3339(),
                entity.accessed_at.as_ref().map(|a| a.to_rfc3339()).unwrap_or(""),
                entity.id.as_str(),
            ],
        )?;
        Ok(rows_affected > 0)
    }

    fn delete(&self, conn: &DatabaseConnection, id: &ProjectId) -> StorageResult<bool> {
        let rows_affected = conn.execute_params(
            "DELETE FROM projects WHERE id = ?1",
            [id.as_str()],
        )?;
        Ok(rows_affected > 0)
    }

    fn exists(&self, conn: &DatabaseConnection, id: &ProjectId) -> StorageResult<bool> {
        let count: i32 = conn.query_row_params(
            "SELECT COUNT(*) FROM projects WHERE id = ?1",
            [id.as_str()],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}

/// Knowledge Repository
#[derive(Debug, Clone)]
pub struct KnowledgeRepository;

impl KnowledgeRepository {
    pub fn new() -> Self {
        KnowledgeRepository
    }

    pub fn find_by_type(&self, conn: &DatabaseConnection, knowledge_type: &str) -> StorageResult<Vec<KnowledgeItem>> {
        let results: Vec<KnowledgeItem> = conn.query_params(
            "SELECT id, knowledge_type, content, context, state, confidence, source, provenance, relations, evidence, tags, metadata, created_at, updated_at, version, content_hash FROM knowledge WHERE knowledge_type = ?1",
            [knowledge_type],
            |row| {
                self.row_to_knowledge(row)
            },
        )?;
        Ok(results)
    }

    pub fn find_by_state(&self, conn: &DatabaseConnection, state: KnowledgeState) -> StorageResult<Vec<KnowledgeItem>> {
        let state_str = serde_json::to_string(&state).unwrap();
        let results: Vec<KnowledgeItem> = conn.query_params(
            "SELECT id, knowledge_type, content, context, state, confidence, source, provenance, relations, evidence, tags, metadata, created_at, updated_at, version, content_hash FROM knowledge WHERE state = ?1",
            [&state_str],
            |row| {
                self.row_to_knowledge(row)
            },
        )?;
        Ok(results)
    }

    pub fn find_by_content_hash(&self, conn: &DatabaseConnection, hash: &str) -> StorageResult<Option<KnowledgeItem>> {
        let result: Option<KnowledgeItem> = conn.query_params(
            "SELECT id, knowledge_type, content, context, state, confidence, source, provenance, relations, evidence, tags, metadata, created_at, updated_at, version, content_hash FROM knowledge WHERE content_hash = ?1 LIMIT 1",
            [hash],
            |row| {
                self.row_to_knowledge(row)
            },
        )?;
        Ok(result)
    }

    fn row_to_knowledge(&self, row: &rusqlite::Row) -> StorageResult<KnowledgeItem> {
        let relations_str: String = row.get(8)?;
        let relations: Vec<malverde_core::models::KnowledgeRelation> = serde_json::from_str(&relations_str).unwrap_or_default();
        
        let evidence_str: String = row.get(9)?;
        let evidence: Vec<Evidence> = serde_json::from_str(&evidence_str).unwrap_or_default();
        
        let tags_str: String = row.get(10)?;
        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
        
        let metadata: serde_json::Value = row.get::<_, String>(11).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(serde_json::Value::Null);
        
        let provenance_str: Option<String> = row.get(7)?;
        let provenance = provenance_str.and_then(|s| serde_json::from_str(&s).ok());
        
        let source: Option<String> = row.get(6)?;

        Ok(KnowledgeItem {
            id: KnowledgeId::from_string(&row.get::<_, String>(0)?).unwrap(),
            knowledge_type: row.get(1)?,
            content: row.get(2)?,
            context: row.get::<_, Option<String>>(3)?,
            state: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or(KnowledgeState::Unknown),
            confidence: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or(Confidence::Medium),
            source,
            provenance,
            relations,
            evidence,
            tags,
            metadata,
            created_at: CreatedAt::from(chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(12)?).unwrap()),
            updated_at: UpdatedAt::from(chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(13)?).unwrap()),
            version: row.get(14)?,
        })
    }
}

impl Repository<KnowledgeItem, KnowledgeId> for KnowledgeRepository {
    fn save(&self, conn: &DatabaseConnection, entity: &KnowledgeItem) -> StorageResult<KnowledgeId> {
        let relations_json = serde_json::to_string(&entity.relations).unwrap();
        let evidence_json = serde_json::to_string(&entity.evidence).unwrap();
        let tags_json = serde_json::to_string(&entity.tags).unwrap();
        let provenance_json = entity.provenance.as_ref().map(|p| serde_json::to_string(p).unwrap()).unwrap_or_default();
        
        conn.execute_params(
            "INSERT INTO knowledge (id, knowledge_type, content, context, state, confidence, source, provenance, relations, evidence, tags, metadata, created_at, updated_at, version, content_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                entity.id.as_str(),
                &entity.knowledge_type,
                &entity.content,
                entity.context.as_deref().unwrap_or(""),
                serde_json::to_string(&entity.state).unwrap(),
                serde_json::to_string(&entity.confidence).unwrap(),
                entity.source.as_deref().unwrap_or(""),
                &provenance_json,
                &relations_json,
                &evidence_json,
                &tags_json,
                serde_json::to_string(&entity.metadata).unwrap(),
                entity.created_at.to_rfc3339(),
                entity.updated_at.to_rfc3339(),
                entity.version,
                &entity.content_hash(),
            ],
        )?;
        Ok(entity.id.clone())
    }

    fn find_by_id(&self, conn: &DatabaseConnection, id: &KnowledgeId) -> StorageResult<Option<KnowledgeItem>> {
        let result: Option<KnowledgeItem> = conn.query_params(
            "SELECT id, knowledge_type, content, context, state, confidence, source, provenance, relations, evidence, tags, metadata, created_at, updated_at, version, content_hash FROM knowledge WHERE id = ?1 LIMIT 1",
            [id.as_str()],
            |row| {
                self.row_to_knowledge(row)
            },
        )?;
        Ok(result)
    }

    fn find_all(&self, conn: &DatabaseConnection) -> StorageResult<Vec<KnowledgeItem>> {
        let results: Vec<KnowledgeItem> = conn.query(
            "SELECT id, knowledge_type, content, context, state, confidence, source, provenance, relations, evidence, tags, metadata, created_at, updated_at, version, content_hash FROM knowledge",
            |row| {
                self.row_to_knowledge(row)
            },
        )?;
        Ok(results)
    }

    fn update(&self, conn: &DatabaseConnection, entity: &KnowledgeItem) -> StorageResult<bool> {
        let relations_json = serde_json::to_string(&entity.relations).unwrap();
        let evidence_json = serde_json::to_string(&entity.evidence).unwrap();
        let tags_json = serde_json::to_string(&entity.tags).unwrap();
        let provenance_json = entity.provenance.as_ref().map(|p| serde_json::to_string(p).unwrap()).unwrap_or_default();
        
        let rows_affected = conn.execute_params(
            "UPDATE knowledge SET knowledge_type = ?1, content = ?2, context = ?3, state = ?4, confidence = ?5, source = ?6, provenance = ?7, relations = ?8, evidence = ?9, tags = ?10, metadata = ?11, updated_at = ?12, version = ?13, content_hash = ?14 WHERE id = ?15",
            params![
                &entity.knowledge_type,
                &entity.content,
                entity.context.as_deref().unwrap_or(""),
                serde_json::to_string(&entity.state).unwrap(),
                serde_json::to_string(&entity.confidence).unwrap(),
                entity.source.as_deref().unwrap_or(""),
                &provenance_json,
                &relations_json,
                &evidence_json,
                &tags_json,
                serde_json::to_string(&entity.metadata).unwrap(),
                entity.updated_at.to_rfc3339(),
                entity.version,
                &entity.content_hash(),
                entity.id.as_str(),
            ],
        )?;
        Ok(rows_affected > 0)
    }

    fn delete(&self, conn: &DatabaseConnection, id: &KnowledgeId) -> StorageResult<bool> {
        let rows_affected = conn.execute_params(
            "DELETE FROM knowledge WHERE id = ?1",
            [id.as_str()],
        )?;
        Ok(rows_affected > 0)
    }

    fn exists(&self, conn: &DatabaseConnection, id: &KnowledgeId) -> StorageResult<bool> {
        let count: i32 = conn.query_row_params(
            "SELECT COUNT(*) FROM knowledge WHERE id = ?1",
            [id.as_str()],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}

/// Memory Repository
#[derive(Debug, Clone)]
pub struct MemoryRepository;

impl MemoryRepository {
    pub fn new() -> Self {
        MemoryRepository
    }

    pub fn find_by_type(&self, conn: &DatabaseConnection, memory_type: MemoryType) -> StorageResult<Vec<MemoryItem>> {
        let type_str = serde_json::to_string(&memory_type).unwrap();
        let results: Vec<MemoryItem> = conn.query_params(
            "SELECT id, memory_type, content, context, confidence, provenance, relations, evidence, tags, metadata, created_at, updated_at, accessed_at, version, content_hash FROM memory_items WHERE memory_type = ?1",
            [&type_str],
            |row| {
                self.row_to_memory(row)
            },
        )?;
        Ok(results)
    }

    pub fn find_by_content_hash(&self, conn: &DatabaseConnection, hash: &str) -> StorageResult<Option<MemoryItem>> {
        let result: Option<MemoryItem> = conn.query_params(
            "SELECT id, memory_type, content, context, confidence, provenance, relations, evidence, tags, metadata, created_at, updated_at, accessed_at, version, content_hash FROM memory_items WHERE content_hash = ?1 LIMIT 1",
            [hash],
            |row| {
                self.row_to_memory(row)
            },
        )?;
        Ok(result)
    }

    fn row_to_memory(&self, row: &rusqlite::Row) -> StorageResult<MemoryItem> {
        let memory_type: MemoryType = serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or(MemoryType::Semantic);
        
        let relations_str: String = row.get(6)?;
        let relations: Vec<malverde_core::models::MemoryRelation> = serde_json::from_str(&relations_str).unwrap_or_default();
        
        let evidence_str: String = row.get(7)?;
        let evidence: Vec<Evidence> = serde_json::from_str(&evidence_str).unwrap_or_default();
        
        let tags_str: String = row.get(8)?;
        let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
        
        let metadata: serde_json::Value = row.get::<_, String>(9).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(serde_json::Value::Null);
        
        let provenance_str: Option<String> = row.get(5)?;
        let provenance = provenance_str.and_then(|s| serde_json::from_str(&s).ok());

        Ok(MemoryItem {
            id: MemoryId::from_string(&row.get::<_, String>(0)?).unwrap(),
            memory_type,
            content: row.get(2)?,
            context: row.get::<_, Option<String>>(3)?,
            confidence: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or(Confidence::Medium),
            provenance,
            relations,
            evidence,
            tags,
            metadata,
            created_at: CreatedAt::from(chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?).unwrap()),
            updated_at: UpdatedAt::from(chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?).unwrap()),
            accessed_at: row.get::<_, Option<String>>(12)?.map(|s| AccessedAt::from(chrono::DateTime::parse_from_rfc3339(&s).unwrap())),
            version: row.get(13)?,
        })
    }
}

impl Repository<MemoryItem, MemoryId> for MemoryRepository {
    fn save(&self, conn: &DatabaseConnection, entity: &MemoryItem) -> StorageResult<MemoryId> {
        let relations_json = serde_json::to_string(&entity.relations).unwrap();
        let evidence_json = serde_json::to_string(&entity.evidence).unwrap();
        let tags_json = serde_json::to_string(&entity.tags).unwrap();
        let provenance_json = entity.provenance.as_ref().map(|p| serde_json::to_string(p).unwrap()).unwrap_or_default();
        
        conn.execute_params(
            "INSERT INTO memory_items (id, memory_type, content, context, confidence, provenance, relations, evidence, tags, metadata, created_at, updated_at, accessed_at, version, content_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                entity.id.as_str(),
                serde_json::to_string(&entity.memory_type).unwrap(),
                &entity.content,
                entity.context.as_deref().unwrap_or(""),
                serde_json::to_string(&entity.confidence).unwrap(),
                &provenance_json,
                &relations_json,
                &evidence_json,
                &tags_json,
                serde_json::to_string(&entity.metadata).unwrap(),
                entity.created_at.to_rfc3339(),
                entity.updated_at.to_rfc3339(),
                entity.accessed_at.as_ref().map(|a| a.to_rfc3339()).unwrap_or(""),
                entity.version,
                &entity.content_hash(),
            ],
        )?;
        Ok(entity.id.clone())
    }

    fn find_by_id(&self, conn: &DatabaseConnection, id: &MemoryId) -> StorageResult<Option<MemoryItem>> {
        let result: Option<MemoryItem> = conn.query_params(
            "SELECT id, memory_type, content, context, confidence, provenance, relations, evidence, tags, metadata, created_at, updated_at, accessed_at, version, content_hash FROM memory_items WHERE id = ?1 LIMIT 1",
            [id.as_str()],
            |row| {
                self.row_to_memory(row)
            },
        )?;
        Ok(result)
    }

    fn find_all(&self, conn: &DatabaseConnection) -> StorageResult<Vec<MemoryItem>> {
        let results: Vec<MemoryItem> = conn.query(
            "SELECT id, memory_type, content, context, confidence, provenance, relations, evidence, tags, metadata, created_at, updated_at, accessed_at, version, content_hash FROM memory_items",
            |row| {
                self.row_to_memory(row)
            },
        )?;
        Ok(results)
    }

    fn update(&self, conn: &DatabaseConnection, entity: &MemoryItem) -> StorageResult<bool> {
        let relations_json = serde_json::to_string(&entity.relations).unwrap();
        let evidence_json = serde_json::to_string(&entity.evidence).unwrap();
        let tags_json = serde_json::to_string(&entity.tags).unwrap();
        let provenance_json = entity.provenance.as_ref().map(|p| serde_json::to_string(p).unwrap()).unwrap_or_default();
        
        let rows_affected = conn.execute_params(
            "UPDATE memory_items SET memory_type = ?1, content = ?2, context = ?3, confidence = ?4, provenance = ?5, relations = ?6, evidence = ?7, tags = ?8, metadata = ?9, updated_at = ?10, accessed_at = ?11, version = ?12, content_hash = ?13 WHERE id = ?14",
            params![
                serde_json::to_string(&entity.memory_type).unwrap(),
                &entity.content,
                entity.context.as_deref().unwrap_or(""),
                serde_json::to_string(&entity.confidence).unwrap(),
                &provenance_json,
                &relations_json,
                &evidence_json,
                &tags_json,
                serde_json::to_string(&entity.metadata).unwrap(),
                entity.updated_at.to_rfc3339(),
                entity.accessed_at.as_ref().map(|a| a.to_rfc3339()).unwrap_or(""),
                entity.version,
                &entity.content_hash(),
                entity.id.as_str(),
            ],
        )?;
        Ok(rows_affected > 0)
    }

    fn delete(&self, conn: &DatabaseConnection, id: &MemoryId) -> StorageResult<bool> {
        let rows_affected = conn.execute_params(
            "DELETE FROM memory_items WHERE id = ?1",
            [id.as_str()],
        )?;
        Ok(rows_affected > 0)
    }

    fn exists(&self, conn: &DatabaseConnection, id: &MemoryId) -> StorageResult<bool> {
        let count: i32 = conn.query_row_params(
            "SELECT COUNT(*) FROM memory_items WHERE id = ?1",
            [id.as_str()],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}

/// Event Repository
#[derive(Debug, Clone)]
pub struct EventRepository;

impl EventRepository {
    pub fn new() -> Self {
        EventRepository
    }

    pub fn find_by_type(&self, conn: &DatabaseConnection, event_type: EventType) -> StorageResult<Vec<Event>> {
        let type_str = serde_json::to_string(&event_type).unwrap();
        let results: Vec<Event> = conn.query_params(
            "SELECT id, event_type, timestamp, actor, actor_type, project_id, operation_id, input_hash, output_hash, evidence, confidence, provenance, parent_event_id, payload, metadata FROM events WHERE event_type = ?1",
            [&type_str],
            |row| {
                self.row_to_event(row)
            },
        )?;
        Ok(results)
    }

    pub fn find_by_actor(&self, conn: &DatabaseConnection, actor: &ActorId) -> StorageResult<Vec<Event>> {
        let results: Vec<Event> = conn.query_params(
            "SELECT id, event_type, timestamp, actor, actor_type, project_id, operation_id, input_hash, output_hash, evidence, confidence, provenance, parent_event_id, payload, metadata FROM events WHERE actor = ?1",
            [actor.as_str()],
            |row| {
                self.row_to_event(row)
            },
        )?;
        Ok(results)
    }

    pub fn find_by_project(&self, conn: &DatabaseConnection, project: &ProjectId) -> StorageResult<Vec<Event>> {
        let results: Vec<Event> = conn.query_params(
            "SELECT id, event_type, timestamp, actor, actor_type, project_id, operation_id, input_hash, output_hash, evidence, confidence, provenance, parent_event_id, payload, metadata FROM events WHERE project_id = ?1",
            [project.as_str()],
            |row| {
                self.row_to_event(row)
            },
        )?;
        Ok(results)
    }

    pub fn find_children(&self, conn: &DatabaseConnection, parent_id: &EventId) -> StorageResult<Vec<Event>> {
        let results: Vec<Event> = conn.query_params(
            "SELECT id, event_type, timestamp, actor, actor_type, project_id, operation_id, input_hash, output_hash, evidence, confidence, provenance, parent_event_id, payload, metadata FROM events WHERE parent_event_id = ?1",
            [parent_id.as_str()],
            |row| {
                self.row_to_event(row)
            },
        )?;
        Ok(results)
    }

    pub fn find_by_time_range(
        &self,
        conn: &DatabaseConnection,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> StorageResult<Vec<Event>> {
        let start_str = start.to_rfc3339();
        let end_str = end.to_rfc3339();
        let results: Vec<Event> = conn.query_params(
            "SELECT id, event_type, timestamp, actor, actor_type, project_id, operation_id, input_hash, output_hash, evidence, confidence, provenance, parent_event_id, payload, metadata FROM events WHERE timestamp >= ?1 AND timestamp <= ?2",
            [&start_str, &end_str],
            |row| {
                self.row_to_event(row)
            },
        )?;
        Ok(results)
    }

    fn row_to_event(&self, row: &rusqlite::Row) -> StorageResult<Event> {
        let event_type: EventType = serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or(EventType::Info);
        let actor_type: ActorType = serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or(ActorType::Unknown);
        let confidence: Confidence = serde_json::from_str(&row.get::<_, String>(10)?).unwrap_or(Confidence::Medium);
        
        let evidence_str: String = row.get(9)?;
        let evidence: Vec<Evidence> = serde_json::from_str(&evidence_str).unwrap_or_default();
        
        let provenance_str: Option<String> = row.get(11)?;
        let provenance = provenance_str.and_then(|s| serde_json::from_str(&s).ok());
        
        let payload_str: String = row.get(14)?;
        let payload_data: serde_json::Value = serde_json::from_str(&payload_str).unwrap_or(serde_json::Value::Null);
        let payload = EventPayload::new(payload_data, "unknown".to_string());
        
        let metadata_str: String = row.get(15)?;
        let metadata: EventMetadata = serde_json::from_str(&metadata_str).unwrap_or(EventMetadata::new());
        
        let parent_event_id: Option<EventId> = row.get::<_, Option<String>>(12)?.map(|s| EventId::from_string(&s).unwrap());
        let project_id: Option<ProjectId> = row.get::<_, Option<String>>(5)?.map(|s| ProjectId::from_string(&s).unwrap());
        let operation_id: Option<OperationId> = row.get::<_, Option<String>>(6)?.map(|s| OperationId::from_string(&s).unwrap());
        let input_hash: Option<String> = row.get(7)?;
        let output_hash: Option<String> = row.get(8)?;

        Ok(Event {
            id: EventId::from_string(&row.get::<_, String>(0)?).unwrap(),
            event_type,
            timestamp: CreatedAt::from(chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?).unwrap()),
            actor: ActorId::from_string(&row.get::<_, String>(3)?).unwrap(),
            actor_type,
            project: project_id,
            operation: operation_id,
            input_hash,
            output_hash,
            evidence,
            confidence,
            provenance,
            parent_event: parent_event_id,
            payload,
            metadata,
        })
    }
}

impl Repository<Event, EventId> for EventRepository {
    fn save(&self, conn: &DatabaseConnection, entity: &Event) -> StorageResult<EventId> {
        let evidence_json = serde_json::to_string(&entity.evidence).unwrap();
        let provenance_json = entity.provenance.as_ref().map(|p| serde_json::to_string(p).unwrap()).unwrap_or_default();
        let payload_json = serde_json::to_string(&entity.payload.data).unwrap();
        let metadata_json = serde_json::to_string(&entity.metadata).unwrap();
        
        conn.execute_params(
            "INSERT INTO events (id, event_type, timestamp, actor, actor_type, project_id, operation_id, input_hash, output_hash, evidence, confidence, provenance, parent_event_id, payload, metadata) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                entity.id.as_str(),
                serde_json::to_string(&entity.event_type).unwrap(),
                entity.timestamp.to_rfc3339(),
                entity.actor.as_str(),
                serde_json::to_string(&entity.actor_type).unwrap(),
                entity.project.as_ref().map(|p| p.as_str()).unwrap_or(""),
                entity.operation.as_ref().map(|o| o.as_str()).unwrap_or(""),
                entity.input_hash.as_deref().unwrap_or(""),
                entity.output_hash.as_deref().unwrap_or(""),
                &evidence_json,
                serde_json::to_string(&entity.confidence).unwrap(),
                &provenance_json,
                entity.parent_event.as_ref().map(|p| p.as_str()).unwrap_or(""),
                &payload_json,
                &metadata_json,
            ],
        )?;
        Ok(entity.id.clone())
    }

    fn find_by_id(&self, conn: &DatabaseConnection, id: &EventId) -> StorageResult<Option<Event>> {
        let result: Option<Event> = conn.query_params(
            "SELECT id, event_type, timestamp, actor, actor_type, project_id, operation_id, input_hash, output_hash, evidence, confidence, provenance, parent_event_id, payload, metadata FROM events WHERE id = ?1 LIMIT 1",
            [id.as_str()],
            |row| {
                self.row_to_event(row)
            },
        )?;
        Ok(result)
    }

    fn find_all(&self, conn: &DatabaseConnection) -> StorageResult<Vec<Event>> {
        let results: Vec<Event> = conn.query(
            "SELECT id, event_type, timestamp, actor, actor_type, project_id, operation_id, input_hash, output_hash, evidence, confidence, provenance, parent_event_id, payload, metadata FROM events",
            |row| {
                self.row_to_event(row)
            },
        )?;
        Ok(results)
    }

    fn update(&self, conn: &DatabaseConnection, entity: &Event) -> StorageResult<bool> {
        let evidence_json = serde_json::to_string(&entity.evidence).unwrap();
        let provenance_json = entity.provenance.as_ref().map(|p| serde_json::to_string(p).unwrap()).unwrap_or_default();
        let payload_json = serde_json::to_string(&entity.payload.data).unwrap();
        let metadata_json = serde_json::to_string(&entity.metadata).unwrap();
        
        let rows_affected = conn.execute_params(
            "UPDATE events SET event_type = ?1, timestamp = ?2, actor = ?3, actor_type = ?4, project_id = ?5, operation_id = ?6, input_hash = ?7, output_hash = ?8, evidence = ?9, confidence = ?10, provenance = ?11, parent_event_id = ?12, payload = ?13, metadata = ?14 WHERE id = ?15",
            params![
                serde_json::to_string(&entity.event_type).unwrap(),
                entity.timestamp.to_rfc3339(),
                entity.actor.as_str(),
                serde_json::to_string(&entity.actor_type).unwrap(),
                entity.project.as_ref().map(|p| p.as_str()).unwrap_or(""),
                entity.operation.as_ref().map(|o| o.as_str()).unwrap_or(""),
                entity.input_hash.as_deref().unwrap_or(""),
                entity.output_hash.as_deref().unwrap_or(""),
                &evidence_json,
                serde_json::to_string(&entity.confidence).unwrap(),
                &provenance_json,
                entity.parent_event.as_ref().map(|p| p.as_str()).unwrap_or(""),
                &payload_json,
                &metadata_json,
                entity.id.as_str(),
            ],
        )?;
        Ok(rows_affected > 0)
    }

    fn delete(&self, conn: &DatabaseConnection, id: &EventId) -> StorageResult<bool> {
        let rows_affected = conn.execute_params(
            "DELETE FROM events WHERE id = ?1",
            [id.as_str()],
        )?;
        Ok(rows_affected > 0)
    }

    fn exists(&self, conn: &DatabaseConnection, id: &EventId) -> StorageResult<bool> {
        let count: i32 = conn.query_row_params(
            "SELECT COUNT(*) FROM events WHERE id = ?1",
            [id.as_str()],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }
}

/// Configuration Repository
#[derive(Debug, Clone)]
pub struct ConfigurationRepository;

impl ConfigurationRepository {
    pub fn new() -> Self {
        ConfigurationRepository
    }

    pub fn get_value(&self, conn: &DatabaseConnection, key: &str) -> StorageResult<Option<String>> {
        let result: Option<String> = conn.query_row_params(
            "SELECT value FROM configuration WHERE key = ?1",
            [key],
            |row| row.get(0),
        )?;
        Ok(result)
    }

    pub fn set_value(&self, conn: &DatabaseConnection, key: &str, value: &str, description: Option<&str>) -> StorageResult<()> {
        let desc = description.unwrap_or("");
        let now = chrono::Utc::now().to_rfc3339();
        
        // Try to update first
        let rows_affected = conn.execute_params(
            "UPDATE configuration SET value = ?1, description = ?2, updated_at = ?3 WHERE key = ?4",
            params![value, desc, &now, key],
        )?;
        
        if rows_affected == 0 {
            // Insert if not exists
            conn.execute_params(
                "INSERT INTO configuration (key, value, description, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params![key, value, desc, &now],
            )?;
        }
        
        Ok(())
    }

    pub fn delete_value(&self, conn: &DatabaseConnection, key: &str) -> StorageResult<bool> {
        let rows_affected = conn.execute_params(
            "DELETE FROM configuration WHERE key = ?1",
            [key],
        )?;
        Ok(rows_affected > 0)
    }

    pub fn get_all(&self, conn: &DatabaseConnection) -> StorageResult<HashMap<String, String>> {
        let mut map = HashMap::new();
        let rows: Vec<(String, String)> = conn.query(
            "SELECT key, value FROM configuration",
            |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            },
        )?;
        for (key, value) in rows {
            map.insert(key, value);
        }
        Ok(map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DatabaseConfig;
    use crate::migration::apply_all_migrations;
    use tempfile::NamedTempFile;

    #[test]
    fn test_project_repository() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        let config = DatabaseConfig::new(path);
        let mut conn = DatabaseConnection::open(&config).unwrap();
        apply_all_migrations(&mut conn).unwrap();

        let repo = ProjectRepository::new();
        
        let project = Project::new(
            ProjectId::from_string("test-project").unwrap(),
            "Test Project",
            ActorId::from_string("test-actor").unwrap(),
        );

        // Save
        let saved_id = repo.save(&conn, &project).unwrap();
        assert_eq!(saved_id, project.id);

        // Find by ID
        let found = repo.find_by_id(&conn, &project.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Test Project");

        // Find all
        let all = repo.find_all(&conn).unwrap();
        assert_eq!(all.len(), 1);

        // Update
        let mut updated_project = project;
        updated_project.name = "Updated Project".to_string();
        assert!(repo.update(&conn, &updated_project).unwrap());

        let found = repo.find_by_id(&conn, &project.id).unwrap();
        assert_eq!(found.unwrap().name, "Updated Project");

        // Delete
        assert!(repo.delete(&conn, &project.id).unwrap());
        assert!(!repo.exists(&conn, &project.id).unwrap());
    }

    #[test]
    fn test_knowledge_repository() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        let config = DatabaseConfig::new(path);
        let mut conn = DatabaseConnection::open(&config).unwrap();
        apply_all_migrations(&mut conn).unwrap();

        let repo = KnowledgeRepository::new();
        
        let knowledge = KnowledgeItem::new(
            KnowledgeId::from_string("test-knowledge").unwrap(),
            "TestType",
            "Test content",
        );

        // Save
        let saved_id = repo.save(&conn, &knowledge).unwrap();
        assert_eq!(saved_id, knowledge.id);

        // Find by ID
        let found = repo.find_by_id(&conn, &knowledge.id).unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().content, "Test content");

        // Find by type
        let by_type = repo.find_by_type(&conn, "TestType").unwrap();
        assert_eq!(by_type.len(), 1);

        // Find by content hash
        let hash = knowledge.content_hash();
        let by_hash = repo.find_by_content_hash(&conn, &hash).unwrap();
        assert!(by_hash.is_some());
    }

    #[test]
    fn test_configuration_repository() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();
        
        let config = DatabaseConfig::new(path);
        let mut conn = DatabaseConnection::open(&config).unwrap();
        apply_all_migrations(&mut conn).unwrap();

        let repo = ConfigurationRepository::new();
        
        // Set value
        repo.set_value(&conn, "test.key", "test.value", Some("Test description")).unwrap();

        // Get value
        let value = repo.get_value(&conn, "test.key").unwrap();
        assert_eq!(value, Some("test.value".to_string()));

        // Update value
        repo.set_value(&conn, "test.key", "updated.value", None).unwrap();
        let value = repo.get_value(&conn, "test.key").unwrap();
        assert_eq!(value, Some("updated.value".to_string()));

        // Delete value
        assert!(repo.delete_value(&conn, "test.key").unwrap());
        let value = repo.get_value(&conn, "test.key").unwrap();
        assert!(value.is_none());

        // Get all
        repo.set_value(&conn, "key1", "value1", None).unwrap();
        repo.set_value(&conn, "key2", "value2", None).unwrap();
        let all = repo.get_all(&conn).unwrap();
        assert_eq!(all.len(), 2);
    }
}
