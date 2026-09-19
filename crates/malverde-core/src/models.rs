//! Domain models for the Malverde Framework

use crate::error::*;
use crate::ids::*;
use crate::states::*;
use crate::timestamps::*;
use crate::trust::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Actor represents who performed an action
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Actor {
    pub id: ActorId,
    pub actor_type: ActorType,
    pub name: String,
    pub description: Option<String>,
    pub trust_level: TrustLevel,
    pub metadata: serde_json::Value,
    pub created_at: CreatedAt,
    pub updated_at: UpdatedAt,
}

impl Actor {
    pub fn new(id: ActorId, actor_type: ActorType, name: impl Into<String>) -> Self {
        let now = CreatedAt::now();
        Actor {
            id, actor_type, name: name.into(), description: None,
            trust_level: TrustLevel::default(), metadata: serde_json::Value::Null,
            created_at: now, updated_at: UpdatedAt::from(now.0.clone()),
        }
    }
    pub fn with_description(mut self, description: impl Into<String>) -> Self { self.description = Some(description.into()); self }
    pub fn with_trust_level(mut self, trust_level: TrustLevel) -> Self { self.trust_level = trust_level; self }
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self { self.metadata = metadata; self }
    pub fn update(&mut self, name: Option<String>, description: Option<String>) {
        if let Some(n) = name { self.name = n; }
        if let Some(d) = description { self.description = Some(d); }
        self.updated_at = UpdatedAt::now();
    }
}

/// Project represents a workspace or context for operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub description: Option<String>,
    pub owner: ActorId,
    pub state: ProjectState,
    pub metadata: serde_json::Value,
    pub created_at: CreatedAt,
    pub updated_at: UpdatedAt,
    pub accessed_at: Option<AccessedAt>,
}

impl Project {
    pub fn new(id: ProjectId, name: impl Into<String>, owner: ActorId) -> Self {
        let now = CreatedAt::now();
        Project {
            id, name: name.into(), description: None, owner,
            state: ProjectState::default(), metadata: serde_json::Value::Null,
            created_at: now, updated_at: UpdatedAt::from(now.0.clone()), accessed_at: None,
        }
    }
    pub fn with_description(mut self, description: impl Into<String>) -> Self { self.description = Some(description.into()); self }
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self { self.metadata = metadata; self }
    pub fn update(&mut self, name: Option<String>, description: Option<String>) {
        if let Some(n) = name { self.name = n; }
        if let Some(d) = description { self.description = Some(d); }
        self.updated_at = UpdatedAt::now();
    }
    pub fn mark_accessed(&mut self) { self.accessed_at = Some(AccessedAt::now()); }
}

/// Operation represents an action performed in the system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operation {
    pub id: OperationId,
    pub operation_type: OperationType,
    pub actor: ActorId,
    pub project: Option<ProjectId>,
    pub description: Option<String>,
    pub input: serde_json::Value,
    pub output: Option<serde_json::Value>,
    pub state: OperationState,
    pub confidence: Confidence,
    pub evidence: Vec<Evidence>,
    pub errors: Vec<MalverdeError>,
    pub metadata: serde_json::Value,
    pub created_at: CreatedAt,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub parent_operation: Option<OperationId>,
}

impl Operation {
    pub fn new(id: OperationId, operation_type: OperationType, actor: ActorId, input: serde_json::Value) -> Self {
        Operation {
            id, operation_type, actor, project: None, description: None, input,
            output: None, state: OperationState::default(), confidence: Confidence::default(),
            evidence: Vec::new(), errors: Vec::new(), metadata: serde_json::Value::Null,
            created_at: CreatedAt::now(), started_at: None, completed_at: None, parent_operation: None,
        }
    }
    pub fn with_project(mut self, project: ProjectId) -> Self { self.project = Some(project); self }
    pub fn with_description(mut self, description: impl Into<String>) -> Self { self.description = Some(description.into()); self }
    pub fn with_output(mut self, output: serde_json::Value) -> Self { self.output = Some(output); self }
    pub fn with_state(mut self, state: OperationState) -> Self { self.state = state; self }
    pub fn with_confidence(mut self, confidence: Confidence) -> Self { self.confidence = confidence; self }
    pub fn add_evidence(&mut self, evidence: Evidence) { self.evidence.push(evidence); }
    pub fn add_error(&mut self, error: MalverdeError) { self.errors.push(error); }
    pub fn mark_started(&mut self) { self.state = OperationState::Running; self.started_at = Some(Utc::now()); }
    pub fn mark_completed(&mut self, output: serde_json::Value) {
        self.state = OperationState::Completed; self.output = Some(output); self.completed_at = Some(Utc::now());
    }
    pub fn mark_failed(&mut self, error: MalverdeError) {
        self.state = OperationState::Failed; self.errors.push(error); self.completed_at = Some(Utc::now());
    }
    pub fn duration(&self) -> Option<std::time::Duration> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => Some(end - start),
            _ => None,
        }
    }
}

