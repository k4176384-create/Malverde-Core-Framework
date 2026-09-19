//! Audit query system for filtering and retrieving audit records

use crate::audit::AuditRecord;
use crate::error::{AuditError, AuditResult};
use malverde_core::ids::{ActorId, AuditId, JobId, ProjectId};
use malverde_core::states::ActorType;
use malverde_core::timestamps::CreatedAt;
use malverde_core::trust::Confidence;
use std::collections::{HashMap, HashSet};

/// Audit query for filtering audit records
#[derive(Debug, Clone, Default)]
pub struct AuditQuery {
    pub actor: Option<ActorId>,
    pub actor_type: Option<ActorType>,
    pub operation: Option<String>,
    pub project: Option<ProjectId>,
    pub job_id: Option<JobId>,
    pub has_errors: Option<bool>,
    pub min_confidence: Option<Confidence>,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub event_id: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl AuditQuery {
    pub fn new() -> Self {
        AuditQuery::default()
    }

    pub fn with_actor(mut self, actor: ActorId) -> Self {
        self.actor = Some(actor);
        self
    }

    pub fn with_actor_type(mut self, actor_type: ActorType) -> Self {
        self.actor_type = Some(actor_type);
        self
    }

    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = Some(operation.into());
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

    pub fn with_errors(mut self, has_errors: bool) -> Self {
        self.has_errors = Some(has_errors);
        self
    }

    pub fn with_min_confidence(mut self, confidence: Confidence) -> Self {
        self.min_confidence = Some(confidence);
        self
    }

    pub fn with_time_range(
        mut self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }

    pub fn with_event_id(mut self, event_id: impl Into<String>) -> Self {
        self.event_id = Some(event_id.into());
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Check if a record matches this query
    pub fn matches(&self, record: &AuditRecord) -> bool {
        if let Some(ref actor) = self.actor {
            if &record.actor != actor {
                return false;
            }
        }

        if let Some(actor_type) = self.actor_type {
            if record.actor_type != actor_type {
                return false;
            }
        }

        if let Some(ref operation) = self.operation {
            if record.operation != *operation {
                return false;
            }
        }

        if let Some(ref project) = self.project {
            if record.project.as_ref() != Some(project) {
                return false;
            }
        }

        if let Some(ref job_id) = self.job_id {
            if record.job_id.as_ref() != Some(job_id) {
                return false;
            }
        }

        if let Some(has_errors) = self.has_errors {
            if record.has_errors() != has_errors {
                return false;
            }
        }

        if let Some(min_confidence) = self.min_confidence {
            if record.confidence < min_confidence {
                return false;
            }
        }

        if let Some(start_time) = self.start_time {
            if record.timestamp.0 < start_time {
                return false;
            }
        }

        if let Some(end_time) = self.end_time {
            if record.timestamp.0 > end_time {
                return false;
            }
        }

        if let Some(ref event_id) = self.event_id {
            if record.event_id.as_ref().map(|e| e.as_str()) != Some(event_id.as_str()) {
                return false;
            }
        }

        true
    }
}

/// Audit query result
#[derive(Debug, Clone)]
pub struct AuditQueryResult {
    pub records: Vec<AuditRecord>,
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
}

impl AuditQueryResult {
    pub fn new(records: Vec<AuditRecord>, total: usize, offset: usize, limit: usize) -> Self {
        AuditQueryResult {
            records,
            total,
            offset,
            limit,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }
}

/// Audit query executor
pub struct AuditQueryExecutor;

impl AuditQueryExecutor {
    /// Execute a query against a list of audit records
    pub fn execute_query(
        query: &AuditQuery,
        records: &[AuditRecord],
    ) -> AuditQueryResult {
        let matching: Vec<AuditRecord> = records
            .iter()
            .filter(|r| query.matches(r))
            .cloned()
            .collect();

        let total = matching.len();
        
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);
        
        let records = matching
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        AuditQueryResult::new(records, total, offset, limit)
    }

    /// Execute a query and return the first result
    pub fn execute_query_first(query: &AuditQuery, records: &[AuditRecord]) -> Option<AuditRecord> {
        records
            .iter()
            .find(|r| query.matches(r))
            .cloned()
    }

    /// Execute a query and return the count
    pub fn execute_query_count(query: &AuditQuery, records: &[AuditRecord]) -> usize {
        records
            .iter()
            .filter(|r| query.matches(r))
            .count()
    }

    /// Get unique values for a field
    pub fn get_unique_actors(records: &[AuditRecord]) -> HashSet<ActorId> {
        records
            .iter()
            .map(|r| r.actor.clone())
            .collect()
    }

    pub fn get_unique_operations(records: &[AuditRecord]) -> HashSet<String> {
        records
            .iter()
            .map(|r| r.operation.clone())
            .collect()
    }

    pub fn get_unique_projects(records: &[AuditRecord]) -> HashSet<ProjectId> {
        records
            .iter()
            .filter_map(|r| r.project.clone())
            .collect()
    }

    /// Get statistics for a query result
    pub fn get_stats(result: &AuditQueryResult) -> AuditQueryStats {
        let mut by_actor: HashMap<ActorId, usize> = HashMap::new();
        let mut by_operation: HashMap<String, usize> = HashMap::new();
        let mut error_count = 0;
        let mut total_confidence: u32 = 0;
        let mut confidence_count = 0;

        for record in &result.records {
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

        AuditQueryStats {
            total_records: result.total,
            returned_records: result.records.len(),
            by_actor,
            by_operation,
            error_count,
            avg_confidence,
        }
    }
}

/// Statistics for a query result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditQueryStats {
    pub total_records: usize,
    pub returned_records: usize,
    pub by_actor: HashMap<ActorId, usize>,
    pub by_operation: HashMap<String, usize>,
    pub error_count: usize,
    pub avg_confidence: Confidence,
}

impl AuditQueryStats {
    pub fn error_rate(&self) -> f64 {
        if self.total_records == 0 {
            0.0
        } else {
            self.error_count as f64 / self.total_records as f64
        }
    }
}

/// Audit query builder for fluent query construction
pub struct AuditQueryBuilder {
    query: AuditQuery,
}

impl AuditQueryBuilder {
    pub fn new() -> Self {
        AuditQueryBuilder {
            query: AuditQuery::new(),
        }
    }

    pub fn actor(mut self, actor: ActorId) -> Self {
        self.query.actor = Some(actor);
        self
    }

    pub fn actor_type(mut self, actor_type: ActorType) -> Self {
        self.query.actor_type = Some(actor_type);
        self
    }

    pub fn operation(mut self, operation: impl Into<String>) -> Self {
        self.query.operation = Some(operation.into());
        self
    }

    pub fn project(mut self, project: ProjectId) -> Self {
        self.query.project = Some(project);
        self
    }

    pub fn job_id(mut self, job_id: JobId) -> Self {
        self.query.job_id = Some(job_id);
        self
    }

    pub fn has_errors(mut self, has_errors: bool) -> Self {
        self.query.has_errors = Some(has_errors);
        self
    }

    pub fn min_confidence(mut self, confidence: Confidence) -> Self {
        self.query.min_confidence = Some(confidence);
        self
    }

    pub fn time_range(
        mut self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        self.query.start_time = Some(start);
        self.query.end_time = Some(end);
        self
    }

    pub fn event_id(mut self, event_id: impl Into<String>) -> Self {
        self.query.event_id = Some(event_id.into());
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.query.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: usize) -> Self {
        self.query.offset = Some(offset);
        self
    }

    pub fn build(self) -> AuditQuery {
        self.query
    }
}

/// Trait for queryable audit storage
pub trait AuditQueryable {
    fn query(&self, query: &AuditQuery) -> AuditResult<AuditQueryResult>;
    fn query_first(&self, query: &AuditQuery) -> AuditResult<Option<AuditRecord>>;
    fn query_count(&self, query: &AuditQuery) -> AuditResult<usize>;
}

impl AuditQueryable for Vec<AuditRecord> {
    fn query(&self, query: &AuditQuery) -> AuditResult<AuditQueryResult> {
        let result = AuditQueryExecutor::execute_query(query, self);
        Ok(result)
    }

    fn query_first(&self, query: &AuditQuery) -> AuditResult<Option<AuditRecord>> {
        let result = AuditQueryExecutor::execute_query_first(query, self);
        Ok(result)
    }

    fn query_count(&self, query: &AuditQuery) -> AuditResult<usize> {
        let count = AuditQueryExecutor::execute_query_count(query, self);
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditLogger;
    use malverde_core::ids::ActorId;
    use malverde_core::states::ActorType;

    #[test]
    fn test_audit_query_matching() {
        let actor = ActorId::from_string("test-actor").unwrap();
        let project = ProjectId::from_string("test-project").unwrap();
        
        let record1 = AuditRecord::new(actor.clone(), ActorType::User, "op1")
            .with_project(project.clone());
        let record2 = AuditRecord::new(actor.clone(), ActorType::System, "op2")
            .with_project(project.clone());
        let record3 = AuditRecord::new(ActorId::from_string("other").unwrap(), ActorType::User, "op1");

        let records = vec![record1, record2, record3];

        // Query by actor
        let query = AuditQuery::new().with_actor(actor.clone());
        let result = AuditQueryExecutor::execute_query(&query, &records);
        assert_eq!(result.records.len(), 2);

        // Query by operation
        let query = AuditQuery::new().with_operation("op1");
        let result = AuditQueryExecutor::execute_query(&query, &records);
        assert_eq!(result.records.len(), 2);

        // Query by project
        let query = AuditQuery::new().with_project(project);
        let result = AuditQueryExecutor::execute_query(&query, &records);
        assert_eq!(result.records.len(), 2);
    }

    #[test]
    fn test_audit_query_with_errors() {
        let actor = ActorId::from_string("test-actor").unwrap();
        
        let mut record1 = AuditRecord::new(actor.clone(), ActorType::User, "op1");
        record1.errors.push("Error 1".to_string());

        let record2 = AuditRecord::new(actor.clone(), ActorType::User, "op2");

        let records = vec![record1, record2];

        // Query for records with errors
        let query = AuditQuery::new().with_errors(true);
        let result = AuditQueryExecutor::execute_query(&query, &records);
        assert_eq!(result.records.len(), 1);

        // Query for records without errors
        let query = AuditQuery::new().with_errors(false);
        let result = AuditQueryExecutor::execute_query(&query, &records);
        assert_eq!(result.records.len(), 1);
    }

    #[test]
    fn test_audit_query_time_range() {
        use chrono::{Utc, Duration};
        
        let actor = ActorId::from_string("test-actor").unwrap();
        let now = Utc::now();
        
        let record1 = AuditRecord {
            id: AuditId::from_string("1").unwrap(),
            event_id: None,
            timestamp: CreatedAt::from(now - Duration::hours(2)),
            actor: actor.clone(),
            actor_type: ActorType::User,
            operation: "op1".to_string(),
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
        };

        let record2 = AuditRecord {
            id: AuditId::from_string("2").unwrap(),
            event_id: None,
            timestamp: CreatedAt::from(now - Duration::hours(1)),
            actor: actor.clone(),
            actor_type: ActorType::User,
            operation: "op2".to_string(),
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
        };

        let records = vec![record1, record2];

        // Query for records in the last hour
        let query = AuditQuery::new().with_time_range(now - Duration::hours(1), now);
        let result = AuditQueryExecutor::execute_query(&query, &records);
        assert_eq!(result.records.len(), 1);
    }

    #[test]
    fn test_audit_query_limit_offset() {
        let actor = ActorId::from_string("test-actor").unwrap();
        
        let records: Vec<AuditRecord> = (0..10)
            .map(|i| AuditRecord::new(actor.clone(), ActorType::User, format!("op{}", i)))
            .collect();

        // Query with limit
        let query = AuditQuery::new().with_limit(3);
        let result = AuditQueryExecutor::execute_query(&query, &records);
        assert_eq!(result.records.len(), 3);
        assert_eq!(result.total, 10);

        // Query with offset
        let query = AuditQuery::new().with_offset(5);
        let result = AuditQueryExecutor::execute_query(&query, &records);
        assert_eq!(result.records.len(), 5);
        assert_eq!(result.offset, 5);

        // Query with both
        let query = AuditQuery::new().with_limit(2).with_offset(3);
        let result = AuditQueryExecutor::execute_query(&query, &records);
        assert_eq!(result.records.len(), 2);
        assert_eq!(result.total, 10);
    }

    #[test]
    fn test_audit_query_builder() {
        let actor = ActorId::from_string("test-actor").unwrap();
        let project = ProjectId::from_string("test-project").unwrap();
        
        let query = AuditQueryBuilder::new()
            .actor(actor.clone())
            .actor_type(ActorType::User)
            .operation("test_op")
            .project(project.clone())
            .has_errors(false)
            .min_confidence(Confidence::Medium)
            .limit(10)
            .offset(0)
            .build();

        assert_eq!(query.actor, Some(actor));
        assert_eq!(query.actor_type, Some(ActorType::User));
        assert_eq!(query.operation, Some("test_op".to_string()));
        assert_eq!(query.project, Some(project));
        assert_eq!(query.has_errors, Some(false));
        assert_eq!(query.min_confidence, Some(Confidence::Medium));
        assert_eq!(query.limit, Some(10));
        assert_eq!(query.offset, Some(0));
    }

    #[test]
    fn test_audit_query_stats() {
        let actor = ActorId::from_string("test-actor").unwrap();
        
        let records: Vec<AuditRecord> = (0..5)
            .map(|i| {
                let mut record = AuditRecord::new(actor.clone(), ActorType::User, "op1");
                if i % 2 == 0 {
                    record.errors.push("Error".to_string());
                }
                record
            })
            .collect();

        let query = AuditQuery::new();
        let result = AuditQueryExecutor::execute_query(&query, &records);
        let stats = AuditQueryExecutor::get_stats(&result);

        assert_eq!(stats.total_records, 5);
        assert_eq!(stats.returned_records, 5);
        assert_eq!(stats.error_count, 3); // 0, 2, 4
    }
}
