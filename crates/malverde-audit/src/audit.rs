//! Audit record and logging system

use malverde_core::ids::{ActorId, AuditId, EventId, EvidenceId, JobId, OperationId, ProjectId};
use malverde_core::models::{Actor, Evidence, Operation};
use malverde_core::states::{ActorType, EventType, JobState, OperationState};
use malverde_core::timestamps::CreatedAt;
use malverde_core::trust::{Confidence, Provenance, TrustLevel};
use malverde_core::{MalverdeError, MalverdeResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Audit record represents a single audit entry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditRecord {
    pub id: AuditId,
    pub event_id: Option<EventId>,
    pub timestamp: CreatedAt,
    pub actor: ActorId,
    pub actor_type: ActorType,
    pub operation: String,
    pub input_hash: Option<String>,
    pub output_hash: Option<String>,
    pub evidence: Vec<Evidence>,
    pub confidence: Confidence,
    pub errors: Vec<String>,
    pub related_events: Vec<EventId>,
    pub project: Option<ProjectId>,
    pub job_id: Option<JobId>,
    pub operation_id: Option<OperationId>,
    pub metadata: serde_json::Value,
}

impl AuditRecord {
    /// Create a new audit record
    pub fn new(
        actor: ActorId,
        actor_type: ActorType,
        operation: impl Into<String>,
    ) -> Self {
        AuditRecord {
            id: AuditId::from_string(&Uuid::new_v4().to_string()).unwrap(),
            event_id: None,
            timestamp: CreatedAt::now(),
            actor,
            actor_type,
            operation: operation.into(),
            input_hash: None,
            output_hash: None,
            evidence: Vec::new(),
            confidence: Confidence::Medium,
            errors: Vec::new(),
            related_events: Vec::new(),
            project: None,
            job_id: None,
            operation_id: None,
            metadata: serde_json::Value::Null,
        }
    }

    /// Create an audit record from an event
    pub fn from_event(event: &malverde_events::event::Event) -> Self {
        let mut record = AuditRecord::new(
            event.actor.clone(),
            event.actor_type,
            format!("{:?}", event.event_type),
        );

        record.event_id = Some(event.id.clone());
        record.timestamp = event.timestamp;
        record.operation = format!("{:?}", event.event_type);
        record.input_hash = event.input_hash.clone();
        record.output_hash = event.output_hash.clone();
        record.evidence = event.evidence.clone();
        record.confidence = event.confidence;
        record.project = event.project.clone();
        record.metadata = serde_json::to_value(&event.metadata).unwrap_or(serde_json::Value::Null);

        if let Some(ref provenance) = event.provenance {
            record.metadata["provenance"] = serde_json::to_value(provenance).unwrap();
        }

        record
    }

    pub fn with_event_id(mut self, event_id: EventId) -> Self {
        self.event_id = Some(event_id);
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

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.errors.push(error.into());
        self
    }

    pub fn with_errors(mut self, errors: Vec<String>) -> Self {
        self.errors = errors;
        self
    }

    pub fn with_related_event(mut self, event_id: EventId) -> Self {
        self.related_events.push(event_id);
        self
    }

    pub fn with_related_events(mut self, events: Vec<EventId>) -> Self {
        self.related_events = events;
        self
    }

    pub fn with_project(mut self, project: ProjectId) -> Self {
        self.project = Some(project);
        self
    }

    pub fn with_job_id(mut self, job_id: JobId) -> Self {
        self.job_id = Some(job_id);
        self
    }

    pub fn with_operation_id(mut self, operation_id: OperationId) -> Self {
        self.operation_id = Some(operation_id);
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Calculate the record's hash
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.id.as_str());
        hasher.update(self.timestamp.to_rfc3339());
        hasher.update(self.actor.as_str());
        hasher.update(self.actor_type.to_string());
        hasher.update(&self.operation);
        if let Some(ref ih) = self.input_hash {
            hasher.update(ih);
        }
        if let Some(ref oh) = self.output_hash {
            hasher.update(oh);
        }
        format!("{:x}", hasher.finalize())
    }

    /// Validate the audit record
    pub fn validate(&self) -> AuditResult<()> {
        if self.id.as_str().is_empty() {
            return Err(AuditError::invalid_record("Audit ID cannot be empty"));
        }
        if self.actor.as_str().is_empty() {
            return Err(AuditError::invalid_record("Actor cannot be empty"));
        }
        if self.operation.is_empty() {
            return Err(AuditError::invalid_record("Operation cannot be empty"));
        }
        if self.timestamp.0 > chrono::Utc::now() {
            return Err(AuditError::invalid_record("Timestamp cannot be in the future"));
        }
        Ok(())
    }

    /// Check if record has errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
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

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Get evidence count
    pub fn evidence_count(&self) -> usize {
        self.evidence.len()
    }

    /// Get related events count
    pub fn related_events_count(&self) -> usize {
        self.related_events.len()
    }
}

