//! Job model and state management

use malverde_core::ids::{ActorId, CheckpointId, JobId, OperationId, ProjectId};
use malverde_core::models::Operation;
use malverde_core::states::JobState;
use malverde_core::timestamps::{CreatedAt, UpdatedAt};
use malverde_core::trust::{Confidence, Evidence};
use malverde_core::{MalverdeError, MalverdeResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use uuid::Uuid;

/// Job configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobConfig {
    pub id: JobId,
    pub job_type: String,
    pub description: Option<String>,
    pub input: serde_json::Value,
    pub max_retries: u32,
    pub timeout: Option<Duration>,
    pub retry_delay: Duration,
    pub metadata: serde_json::Value,
}

impl JobConfig {
    pub fn new(job_type: impl Into<String>, input: serde_json::Value) -> Self {
        JobConfig {
            id: JobId::from_string(&Uuid::new_v4().to_string()).unwrap(),
            job_type: job_type.into(),
            description: None,
            input,
            max_retries: 3,
            timeout: None,
            retry_delay: Duration::from_secs(1),
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn with_retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = delay;
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

impl Default for JobConfig {
    fn default() -> Self {
        JobConfig::new("default", serde_json::Value::Null)
    }
}

/// Job instance represents a running or pending job
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobInstance {
    pub id: JobId,
    pub config: JobConfig,
    pub state: JobState,
    pub actor: ActorId,
    pub project: Option<ProjectId>,
    pub operation: Option<OperationId>,
    pub output: Option<serde_json::Value>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub progress: u8, // 0-100
    pub checkpoint: Option<Checkpoint>,
    pub metadata: serde_json::Value,
    pub created_at: CreatedAt,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub retry_count: u32,
    pub last_error: Option<String>,
}

impl JobInstance {
    pub fn new(config: JobConfig, actor: ActorId) -> Self {
        JobInstance {
            id: config.id.clone(),
            config,
            state: JobState::Pending,
            actor,
            project: None,
            operation: None,
            output: None,
            errors: Vec::new(),
            warnings: Vec::new(),
            progress: 0,
            checkpoint: None,
            metadata: serde_json::Value::Null,
            created_at: CreatedAt::now(),
            started_at: None,
            completed_at: None,
            retry_count: 0,
            last_error: None,
        }
    }

    pub fn with_project(mut self, project: ProjectId) -> Self {
        self.project = Some(project);
        self
    }

    pub fn with_operation(mut self, operation: OperationId) -> Self {
        self.operation = Some(operation);
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Start the job
    pub fn start(&mut self) {
        self.state = JobState::Running;
        self.started_at = Some(chrono::Utc::now());
        self.retry_count = 0;
    }

    /// Update progress (0-100)
    pub fn set_progress(&mut self, progress: u8) -> Result<(), JobError> {
        if progress > 100 {
            return Err(JobError::InvalidProgress);
        }
        self.progress = progress;
        Ok(())
    }

    /// Increment progress
    pub fn increment_progress(&mut self, amount: u8) -> Result<(), JobError> {
        let new_progress = self.progress.saturating_add(amount);
        if new_progress > 100 {
            self.progress = 100;
        } else {
            self.progress = new_progress;
        }
        Ok(())
    }

    /// Add an error
    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
        self.last_error = Some(error.into());
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Set output
    pub fn set_output(&mut self, output: serde_json::Value) {
        self.output = Some(output);
    }

    /// Mark as completed
    pub fn mark_completed(&mut self) {
        self.state = JobState::Completed;
        self.progress = 100;
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Mark as failed
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.state = JobState::Failed;
        self.add_error(error);
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Mark as cancelled
    pub fn mark_cancelled(&mut self) {
        self.state = JobState::Cancelled;
        self.completed_at = Some(chrono::Utc::now());
    }

    /// Mark as paused
    pub fn mark_paused(&mut self) {
        self.state = JobState::Paused;
    }

    /// Mark as recovering
    pub fn mark_recovering(&mut self) {
        self.state = JobState::Recovering;
    }

    /// Check if job can be cancelled
    pub fn can_cancel(&self) -> bool {
        self.state.can_cancel()
    }

    /// Check if job can be paused
    pub fn can_pause(&self) -> bool {
        self.state.can_pause()
    }

    /// Check if job can be resumed
    pub fn can_resume(&self) -> bool {
        self.state.can_resume()
    }

    /// Check if job can be retried
    pub fn can_retry(&self) -> bool {
        self.state.can_retry()
    }

    /// Check if job is terminal (cannot transition further)
    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    /// Check if job is running
    pub fn is_running(&self) -> bool {
        self.state.is_running()
    }

    /// Get duration if available
    pub fn duration(&self) -> Option<Duration> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => Some(end - start),
            (Some(start), None) => Some(chrono::Utc::now() - start),
            _ => None,
        }
    }

    /// Get elapsed time since start
    pub fn elapsed(&self) -> Option<Duration> {
        self.started_at.map(|start| chrono::Utc::now() - start)
    }

    /// Check if timeout has occurred
    pub fn is_timed_out(&self) -> bool {
        if let Some(timeout) = self.config.timeout {
            if let Some(elapsed) = self.elapsed() {
                return elapsed >= timeout;
            }
        }
        false
    }

    /// Create a checkpoint
    pub fn create_checkpoint(&mut self, metadata: serde_json::Value) -> Checkpoint {
        let checkpoint = Checkpoint::new(self.id.clone(), metadata);
        self.checkpoint = Some(checkpoint.clone());
        checkpoint
    }

    /// Update checkpoint
    pub fn update_checkpoint(&mut self, checkpoint: Checkpoint) {
        self.checkpoint = Some(checkpoint);
    }

    /// Clear checkpoint
    pub fn clear_checkpoint(&mut self) {
        self.checkpoint = None;
    }

    /// Get the latest checkpoint
    pub fn get_checkpoint(&self) -> Option<&Checkpoint> {
        self.checkpoint.as_ref()
    }

    /// Increment retry count
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    /// Check if max retries exceeded
    pub fn retries_exceeded(&self) -> bool {
        self.retry_count >= self.config.max_retries
    }

    /// Get job summary
    pub fn summary(&self) -> JobSummary {
        JobSummary {
            id: self.id.clone(),
            job_type: self.config.job_type.clone(),
            state: self.state,
            progress: self.progress,
            created_at: self.created_at,
            started_at: self.started_at,
            completed_at: self.completed_at,
            duration: self.duration(),
            retry_count: self.retry_count,
            error_count: self.errors.len(),
            warning_count: self.warnings.len(),
        }
    }
}

/// Job summary for quick status checks
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobSummary {
    pub id: JobId,
    pub job_type: String,
    pub state: JobState,
    pub progress: u8,
    pub created_at: CreatedAt,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub duration: Option<Duration>,
    pub retry_count: u32,
    pub error_count: usize,
    pub warning_count: usize,
}

impl JobSummary {
    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    pub fn is_successful(&self) -> bool {
        self.state == JobState::Completed
    }

    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }
}

/// Job builder for fluent construction
pub struct JobBuilder {
    config: JobConfig,
    actor: ActorId,
    project: Option<ProjectId>,
    operation: Option<OperationId>,
    metadata: serde_json::Value,
}

impl JobBuilder {
    pub fn new(job_type: impl Into<String>, actor: ActorId) -> Self {
        JobBuilder {
            config: JobConfig::new(job_type, serde_json::Value::Null),
            actor,
            project: None,
            operation: None,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_input(mut self, input: serde_json::Value) -> Self {
        self.config.input = input;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.config.description = Some(description.into());
        self
    }

    pub fn with_project(mut self, project: ProjectId) -> Self {
        self.project = Some(project);
        self
    }

    pub fn with_operation(mut self, operation: OperationId) -> Self {
        self.operation = Some(operation);
        self
    }

    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.config.max_retries = retries;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = Some(timeout);
        self
    }

    pub fn with_retry_delay(mut self, delay: Duration) -> Self {
        self.config.retry_delay = delay;
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn build(self) -> JobInstance {
        JobInstance::new(self.config, self.actor)
            .with_project(self.project.clone())
            .with_operation(self.operation.clone())
            .with_metadata(self.metadata)
    }
}

/// Job registry for managing jobs
#[derive(Debug, Clone)]
pub struct JobRegistry {
    jobs: Arc<RwLock<HashMap<JobId, JobInstance>>>,
}

impl JobRegistry {
    pub fn new() -> Self {
        JobRegistry {
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a job
    pub fn register(&self, job: JobInstance) -> JobId {
        let mut jobs = self.jobs.write().unwrap();
        jobs.insert(job.id.clone(), job);
        job.id.clone()
    }

    /// Unregister a job
    pub fn unregister(&self, id: &JobId) -> Option<JobInstance> {
        let mut jobs = self.jobs.write().unwrap();
        jobs.remove(id)
    }

    /// Get a job by ID
    pub fn get(&self, id: &JobId) -> Option<JobInstance> {
        let jobs = self.jobs.read().unwrap();
        jobs.get(id).cloned()
    }

    /// Get all jobs
    pub fn all(&self) -> Vec<JobInstance> {
        let jobs = self.jobs.read().unwrap();
        jobs.values().cloned().collect()
    }

    /// Get jobs by state
    pub fn get_by_state(&self, state: JobState) -> Vec<JobInstance> {
        let jobs = self.jobs.read().unwrap();
        jobs.values()
            .filter(|j| j.state == state)
            .cloned()
            .collect()
    }

    /// Get running jobs
    pub fn get_running(&self) -> Vec<JobInstance> {
        self.get_by_state(JobState::Running)
    }

    /// Get pending jobs
    pub fn get_pending(&self) -> Vec<JobInstance> {
        self.get_by_state(JobState::Pending)
    }

    /// Get completed jobs
    pub fn get_completed(&self) -> Vec<JobInstance> {
        self.get_by_state(JobState::Completed)
    }

    /// Get failed jobs
    pub fn get_failed(&self) -> Vec<JobInstance> {
        self.get_by_state(JobState::Failed)
    }

    /// Get jobs by actor
    pub fn get_by_actor(&self, actor: &ActorId) -> Vec<JobInstance> {
        let jobs = self.jobs.read().unwrap();
        jobs.values()
            .filter(|j| &j.actor == actor)
            .cloned()
            .collect()
    }

    /// Get jobs by project
    pub fn get_by_project(&self, project: &ProjectId) -> Vec<JobInstance> {
        let jobs = self.jobs.read().unwrap();
        jobs.values()
            .filter(|j| j.project.as_ref() == Some(project))
            .cloned()
            .collect()
    }

    /// Count jobs by state
    pub fn count_by_state(&self) -> HashMap<JobState, usize> {
        let jobs = self.jobs.read().unwrap();
        let mut counts = HashMap::new();
        for job in jobs.values() {
            *counts.entry(job.state).or_insert(0) += 1;
        }
        counts
    }

    /// Clear all jobs
    pub fn clear(&self) {
        let mut jobs = self.jobs.write().unwrap();
        jobs.clear();
    }

    /// Get job count
    pub fn len(&self) -> usize {
        let jobs = self.jobs.read().unwrap();
        jobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for JobRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use malverde_core::ids::ActorId;

    #[test]
    fn test_job_creation() {
        let actor = ActorId::from_string("test-actor").unwrap();
        let config = JobConfig::new("test-job", serde_json::json!({"input": "data"}));
        let job = JobInstance::new(config, actor.clone());

        assert_eq!(job.state, JobState::Pending);
        assert_eq!(job.actor, actor);
        assert_eq!(job.progress, 0);
    }

    #[test]
    fn test_job_state_transitions() {
        let actor = ActorId::from_string("test-actor").unwrap();
        let config = JobConfig::new("test-job", serde_json::Value::Null);
        let mut job = JobInstance::new(config, actor);

        // Start
        job.start();
        assert_eq!(job.state, JobState::Running);
        assert!(job.started_at.is_some());

        // Increment progress
        job.increment_progress(25).unwrap();
        assert_eq!(job.progress, 25);

        // Mark completed
        job.mark_completed();
        assert_eq!(job.state, JobState::Completed);
        assert_eq!(job.progress, 100);
        assert!(job.completed_at.is_some());
    }

    #[test]
    fn test_job_failure() {
        let actor = ActorId::from_string("test-actor").unwrap();
        let config = JobConfig::new("test-job", serde_json::Value::Null);
        let mut job = JobInstance::new(config, actor);

        job.start();
        job.add_error("Test error");
        job.mark_failed("Final error");

        assert_eq!(job.state, JobState::Failed);
        assert_eq!(job.errors.len(), 2); // One from add_error, one from mark_failed
        assert!(job.last_error.is_some());
    }

    #[test]
    fn test_job_builder() {
        let actor = ActorId::from_string("test-actor").unwrap();
        let project = ProjectId::from_string("test-project").unwrap();
        
        let job = JobBuilder::new("test-job", actor.clone())
            .with_input(serde_json::json!({"key": "value"}))
            .with_description("Test job")
            .with_project(project.clone())
            .with_max_retries(5)
            .with_timeout(Duration::from_secs(30))
            .build();

        assert_eq!(job.config.job_type, "test-job");
        assert_eq!(job.actor, actor);
        assert_eq!(job.project, Some(project));
        assert_eq!(job.config.max_retries, 5);
        assert_eq!(job.config.timeout, Some(Duration::from_secs(30)));
    }

    #[test]
    fn test_job_summary() {
        let actor = ActorId::from_string("test-actor").unwrap();
        let config = JobConfig::new("test-job", serde_json::Value::Null);
        let mut job = JobInstance::new(config, actor);

        job.start();
        job.increment_progress(50).unwrap();
        job.add_error("Error 1");
        job.add_warning("Warning 1");

        let summary = job.summary();
        assert_eq!(summary.progress, 50);
        assert_eq!(summary.error_count, 1);
        assert_eq!(summary.warning_count, 1);
    }

    #[test]
    fn test_job_registry() {
        let registry = JobRegistry::new();
        let actor = ActorId::from_string("test-actor").unwrap();
        
        let job1 = JobInstance::new(JobConfig::new("job1", serde_json::Value::Null), actor.clone());
        let job2 = JobInstance::new(JobConfig::new("job2", serde_json::Value::Null), actor);

        let id1 = registry.register(job1.clone());
        let id2 = registry.register(job2.clone());

        assert_eq!(registry.len(), 2);
        assert!(registry.get(&id1).is_some());
        assert!(registry.get(&id2).is_some());

        // Get by state
        let pending = registry.get_pending();
        assert_eq!(pending.len(), 2);

        // Unregister
        assert!(registry.unregister(&id1).is_some());
        assert_eq!(registry.len(), 1);
    }
}
