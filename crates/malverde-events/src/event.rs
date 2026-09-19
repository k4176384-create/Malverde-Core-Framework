//! Event model and builder

use malverde_core::ids::{ActorId, EventId, EvidenceId, OperationId, ProjectId};
use malverde_core::states::{ActorType, EventType};
use malverde_core::timestamps::CreatedAt;
use malverde_core::trust::{Confidence, Evidence, Provenance};
use malverde_core::{MalverdeError, MalverdeResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

/// Event payload for flexible event data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventPayload {
    pub data: serde_json::Value,
    pub data_type: String,
}

impl EventPayload {
    pub fn new(data: serde_json::Value, data_type: impl Into<String>) -> Self {
        EventPayload {
            data,
            data_type: data_type.into(),
        }
    }
    pub fn with_data(data: serde_json::Value) -> Self {
        EventPayload {
            data,
            data_type: "unknown".to_string(),
        }
    }
    pub fn as_string(&self) -> Option<String> {
        self.data.as_str().map(|s| s.to_string())
    }
    pub fn as_object(&self) -> Option<&serde_json::Map<String, serde_json::Value>> {
        self.data.as_object()
    }
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(&self.data_type);
        hasher.update(self.data.to_string());
        format!("{:x}", hasher.finalize())
    }
}

/// Event metadata for additional context
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventMetadata {
    pub source: Option<String>,
    pub version: Option<String>,
    pub correlation_id: Option<String>,
    pub tags: Vec<String>,
    pub custom: HashMap<String, serde_json::Value>,
}

impl EventMetadata {
    pub fn new() -> Self {
        EventMetadata {
            source: None,
            version: None,
            correlation_id: None,
            tags: Vec::new(),
            custom: HashMap::new(),
        }
    }
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
    pub fn with_custom(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }
}

/// Malverde Event - the core event structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub id: EventId,
    pub event_type: EventType,
    pub timestamp: CreatedAt,
    pub actor: ActorId,
    pub actor_type: ActorType,
    pub project: Option<ProjectId>,
    pub operation: Option<OperationId>,
    pub input_hash: Option<String>,
    pub output_hash: Option<String>,
    pub evidence: Vec<Evidence>,
    pub confidence: Confidence,
    pub provenance: Option<Provenance>,
    pub parent_event: Option<EventId>,
    pub payload: EventPayload,
    pub metadata: EventMetadata,
}

impl Event {
    /// Create a new event with required fields
    pub fn new(
        event_type: EventType,
        actor: ActorId,
        actor_type: ActorType,
        payload: EventPayload,
    ) -> Self {
        Event {
            id: EventId::from_string(&Uuid::new_v4().to_string()).unwrap(),
            event_type,
            timestamp: CreatedAt::now(),
            actor,
            actor_type,
            project: None,
            operation: None,
            input_hash: None,
            output_hash: None,
            evidence: Vec::new(),
            confidence: Confidence::Medium,
            provenance: None,
            parent_event: None,
            payload,
            metadata: EventMetadata::new(),
        }
    }

    /// Builder-style constructor
    pub fn builder(event_type: EventType, actor: ActorId, actor_type: ActorType) -> EventBuilder {
        EventBuilder::new(event_type, actor, actor_type)
    }

    pub fn with_project(mut self, project: ProjectId) -> Self {
        self.project = Some(project);
        self
    }

    pub fn with_operation(mut self, operation: OperationId) -> Self {
        self.operation = Some(operation);
        self
    }

    pub fn with_input_hash(mut self, hash: impl Into<String>) -> Self {
        self.input_hash = Some(hash.into());
        self
    }

    pub fn with_output_hash(mut self, hash: impl Into<String>) -> Self {
        self.output_hash = Some(hash.into());
        self
    }

    pub fn with_evidence(mut self, evidence: Vec<Evidence>) -> Self {
        self.evidence = evidence;
        self
    }

    pub fn add_evidence(&mut self, evidence: Evidence) {
        self.evidence.push(evidence);
    }

    pub fn with_confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn with_provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = Some(provenance);
        self
    }

    pub fn with_parent_event(mut self, parent: EventId) -> Self {
        self.parent_event = Some(parent);
        self
    }

    pub fn with_metadata(mut self, metadata: EventMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Calculate the event's hash based on content
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.id.as_str());
        hasher.update(self.event_type.to_string());
        hasher.update(self.timestamp.to_rfc3339());
        hasher.update(self.actor.as_str());
        hasher.update(self.actor_type.to_string());
        if let Some(ref p) = self.project {
            hasher.update(p.as_str());
        }
        if let Some(ref o) = self.operation {
            hasher.update(o.as_str());
        }
        if let Some(ref ih) = self.input_hash {
            hasher.update(ih);
        }
        if let Some(ref oh) = self.output_hash {
            hasher.update(oh);
        }
        hasher.update(&self.payload.hash());
        format!("{:x}", hasher.finalize())
    }

    /// Validate the event
    pub fn validate(&self) -> MalverdeResult<()> {
        if self.id.as_str().is_empty() {
            return Err(MalverdeError::validation("Event ID cannot be empty"));
        }
        if self.timestamp.0 > chrono::Utc::now() {
            return Err(MalverdeError::validation("Event timestamp cannot be in the future"));
        }
        if self.actor.as_str().is_empty() {
            return Err(MalverdeError::validation("Event actor cannot be empty"));
        }
        Ok(())
    }

    /// Check if this event is a direct child of another event
    pub fn is_child_of(&self, parent_id: &EventId) -> bool {
        match &self.parent_event {
            Some(p) => p == parent_id,
            None => false,
        }
    }

    /// Get the event's lineage (all parent events)
    pub fn get_lineage(&self) -> Vec<EventId> {
        let mut lineage = Vec::new();
        let mut current = self.parent_event.clone();
        while let Some(parent) = current {
            lineage.push(parent.clone());
            // In a real implementation, you'd fetch the parent event and get its parent
            // For now, we just return what we have
            break; // This would be recursive in a full implementation
        }
        lineage
    }

    /// Check if event has evidence
    pub fn has_evidence(&self) -> bool {
        !self.evidence.is_empty()
    }

    /// Get average confidence from evidence
    pub fn average_evidence_confidence(&self) -> Confidence {
        if self.evidence.is_empty() {
            return Confidence::None;
        }
        let total: u32 = self.evidence.iter().map(|e| e.confidence.as_u8() as u32).sum();
        let avg = total / self.evidence.len() as u32;
        Confidence::from_u8(avg as u8)
    }
}