/// Audit record builder for fluent construction
pub struct AuditRecordBuilder {
    actor: ActorId,
    actor_type: ActorType,
    operation: String,
    event_id: Option<EventId>,
    input_hash: Option<String>,
    output_hash: Option<String>,
    evidence: Vec<Evidence>,
    confidence: Confidence,
    errors: Vec<String>,
    related_events: Vec<EventId>,
    project: Option<ProjectId>,
    job_id: Option<JobId>,
    operation_id: Option<OperationId>,
    metadata: serde_json::Value,
}

impl AuditRecordBuilder {
    pub fn new(actor: ActorId, actor_type: ActorType, operation: impl Into<String>) -> Self {
        AuditRecordBuilder {
            actor,
            actor_type,
            operation: operation.into(),
            event_id: None,
            input_hash: None,
            output_hash: None,
            evidence: Vec::new(),
            confidence: Confidence::Medium,
            errors: Vec::new(),
            related_events: Vec::new(),
            project: None,
            job_id: None,
            operation_id: None,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn event_id(mut self, event_id: EventId) -> Self {
        self.event_id = Some(event_id);
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

    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.errors.push(error.into());
        self
    }

    pub fn errors(mut self, errors: Vec<String>) -> Self {
        self.errors = errors;
        self
    }

    pub fn related_event(mut self, event_id: EventId) -> Self {
        self.related_events.push(event_id);
        self
    }

    pub fn related_events(mut self, events: Vec<EventId>) -> Self {
        self.related_events = events;
        self
    }

    pub fn project(mut self, project: ProjectId) -> Self {
        self.project = Some(project);
        self
    }

    pub fn job_id(mut self, job_id: JobId) -> Self {
        self.job_id = Some(job_id);
        self
    }

    pub fn operation_id(mut self, operation_id: OperationId) -> Self {
        self.operation_id = Some(operation_id);
        self
    }

    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn build(self) -> AuditRecord {
        AuditRecord {
            id: AuditId::from_string(&Uuid::new_v4().to_string()).unwrap(),
            event_id: self.event_id,
            timestamp: CreatedAt::now(),
            actor: self.actor,
            actor_type: self.actor_type,
            operation: self.operation,
            input_hash: self.input_hash,
            output_hash: self.output_hash,
            evidence: self.evidence,
            confidence: self.confidence,
            errors: self.errors,
            related_events: self.related_events,
            project: self.project,
            job_id: self.job_id,
            operation_id: self.operation_id,
            metadata: self.metadata,
        }
    }
}

/// Audit logger for recording audit entries
#[derive(Debug, Clone)]
pub struct AuditLogger {
    enabled: bool,
    records: Arc<RwLock<VecDeque<AuditRecord>>>,
    max_records: usize,
}

impl AuditLogger {
    pub fn new() -> Self {
        AuditLogger {
            enabled: true,
            records: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            max_records: 10000,
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_max_records(mut self, max: usize) -> Self {
        self.max_records = max;
        self
    }

    /// Check if audit logging is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable audit logging
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable audit logging
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Log an audit record
    pub fn log(&self, record: AuditRecord) -> AuditResult<AuditId> {
        if !self.enabled {
            return Err(AuditError::disabled());
        }

        record.validate()?;

        let mut records = self.records.write().unwrap();
        
        // If we have too many records, remove the oldest
        if records.len() >= self.max_records {
            records.pop_front();
        }

        records.push_back(record.clone());
        
        Ok(record.id.clone())
    }

    /// Log a simple audit entry
    pub fn log_simple(
        &self,
        actor: ActorId,
        actor_type: ActorType,
        operation: impl Into<String>,
    ) -> AuditResult<AuditId> {
        let record = AuditRecord::new(actor, actor_type, operation);
        self.log(record)
    }

    /// Log an audit entry with event
    pub fn log_with_event(&self, event: &malverde_events::event::Event) -> AuditResult<AuditId> {
        let record = AuditRecord::from_event(event);
        self.log(record)
    }

    /// Get all audit records
    pub fn all(&self) -> Vec<AuditRecord> {
        let records = self.records.read().unwrap();
        records.iter().cloned().collect()
    }

    /// Get audit records by actor
    pub fn by_actor(&self, actor: &ActorId) -> Vec<AuditRecord> {
        let records = self.records.read().unwrap();
        records
            .iter()
            .filter(|r| &r.actor == actor)
            .cloned()
            .collect()
    }

    /// Get audit records by operation
    pub fn by_operation(&self, operation: &str) -> Vec<AuditRecord> {
        let records = self.records.read().unwrap();
        records
            .iter()
            .filter(|r| r.operation == operation)
            .cloned()
            .collect()
    }

    /// Get audit records by project
    pub fn by_project(&self, project: &ProjectId) -> Vec<AuditRecord> {
        let records = self.records.read().unwrap();
        records
            .iter()
            .filter(|r| r.project.as_ref() == Some(project))
            .cloned()
            .collect()
    }

    /// Get audit records by job
    pub fn by_job(&self, job_id: &JobId) -> Vec<AuditRecord> {
        let records = self.records.read().unwrap();
        records
            .iter()
            .filter(|r| r.job_id.as_ref() == Some(job_id))
            .cloned()
            .collect()
    }

    /// Get audit records with errors
    pub fn with_errors(&self) -> Vec<AuditRecord> {
        let records = self.records.read().unwrap();
        records
            .iter()
            .filter(|r| r.has_errors())
            .cloned()
            .collect()
    }

    /// Get audit records in time range
    pub fn in_time_range(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Vec<AuditRecord> {
        let records = self.records.read().unwrap();
        records
            .iter()
            .filter(|r| r.timestamp.0 >= start && r.timestamp.0 <= end)
            .cloned()
            .collect()
    }

    /// Get the most recent audit records
    pub fn recent(&self, count: usize) -> Vec<AuditRecord> {
        let records = self.records.read().unwrap();
        records
            .iter()
            .rev()
            .take(count)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Get audit records by confidence
    pub fn by_confidence(&self, min_confidence: Confidence) -> Vec<AuditRecord> {
        let records = self.records.read().unwrap();
        records
            .iter()
            .filter(|r| r.confidence >= min_confidence)
            .cloned()
            .collect()
    }

    /// Get the number of audit records
    pub fn len(&self) -> usize {
        let records = self.records.read().unwrap();
        records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clear all audit records
    pub fn clear(&self) {
        let mut records = self.records.write().unwrap();
        records.clear();
    }

    /// Get statistics
    pub fn stats(&self) -> AuditStats {
        let records = self.records.read().unwrap();
        
        let mut by_actor: HashMap<ActorId, usize> = HashMap::new();
        let mut by_operation: HashMap<String, usize> = HashMap::new();
        let mut error_count = 0;
        let mut total_confidence: u32 = 0;
        let mut confidence_count = 0;
        
        for record in records.iter() {
            *by_actor.entry(record.actor.clone()).or_insert(0) += 1;
            *by_operation.entry(record.operation.clone()).or_insert(0) += 1;
            
            if record.has_errors() {
                error_count += 1;
            }
            
            total_confidence += record.confidence.as_u8() as u32;
            confidence_count += 1;
        }
        
        let avg_confidence = if confidence_count > 0 {
            Confidence::from_u8((total_confidence / confidence_count) as u8)
        } else {
            Confidence::None
        };
        
        AuditStats {
            total_records: records.len(),
            by_actor,
            by_operation,
            error_count,
            avg_confidence,
        }
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// Audit statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditStats {
    pub total_records: usize,
    pub by_actor: HashMap<ActorId, usize>,
    pub by_operation: HashMap<String, usize>,
    pub error_count: usize,
    pub avg_confidence: Confidence,
}

impl AuditStats {
    pub fn get_actor_count(&self, actor: &ActorId) -> usize {
        self.by_actor.get(actor).copied().unwrap_or(0)
    }

    pub fn get_operation_count(&self, operation: &str) -> usize {
        self.by_operation.get(operation).copied().unwrap_or(0)
    }

    pub fn error_rate(&self) -> f64 {
        if self.total_records == 0 {
            0.0
        } else {
            self.error_count as f64 / self.total_records as f64
        }
    }
}

/// Global audit logger singleton
#[derive(Debug, Clone)]
pub struct GlobalAuditLogger {
    logger: Option<Arc<AuditLogger>>,
}

impl GlobalAuditLogger {
    pub fn new() -> Self {
        GlobalAuditLogger { logger: None }
    }

    pub fn init(&mut self) -> Arc<AuditLogger> {
        let logger = AuditLogger::new();
        self.logger = Some(Arc::new(logger));
        self.logger.clone().unwrap()
    }

    pub fn get(&self) -> Option<Arc<AuditLogger>> {
        self.logger.clone()
    }

    pub fn log(&self, record: AuditRecord) -> AuditResult<AuditId> {
        if let Some(logger) = &self.logger {
            logger.log(record)
        } else {
            Err(AuditError::disabled())
        }
    }

    pub fn log_simple(
        &self,
        actor: ActorId,
        actor_type: ActorType,
        operation: impl Into<String>,
    ) -> AuditResult<AuditId> {
        if let Some(logger) = &self.logger {
            logger.log_simple(actor, actor_type, operation)
        } else {
            Err(AuditError::disabled())
        }
    }
}

impl Default for GlobalAuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for audit logging
pub trait AuditLoggerExt {
    fn audit_log(
        &self,
        actor: ActorId,
        actor_type: ActorType,
        operation: impl Into<String>,
    ) -> AuditResult<AuditId>;

    fn audit_log_with_event(
        &self,
        event: &malverde_events::event::Event,
    ) -> AuditResult<AuditId>;
}

impl AuditLoggerExt for AuditLogger {
    fn audit_log(
        &self,
        actor: ActorId,
        actor_type: ActorType,
        operation: impl Into<String>,
    ) -> AuditResult<AuditId> {
        self.log_simple(actor, actor_type, operation)
    }

    fn audit_log_with_event(
        &self,
        event: &malverde_events::event::Event,
    ) -> AuditResult<AuditId> {
        self.log_with_event(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use malverde_core::ids::ActorId;
    use malverde_core::states::ActorType;

    #[test]
    fn test_audit_record_creation() {
        let actor = ActorId::from_string("test-actor").unwrap();
        let record = AuditRecord::new(actor.clone(), ActorType::User, "test_operation");

        assert_eq!(record.actor, actor);
        assert_eq!(record.operation, "test_operation");
        assert_eq!(record.actor_type, ActorType::User);
    }

    #[test]
    fn test_audit_record_validation() {
        let actor = ActorId::from_string("test-actor").unwrap();
        let record = AuditRecord::new(actor, ActorType::User, "test_operation");

        assert!(record.validate().is_ok());

        // Invalid record (empty operation)
        let mut invalid = AuditRecord::new(actor, ActorType::User, "");
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_audit_record_builder() {
        let actor = ActorId::from_string("test-actor").unwrap();
        let project = ProjectId::from_string("test-project").unwrap();
        
        let record = AuditRecordBuilder::new(actor.clone(), ActorType::System, "test_op")
            .input_hash("abc123")
            .output_hash("def456")
            .confidence(Confidence::High)
            .error("Test error")
            .project(project.clone())
            .build();

        assert_eq!(record.actor, actor);
        assert_eq!(record.input_hash, Some("abc123".to_string()));
        assert_eq!(record.output_hash, Some("def456".to_string()));
        assert_eq!(record.confidence, Confidence::High);
        assert_eq!(record.errors.len(), 1);
        assert_eq!(record.project, Some(project));
    }

    #[test]
    fn test_audit_logger() {
        let logger = AuditLogger::new();
        let actor = ActorId::from_string("test-actor").unwrap();
        
        let record = AuditRecord::new(actor.clone(), ActorType::User, "test_op");
        let id = logger.log(record).unwrap();

        assert_eq!(logger.len(), 1);

        let all = logger.all();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, id);

        let by_actor = logger.by_actor(&actor);
        assert_eq!(by_actor.len(), 1);

        let by_op = logger.by_operation("test_op");
        assert_eq!(by_op.len(), 1);
    }

    #[test]
    fn test_audit_logger_disabled() {
        let mut logger = AuditLogger::new();
        logger.disable();

        let actor = ActorId::from_string("test-actor").unwrap();
        let record = AuditRecord::new(actor, ActorType::User, "test_op");
        
        let result = logger.log(record);
        assert!(result.is_err());
        assert_eq!(logger.len(), 0);
    }

    #[test]
    fn test_audit_stats() {
        let logger = AuditLogger::new();
        let actor1 = ActorId::from_string("actor1").unwrap();
        let actor2 = ActorId::from_string("actor2").unwrap();
        
        logger.log(AuditRecord::new(actor1.clone(), ActorType::User, "op1")).unwrap();
        logger.log(AuditRecord::new(actor1.clone(), ActorType::User, "op1")).unwrap();
        logger.log(AuditRecord::new(actor2.clone(), ActorType::System, "op2")).unwrap();
        
        let mut record_with_error = AuditRecord::new(actor2.clone(), ActorType::System, "op3");
        record_with_error.errors.push("Error".to_string());
        logger.log(record_with_error).unwrap();

        let stats = logger.stats();
        assert_eq!(stats.total_records, 4);
        assert_eq!(stats.get_actor_count(&actor1), 2);
        assert_eq!(stats.get_actor_count(&actor2), 2);
        assert_eq!(stats.error_count, 1);
    }

    #[test]
    fn test_audit_record_hash() {
        let actor = ActorId::from_string("test-actor").unwrap();
        let record = AuditRecord::new(actor, ActorType::User, "test_op")
            .with_input_hash("input123")
            .with_output_hash("output456");

        let hash1 = record.hash();
        let hash2 = record.hash();
        assert_eq!(hash1, hash2);
        assert!(!hash1.is_empty());
    }
}