/// Command represents a command to be executed
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Command {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub args: Vec<String>,
    pub options: HashMap<String, String>,
    pub input: Option<serde_json::Value>,
    pub output_type: Option<String>,
    pub metadata: serde_json::Value,
}

impl Command {
    pub fn new(name: impl Into<String>) -> Self {
        Command {
            id: uuid::Uuid::new_v4().to_string(), name: name.into(),
            description: None, args: Vec::new(), options: HashMap::new(),
            input: None, output_type: None, metadata: serde_json::Value::Null,
        }
    }
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self { self.args.push(arg.into()); self }
    pub fn with_args(mut self, args: Vec<String>) -> Self { self.args = args; self }
    pub fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert(key.into(), value.into()); self
    }
    pub fn with_input(mut self, input: serde_json::Value) -> Self { self.input = Some(input); self }
    pub fn with_description(mut self, description: impl Into<String>) -> Self { self.description = Some(description.into()); self }
    pub fn with_output_type(mut self, output_type: impl Into<String>) -> Self { self.output_type = Some(output_type.into()); self }
}

/// Result of a command execution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandResult {
    pub id: String,
    pub command_id: String,
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub metadata: serde_json::Value,
    pub executed_at: DateTime<Utc>,
    pub duration: Option<std::time::Duration>,
}

impl CommandResult {
    pub fn success(command_id: impl Into<String>, output: serde_json::Value) -> Self {
        CommandResult {
            id: uuid::Uuid::new_v4().to_string(), command_id: command_id.into(),
            success: true, output: Some(output), errors: Vec::new(), warnings: Vec::new(),
            metadata: serde_json::Value::Null, executed_at: Utc::now(), duration: None,
        }
    }
    pub fn failure(command_id: impl Into<String>, errors: Vec<String>) -> Self {
        CommandResult {
            id: uuid::Uuid::new_v4().to_string(), command_id: command_id.into(),
            success: false, output: None, errors, warnings: Vec::new(),
            metadata: serde_json::Value::Null, executed_at: Utc::now(), duration: None,
        }
    }
    pub fn with_error(mut self, error: impl Into<String>) -> Self { self.errors.push(error.into()); self.success = false; self }
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self { self.warnings.push(warning.into()); self }
    pub fn with_duration(mut self, duration: std::time::Duration) -> Self { self.duration = Some(duration); self }
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self { self.metadata = metadata; self }
}

/// Relation type for knowledge and memory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationType {
    Related, DependsOn, Implies, Contradicts, Supports, PartOf, HasPart, Similar, Opposite,
}

impl std::fmt::Display for RelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Knowledge relation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeRelation {
    pub relation_type: RelationType,
    pub target_id: KnowledgeId,
    pub strength: f32,
    pub description: Option<String>,
}

impl KnowledgeRelation {
    pub fn new(relation_type: RelationType, target_id: KnowledgeId) -> Self {
        KnowledgeRelation { relation_type, target_id, strength: 1.0, description: None }
    }
    pub fn with_strength(mut self, strength: f32) -> Self { self.strength = strength.clamp(0.0, 1.0); self }
    pub fn with_description(mut self, description: impl Into<String>) -> Self { self.description = Some(description.into()); self }
}

/// Memory relation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryRelation {
    pub relation_type: RelationType,
    pub target_id: MemoryId,
    pub strength: f32,
}

impl MemoryRelation {
    pub fn new(relation_type: RelationType, target_id: MemoryId) -> Self {
        MemoryRelation { relation_type, target_id, strength: 1.0 }
    }
    pub fn with_strength(mut self, strength: f32) -> Self { self.strength = strength.clamp(0.0, 1.0); self }
}