/// Event builder for fluent construction
pub struct EventBuilder {
    event_type: EventType,
    actor: ActorId,
    actor_type: ActorType,
    project: Option<ProjectId>,
    operation: Option<OperationId>,
    input_hash: Option<String>,
    output_hash: Option<String>,
    evidence: Vec<Evidence>,
    confidence: Confidence,
    provenance: Option<Provenance>,
    parent_event: Option<EventId>,
    payload: Option<EventPayload>,
    metadata: EventMetadata,
}

impl EventBuilder {
    pub fn new(event_type: EventType, actor: ActorId, actor_type: ActorType) -> Self {
        EventBuilder {
            event_type,
            actor,
            actor_type,
            project: None,
            operation: None,
            input_hash: None,
            output_hash: None,
            evidence: Vec::new(),
            confidence: Confidence::Medium,
            provenance: None,
            parent_event: None,
            payload: None,
            metadata: EventMetadata::new(),
        }
    }

    pub fn project(mut self, project: ProjectId) -> Self {
        self.project = Some(project);
        self
    }

    pub fn operation(mut self, operation: OperationId) -> Self {
        self.operation = Some(operation);
        self
    }

    pub fn input_hash(mut self, hash: impl Into<String>) -> Self {
        self.input_hash = Some(hash.into());
        self
    }

    pub fn output_hash(mut self, hash: impl Into<String>) -> Self {
        self.output_hash = Some(hash.into());
        self
    }

    pub fn evidence(mut self, evidence: Vec<Evidence>) -> Self {
        self.evidence = evidence;
        self
    }

    pub fn add_evidence(mut self, evidence: Evidence) -> Self {
        self.evidence.push(evidence);
        self
    }

    pub fn confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn provenance(mut self, provenance: Provenance) -> Self {
        self.provenance = Some(provenance);
        self
    }

    pub fn parent_event(mut self, parent: EventId) -> Self {
        self.parent_event = Some(parent);
        self
    }

    pub fn payload(mut self, payload: EventPayload) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn metadata(mut self, metadata: EventMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.metadata.source = Some(source.into());
        self
    }

    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.metadata.version = Some(version.into());
        self
    }

    pub fn correlation_id(mut self, id: impl Into<String>) -> Self {
        self.metadata.correlation_id = Some(id.into());
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.metadata.tags.push(tag.into());
        self
    }

    pub fn custom(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.custom.insert(key.into(), value);
        self
    }

    pub fn build(self) -> MalverdeResult<Event> {
        let payload = self.payload.ok_or_else(|| {
            MalverdeError::validation("Event payload is required")
        })?;

        Ok(Event {
            id: EventId::from_string(&Uuid::new_v4().to_string()).unwrap(),
            event_type: self.event_type,
            timestamp: CreatedAt::now(),
            actor: self.actor,
            actor_type: self.actor_type,
            project: self.project,
            operation: self.operation,
            input_hash: self.input_hash,
            output_hash: self.output_hash,
            evidence: self.evidence,
            confidence: self.confidence,
            provenance: self.provenance,
            parent_event: self.parent_event,
            payload,
            metadata: self.metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use malverde_core::ids::{ActorId, EventId};
    use malverde_core::states::EventType;

    #[test]
    fn test_event_creation() {
        let actor = ActorId::from_string("test-actor").unwrap();
        let payload = EventPayload::with_data(serde_json::json!({"key": "value"}));
        let event = Event::new(EventType::KnowledgeCreated, actor.clone(), ActorType::User, payload);

        assert!(!event.id.as_str().is_empty());
        assert_eq!(event.actor, actor);
        assert_eq!(event.event_type, EventType::KnowledgeCreated);
    }

    #[test]
    fn test_event_builder() {
        let actor = ActorId::from_string("test-actor").unwrap();
        let payload = EventPayload::with_data(serde_json::json!({"action": "test"}));
        
        let event = Event::builder(EventType::JobStarted, actor.clone(), ActorType::System)
            .project(ProjectId::from_string("test-project").unwrap())
            .operation(OperationId::from_string("test-op").unwrap())
            .payload(payload)
            .build()
            .unwrap();

        assert_eq!(event.event_type, EventType::JobStarted);
        assert!(event.project.is_some());
        assert!(event.operation.is_some());
    }

    #[test]
    fn test_event_hash() {
        let actor = ActorId::from_string("test-actor").unwrap();
        let payload = EventPayload::with_data(serde_json::json!({"test": 123}));
        let event = Event::new(EventType::Info, actor, ActorType::System, payload);

        let hash1 = event.hash();
        let hash2 = event.hash();
        assert_eq!(hash1, hash2);
        assert!(!hash1.is_empty());
    }

    #[test]
    fn test_event_validation() {
        let actor = ActorId::from_string("test-actor").unwrap();
        let payload = EventPayload::with_data(serde_json::json!({"test": "data"}));
        let event = Event::new(EventType::Info, actor, ActorType::System, payload);

        assert!(event.validate().is_ok());
    }
}
