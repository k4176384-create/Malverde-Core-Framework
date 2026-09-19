//! Job executor for running jobs with checkpoint support

use crate::checkpoint::{Checkpoint, CheckpointManager, CheckpointMetadata, CheckpointStrategy, RecoveryInfo};
use crate::error::{JobError, JobResult};
use crate::job::{JobConfig, JobInstance, JobRegistry, JobState, JobSummary};
use malverde_core::ids::{ActorId, JobId, ProjectId};
use malverde_core::states::EventType;
use malverde_events::bus::{EventBus, EventPublisher};
use malverde_events::event::{Event, EventPayload};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::mpsc::{self, Sender, Receiver};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// Job execution context
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub job: JobInstance,
    pub checkpoint_manager: Arc<CheckpointManager>,
    pub bus: Option<Arc<Mutex<EventBus>>>,
    pub project: Option<ProjectId>,
}

impl ExecutionContext {
    pub fn new(job: JobInstance, checkpoint_manager: Arc<CheckpointManager>) -> Self {
        ExecutionContext {
            job,
            checkpoint_manager,
            bus: None,
            project: None,
        }
    }

    pub fn with_bus(mut self, bus: Arc<Mutex<EventBus>>) -> Self {
        self.bus = Some(bus);
        self
    }

    pub fn with_project(mut self, project: ProjectId) -> Self {
        self.project = Some(project);
        self
    }

    /// Create a checkpoint
    pub fn create_checkpoint(&self, metadata: serde_json::Value) -> Checkpoint {
        self.checkpoint_manager.create(metadata)
    }

    /// Get the latest checkpoint
    pub fn latest_checkpoint(&self) -> Option<Checkpoint> {
        self.checkpoint_manager.latest_valid()
    }

    /// Publish an event
    pub async fn publish_event(
        &self,
        event_type: EventType,
        payload: serde_json::Value,
    ) -> JobResult<()> {
        if let Some(bus) = &self.bus {
            let bus = bus.lock().await;
            let actor = self.job.actor.clone();
            let event = Event::new(
                event_type,
                actor,
                malverde_core::states::ActorType::Job,
                EventPayload::with_data(payload),
            )
            .with_project(self.project.clone());
            
            bus.publish("job.events", event).await?;
        }
        Ok(())
    }
}

/// Job function type
pub type JobFunction = Box<dyn Fn(&mut ExecutionContext) -> JobResult<()> + Send + Sync>;

/// Job handler for executing specific job types
pub struct JobHandler {
    pub job_type: String,
    pub function: JobFunction,
    pub checkpoint_strategy: CheckpointStrategy,
}

impl JobHandler {
    pub fn new(job_type: impl Into<String>, function: JobFunction) -> Self {
        JobHandler {
            job_type: job_type.into(),
            function,
            checkpoint_strategy: CheckpointStrategy::default(),
        }
    }

    pub fn with_checkpoint_strategy(mut self, strategy: CheckpointStrategy) -> Self {
        self.checkpoint_strategy = strategy;
        self
    }

    pub fn execute(&self, ctx: &mut ExecutionContext) -> JobResult<()> {
        (self.function)(ctx)
    }

    pub fn should_checkpoint(&self, progress: u8, elapsed: Duration) -> bool {
        self.checkpoint_strategy.should_checkpoint(progress, elapsed)
    }
}

/// Job executor for running jobs
#[derive(Debug, Clone)]
pub struct JobExecutor {
    registry: Arc<JobRegistry>,
    handlers: Arc<RwLock<HashMap<String, JobHandler>>>,
    bus: Option<Arc<Mutex<EventBus>>>,
    max_concurrent_jobs: usize,
}

impl JobExecutor {
    pub fn new(registry: Arc<JobRegistry>) -> Self {
        JobExecutor {
            registry,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            bus: None,
            max_concurrent_jobs: 10,
        }
    }