/// Knowledge item represents a piece of knowledge in the system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeItem {
    pub id: KnowledgeId,
    pub knowledge_type: String,
    pub content: String,
    pub context: Option<String>,
    pub state: KnowledgeState,
    pub confidence: Confidence,
    pub source: Option<String>,
    pub provenance: Option<Provenance>,
    pub relations: Vec<KnowledgeRelation>,
    pub evidence: Vec<Evidence>,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub created_at: CreatedAt,
    pub updated_at: UpdatedAt,
    pub version: u64,
}

impl KnowledgeItem {
    pub fn new(id: KnowledgeId, knowledge_type: impl Into<String>, content: impl Into<String>) -> Self {
        let now = CreatedAt::now();
        KnowledgeItem {
            id, knowledge_type: knowledge_type.into(), content: content.into(),
            context: None, state: KnowledgeState::default(), confidence: Confidence::default(),
            source: None, provenance: None, relations: Vec::new(), evidence: Vec::new(),
            tags: Vec::new(), metadata: serde_json::Value::Null,
            created_at: now, updated_at: UpdatedAt::from(now.0.clone()), version: 1,
        }
    }
    pub fn with_context(mut self, context: impl Into<String>) -> Self { self.context = Some(context.into()); self }
    pub fn with_state(mut self, state: KnowledgeState) -> Self { self.state = state; self }
    pub fn with_confidence(mut self, confidence: Confidence) -> Self { self.confidence = confidence; self }
    pub fn with_source(mut self, source: impl Into<String>) -> Self { self.source = Some(source.into()); self }
    pub fn with_provenance(mut self, provenance: Provenance) -> Self { self.provenance = Some(provenance); self }
    pub fn with_relation(mut self, relation: KnowledgeRelation) -> Self { self.relations.push(relation); self }
    pub fn with_evidence(mut self, evidence: Vec<Evidence>) -> Self { self.evidence = evidence; self }
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self { self.tags.push(tag.into()); self }
    pub fn with_tags(mut self, tags: Vec<String>) -> Self { self.tags = tags; self }
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self { self.metadata = metadata; self }
    pub fn update(&mut self, content: Option<String>, state: Option<KnowledgeState>) {
        if let Some(c) = content { self.content = c; }
        if let Some(s) = state { self.state = s; }
        self.updated_at = UpdatedAt::now(); self.version += 1;
    }
    pub fn add_evidence(&mut self, evidence: Evidence) {
        self.evidence.push(evidence);
        if !self.evidence.is_empty() {
            let total: u32 = self.evidence.iter().map(|e| e.confidence.as_u8() as u32).sum();
            let avg = total / self.evidence.len() as u32;
            self.confidence = Confidence::from_u8(avg as u8);
        }
        self.updated_at = UpdatedAt::now(); self.version += 1;
    }
    pub fn promote(&mut self) -> KnowledgeState {
        let old = self.state;
        if let Some(new) = old.next() { self.state = new; self.updated_at = UpdatedAt::now(); self.version += 1; }
        old
    }
    pub fn demote(&mut self) -> KnowledgeState {
        let old = self.state;
        if let Some(new) = old.prev() { self.state = new; self.updated_at = UpdatedAt::now(); self.version += 1; }
        old
    }
    pub fn is_confirmed(&self) -> bool { self.state.is_confirmed() }
    pub fn content_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.knowledge_type);
        hasher.update(&self.content);
        if let Some(ctx) = &self.context { hasher.update(ctx); }
        format!("{:x}", hasher.finalize())
    }
}

/// Memory item represents a piece of memory in the system
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: MemoryId,
    pub memory_type: MemoryType,
    pub content: String,
    pub context: Option<String>,
    pub confidence: Confidence,
    pub provenance: Option<Provenance>,
    pub relations: Vec<MemoryRelation>,
    pub evidence: Vec<Evidence>,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub created_at: CreatedAt,
    pub updated_at: UpdatedAt,
    pub accessed_at: Option<AccessedAt>,
    pub version: u64,
}

impl MemoryItem {
    pub fn new(id: MemoryId, memory_type: MemoryType, content: impl Into<String>) -> Self {
        let now = CreatedAt::now();
        MemoryItem {
            id, memory_type, content: content.into(), context: None,
            confidence: Confidence::default(), provenance: None, relations: Vec::new(),
            evidence: Vec::new(), tags: Vec::new(), metadata: serde_json::Value::Null,
            created_at: now, updated_at: UpdatedAt::from(now.0.clone()),
            accessed_at: None, version: 1,
        }
    }
    pub fn with_context(mut self, context: impl Into<String>) -> Self { self.context = Some(context.into()); self }
    pub fn with_confidence(mut self, confidence: Confidence) -> Self { self.confidence = confidence; self }
    pub fn with_provenance(mut self, provenance: Provenance) -> Self { self.provenance = Some(provenance); self }
    pub fn with_relation(mut self, relation: MemoryRelation) -> Self { self.relations.push(relation); self }
    pub fn with_evidence(mut self, evidence: Vec<Evidence>) -> Self { self.evidence = evidence; self }
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self { self.tags.push(tag.into()); self }
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self { self.metadata = metadata; self }
    pub fn update(&mut self, content: Option<String>) {
        if let Some(c) = content { self.content = c; }
        self.updated_at = UpdatedAt::now(); self.version += 1;
    }
    pub fn mark_accessed(&mut self) { self.accessed_at = Some(AccessedAt::now()); }
    pub fn add_evidence(&mut self, evidence: Evidence) {
        self.evidence.push(evidence);
        if !self.evidence.is_empty() {
            let total: u32 = self.evidence.iter().map(|e| e.confidence.as_u8() as u32).sum();
            let avg = total / self.evidence.len() as u32;
            self.confidence = Confidence::from_u8(avg as u8);
        }
        self.updated_at = UpdatedAt::now(); self.version += 1;
    }
    pub fn content_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.memory_type.to_string());
        hasher.update(&self.content);
        if let Some(ctx) = &self.context { hasher.update(ctx); }
        format!("{:x}", hasher.finalize())
    }
}

/// Pattern represents a recognized pattern in the data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Pattern {
    pub id: PatternId,
    pub name: String,
    pub description: Option<String>,
    pub pattern_type: String,
    pub pattern: String,
    pub examples: Vec<String>,
    pub confidence: Confidence,
    pub provenance: Option<Provenance>,
    pub evidence: Vec<Evidence>,
    pub tags: Vec<String>,
    pub metadata: serde_json::Value,
    pub created_at: CreatedAt,
    pub updated_at: UpdatedAt,
    pub version: u64,
}

impl Pattern {
    pub fn new(id: PatternId, name: impl Into<String>, pattern_type: impl Into<String>, pattern: impl Into<String>) -> Self {
        let now = CreatedAt::now();
        Pattern {
            id, name: name.into(), description: None, pattern_type: pattern_type.into(),
            pattern: pattern.into(), examples: Vec::new(), confidence: Confidence::default(),
            provenance: None, evidence: Vec::new(), tags: Vec::new(), metadata: serde_json::Value::Null,
            created_at: now, updated_at: UpdatedAt::from(now.0.clone()), version: 1,
        }
    }
    pub fn with_description(mut self, description: impl Into<String>) -> Self { self.description = Some(description.into()); self }
    pub fn with_example(mut self, example: impl Into<String>) -> Self { self.examples.push(example.into()); self }
    pub fn with_examples(mut self, examples: Vec<String>) -> Self { self.examples = examples; self }
    pub fn with_confidence(mut self, confidence: Confidence) -> Self { self.confidence = confidence; self }
    pub fn with_provenance(mut self, provenance: Provenance) -> Self { self.provenance = Some(provenance); self }
    pub fn with_evidence(mut self, evidence: Vec<Evidence>) -> Self { self.evidence = evidence; self }
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self { self.tags.push(tag.into()); self }
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self { self.metadata = metadata; self }
    pub fn update(&mut self, pattern: Option<String>, name: Option<String>) {
        if let Some(p) = pattern { self.pattern = p; }
        if let Some(n) = name { self.name = n; }
        self.updated_at = UpdatedAt::now(); self.version += 1;
    }
    pub fn add_evidence(&mut self, evidence: Evidence) {
        self.evidence.push(evidence);
        if !self.evidence.is_empty() {
            let total: u32 = self.evidence.iter().map(|e| e.confidence.as_u8() as u32).sum();
            let avg = total / self.evidence.len() as u32;
            self.confidence = Confidence::from_u8(avg as u8);
        }
        self.updated_at = UpdatedAt::now(); self.version += 1;
    }
}