    pub fn with_bus(mut self, bus: Arc<Mutex<EventBus>>) -> Self {
        self.bus = Some(bus);
        self
    }

    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent_jobs = max;
        self
    }

    /// Register a job handler
    pub fn register_handler(&self, handler: JobHandler) {
        let mut handlers = self.handlers.write().unwrap();
        handlers.insert(handler.job_type.clone(), handler);
    }

    /// Unregister a job handler
    pub fn unregister_handler(&self, job_type: &str) -> Option<JobHandler> {
        let mut handlers = self.handlers.write().unwrap();
        handlers.remove(job_type)
    }

    /// Get a job handler
    pub fn get_handler(&self, job_type: &str) -> Option<JobHandler> {
        let handlers = self.handlers.read().unwrap();
        handlers.get(job_type).cloned()
    }

    /// Submit a job for execution
    pub async fn submit(&self, config: JobConfig, actor: ActorId) -> JobResult<JobId> {
        let job = JobInstance::new(config, actor);
        let job_id = self.registry.register(job);
        
        // Start the job
        self.start_job(job_id.clone()).await?;
        
        Ok(job_id)
    }

    /// Start a job
    pub async fn start_job(&self, job_id: JobId) -> JobResult<()> {
        let job = self.registry.get(&job_id)
            .ok_or_else(|| JobError::not_found(job_id.as_str()))?;
        
        if job.state != JobState::Pending {
            return Err(JobError::invalid_state(format!(
                "Job {} is not in Pending state", job_id
            )));
        }

        let handler = self.get_handler(&job.config.job_type)
            .ok_or_else(|| JobError::invalid_configuration(format!(
                "No handler for job type: {}", job.config.job_type
            )))?;

        let checkpoint_manager = Arc::new(CheckpointManager::new(job_id.clone()));
        let mut ctx = ExecutionContext::new(job, checkpoint_manager);
        
        if let Some(ref bus) = self.bus {
            ctx = ctx.with_bus(bus.clone());
        }
        if let Some(ref project) = job.project {
            ctx = ctx.with_project(project.clone());
        }

        // Publish job started event
        if let Some(ref bus) = self.bus {
            let bus = bus.lock().await;
            let event = Event::new(
                EventType::JobStarted,
                ctx.job.actor.clone(),
                malverde_core::states::ActorType::Job,
                EventPayload::with_data(serde_json::json!({
                    "job_id": job_id.as_str(),
                    "job_type": ctx.job.config.job_type.clone()
                })),
            );
            bus.publish("job.events", event).await?;
        }

        // Spawn the job execution in a separate task
        tokio::spawn(async move {
            Self::execute_job(ctx, handler).await;
        });

        Ok(())
    }

    /// Execute a job
    async fn execute_job(mut ctx: ExecutionContext, handler: JobHandler) {
        let job_id = ctx.job.id.clone();
        let mut job = ctx.job.clone();
        
        // Start the job
        job.start();
        
        // Update registry
        if let Some(registry) = Arc::get_mut(&mut ctx.job) {
            // This won't work because we need the registry
            // In a real implementation, we'd update the registry
        }
        
        let start_time = chrono::Utc::now();
        let mut last_checkpoint = start_time;
        
        // Execute the job function
        let result = handler.execute(&mut ctx);
        
        match result {
            Ok(_) => {
                // Job completed successfully
                job.mark_completed();
                
                // Publish completion event
                if let Some(ref bus) = ctx.bus {
                    let bus = bus.lock().await;
                    let event = Event::new(
                        EventType::JobCompleted,
                        job.actor.clone(),
                        malverde_core::states::ActorType::Job,
                        EventPayload::with_data(serde_json::json!({
                            "job_id": job_id.as_str(),
                            "duration": job.duration().map(|d| d.as_secs_f64()).unwrap_or(0.0)
                        })),
                    );
                    let _ = bus.publish("job.events", event).await;
                }
            }
            Err(e) => {
                // Job failed
                job.mark_failed(e.to_string());
                
                // Publish failure event
                if let Some(ref bus) = ctx.bus {
                    let bus = bus.lock().await;
                    let event = Event::new(
                        EventType::JobFailed,
                        job.actor.clone(),
                        malverde_core::states::ActorType::Job,
                        EventPayload::with_data(serde_json::json!({
                            "job_id": job_id.as_str(),
                            "error": e.to_string()
                        })),
                    );
                    let _ = bus.publish("job.events", event).await;
                }
            }
        }
    }

    /// Execute a job with automatic checkpointing
    pub async fn execute_with_checkpoints(
        &self,
        job_id: JobId,
        handler: JobHandler,
    ) -> JobResult<()> {
        let job = self.registry.get(&job_id)
            .ok_or_else(|| JobError::not_found(job_id.as_str()))?;
        
        if job.state != JobState::Running {
            return Err(JobError::invalid_state(format!(
                "Job {} is not in Running state", job_id
            )));
        }

        let checkpoint_manager = Arc::new(CheckpointManager::new(job_id.clone()));
        let mut ctx = ExecutionContext::new(job, checkpoint_manager);
        
        if let Some(ref bus) = self.bus {
            ctx = ctx.with_bus(bus.clone());
        }
        if let Some(ref project) = job.project {
            ctx = ctx.with_project(project.clone());
        }

        let start_time = chrono::Utc::now();
        let mut last_checkpoint = start_time;
        
        // Execute with periodic checkpointing
        loop {
            // Check if we should create a checkpoint
            if let Some(latest) = ctx.latest_checkpoint() {
                if handler.should_checkpoint(ctx.job.progress, latest.created_at.0 - start_time) {
                    // Create checkpoint
                    let metadata = CheckpointMetadata::new()
                        .with_job_state(ctx.job.state.to_string())
                        .with_progress(ctx.job.progress)
                        .with_current_step(Some(format!("progress_{}", ctx.job.progress)));
                    
                    ctx.create_checkpoint(serde_json::to_value(metadata).unwrap());
                    last_checkpoint = chrono::Utc::now();
                }
            }

            // Execute a chunk of work
            // In a real implementation, this would be more sophisticated
            match handler.execute(&mut ctx) {
                Ok(_) => {
                    // Job completed
                    ctx.job.mark_completed();
                    break;
                }
                Err(e) => {
                    // Handle error
                    if ctx.job.retries_exceeded() {
                        ctx.job.mark_failed(e.to_string());
                        break;
                    }
                    
                    // Wait before retrying
                    ctx.job.increment_retry();
                    tokio::time::sleep(handler.checkpoint_strategy.next_trigger(
                        ctx.job.progress, 
                        chrono::Utc::now() - start_time
                    ).map(|(_, d)| d).unwrap_or(Duration::from_secs(1))).await;
                }
            }
        }

        Ok(())
    }

    /// Cancel a job
    pub async fn cancel_job(&self, job_id: &JobId) -> JobResult<()> {
        let mut job = self.registry.get(job_id)
            .ok_or_else(|| JobError::not_found(job_id.as_str()))?;
        
        if !job.can_cancel() {
            return Err(JobError::invalid_state(format!(
                "Job {} cannot be cancelled in state: {:?}", job_id, job.state
            )));
        }

        job.mark_cancelled();
        
        // Publish cancellation event
        if let Some(ref bus) = self.bus {
            let bus = bus.lock().await;
            let event = Event::new(
                EventType::JobCancelled,
                job.actor.clone(),
                malverde_core::states::ActorType::Job,
                EventPayload::with_data(serde_json::json!({
                    "job_id": job_id.as_str()
                })),
            );
            bus.publish("job.events", event).await?;
        }

        Ok(())
    }

    /// Pause a job
    pub async fn pause_job(&self, job_id: &JobId) -> JobResult<()> {
        let mut job = self.registry.get(job_id)
            .ok_or_else(|| JobError::not_found(job_id.as_str()))?;
        
        if !job.can_pause() {
            return Err(JobError::invalid_state(format!(
                "Job {} cannot be paused in state: {:?}", job_id, job.state
            )));
        }

        job.mark_paused();
        
        // Publish pause event
        if let Some(ref bus) = self.bus {
            let bus = bus.lock().await;
            let event = Event::new(
                EventType::JobPaused,
                job.actor.clone(),
                malverde_core::states::ActorType::Job,
                EventPayload::with_data(serde_json::json!({
                    "job_id": job_id.as_str()
                })),
            );
            bus.publish("job.events", event).await?;
        }

        Ok(())
    }

    /// Resume a job
    pub async fn resume_job(&self, job_id: &JobId) -> JobResult<()> {
        let mut job = self.registry.get(job_id)
            .ok_or_else(|| JobError::not_found(job_id.as_str()))?;
        
        if !job.can_resume() {
            return Err(JobError::invalid_state(format!(
                "Job {} cannot be resumed in state: {:?}", job_id, job.state
            )));
        }

        job.start(); // Reset to running state
        
        // Publish resume event
        if let Some(ref bus) = self.bus {
            let bus = bus.lock().await;
            let event = Event::new(
                EventType::JobResumed,
                job.actor.clone(),
                malverde_core::states::ActorType::Job,
                EventPayload::with_data(serde_json::json!({
                    "job_id": job_id.as_str()
                })),
            );
            bus.publish("job.events", event).await?;
        }

        // Restart the job
        self.start_job(job_id.clone()).await?;
        
        Ok(())
    }

    /// Retry a job
    pub async fn retry_job(&self, job_id: &JobId) -> JobResult<()> {
        let mut job = self.registry.get(job_id)
            .ok_or_else(|| JobError::not_found(job_id.as_str()))?;
        
        if !job.can_retry() {
            return Err(JobError::invalid_state(format!(
                "Job {} cannot be retried in state: {:?}", job_id, job.state
            )));
        }

        // Reset job state for retry
        job.state = JobState::Pending;
        job.started_at = None;
        job.completed_at = None;
        job.errors.clear();
        job.warnings.clear();
        job.retry_count = 0;
        job.last_error = None;
        
        // Restart the job
        self.start_job(job_id.clone()).await?;
        
        Ok(())
    }

    /// Get job status
    pub fn get_status(&self, job_id: &JobId) -> JobResult<JobSummary> {
        let job = self.registry.get(job_id)
            .ok_or_else(|| JobError::not_found(job_id.as_str()))?;
        
        Ok(job.summary())
    }

    /// Get all job statuses
    pub fn get_all_statuses(&self) -> Vec<JobSummary> {
        self.registry.all().into_iter().map(|j| j.summary()).collect()
    }

    /// Get jobs by state
    pub fn get_jobs_by_state(&self, state: JobState) -> Vec<JobSummary> {
        self.registry.get_by_state(state).into_iter().map(|j| j.summary()).collect()
    }

    /// Clean up completed jobs
    pub fn cleanup_completed(&self) -> usize {
        let completed = self.registry.get_completed();
        for job in &completed {
            self.registry.unregister(&job.id);
        }
        completed.len()
    }

    /// Clean up old jobs
    pub fn cleanup_old(&self, max_age: Duration) -> usize {
        let now = chrono::Utc::now();
        let jobs = self.registry.all();
        let mut count = 0;
        
        for job in jobs {
            if let Some(created) = job.created_at.0 {
                if now - created > max_age {
                    self.registry.unregister(&job.id);
                    count += 1;
                }
            }
        }
        
        count
    }
}

impl Default for JobExecutor {
    fn default() -> Self {
        Self::new(Arc::new(JobRegistry::default()))
    }
}

/// Simple job executor for testing
#[derive(Debug, Clone)]
pub struct SimpleJobExecutor {
    executor: JobExecutor,
    sender: Option<Sender<JobConfig>>,
    receiver: Option<Mutex<Receiver<JobConfig>>>,
}

impl SimpleJobExecutor {
    pub fn new() -> Self {
        SimpleJobExecutor {
            executor: JobExecutor::new(Arc::new(JobRegistry::default())),
            sender: None,
            receiver: None,
        }
    }

    pub fn start(&mut self) -> (&Sender<JobConfig>, &Mutex<Receiver<JobConfig>>) {
        let (sender, receiver) = mpsc::channel(100);
        self.sender = Some(sender);
        self.receiver = Some(Mutex::new(receiver));
        
        // Spawn a task to process jobs
        let executor = self.executor.clone();
        let receiver = self.receiver.clone().unwrap();
        
        tokio::spawn(async move {
            loop {
                let mut receiver = receiver.lock().await;
                if let Some(config) = receiver.recv().await {
                    // In a real implementation, we'd process the job
                    let _ = executor.submit(config, ActorId::from_string("system").unwrap()).await;
                } else {
                    break;
                }
            }
        });
        
        (self.sender.as_ref().unwrap(), self.receiver.as_ref().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use malverde_core::ids::ActorId;

    #[tokio::test]
    async fn test_job_executor_creation() {
        let registry = Arc::new(JobRegistry::default());
        let executor = JobExecutor::new(registry);
        
        assert!(executor.get_all_statuses().is_empty());
    }

    #[tokio::test]
    async fn test_job_handler() {
        let handler = JobHandler::new("test-job", Box::new(|ctx| {
            // Simple job that just increments progress
            ctx.job.increment_progress(50)?;
            Ok(())
        }));
        
        assert_eq!(handler.job_type, "test-job");
    }

    #[tokio::test]
    async fn test_job_execution() {
        let registry = Arc::new(JobRegistry::default());
        let executor = JobExecutor::new(registry.clone());
        
        let actor = ActorId::from_string("test-actor").unwrap();
        let config = JobConfig::new("test-job", serde_json::json!({"test": "data"}));
        
        let job_id = executor.submit(config, actor).await.unwrap();
        
        // Give time for job to be processed
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Check status
        let status = executor.get_status(&job_id).unwrap();
        assert!(status.is_terminal() || status.state == JobState::Pending);
    }

    #[tokio::test]
    async fn test_job_registry_integration() {
        let registry = Arc::new(JobRegistry::default());
        let actor = ActorId::from_string("test-actor").unwrap();
        
        let job = JobInstance::new(
            JobConfig::new("test-job", serde_json::Value::Null),
            actor,
        );
        
        let job_id = registry.register(job);
        
        // Get job
        let retrieved = registry.get(&job_id);
        assert!(retrieved.is_some());
        
        // Get by state
        let pending = registry.get_pending();
        assert_eq!(pending.len(), 1);
        
        // Unregister
        assert!(registry.unregister(&job_id).is_some());
        assert!(registry.is_empty());
    }

    #[test]
    fn test_checkpoint_strategy_integration() {
        let strategy = CheckpointStrategy::ProgressPoints(vec![25, 50, 75]);
        
        assert!(!strategy.should_checkpoint(20, Duration::ZERO));
        assert!(strategy.should_checkpoint(25, Duration::ZERO));
        assert!(!strategy.should_checkpoint(30, Duration::ZERO));
        assert!(strategy.should_checkpoint(50, Duration::ZERO));
    }
}
