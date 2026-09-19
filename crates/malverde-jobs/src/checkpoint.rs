//! Checkpoint system for job recovery

use malverde_core::ids::{CheckpointId, JobId};
use malverde_core::timestamps::CreatedAt;
use malverde_core::{MalverdeError, MalverdeResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Checkpoint represents a recovery point for a job
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: CheckpointId,
    pub job_id: JobId,
    pub metadata: serde_json::Value,
    pub created_at: CreatedAt,
    pub is_valid: bool,
    pub data_hash: String,
}

impl Checkpoint {
    /// Create a new checkpoint
    pub fn new(job_id: JobId, metadata: serde_json::Value) -> Self {
        let id = CheckpointId::from_string(&Uuid::new_v4().to_string()).unwrap();
        let data_hash = Self::calculate_hash(&metadata);
        
        Checkpoint {
            id,
            job_id,
            metadata,
            created_at: CreatedAt::now(),
            is_valid: true,
            data_hash,
        }
    }

    /// Create a checkpoint with custom ID
    pub fn with_id(id: CheckpointId, job_id: JobId, metadata: serde_json::Value) -> Self {
        let data_hash = Self::calculate_hash(&metadata);
        
        Checkpoint {
            id,
            job_id,
            metadata,
            created_at: CreatedAt::now(),
            is_valid: true,
            data_hash,
        }
    }

    /// Validate the checkpoint
    pub fn validate(&self) -> bool {
        // Recalculate hash and compare
        let calculated_hash = Self::calculate_hash(&self.metadata);
        calculated_hash == self.data_hash
    }

    /// Invalidate the checkpoint
    pub fn invalidate(&mut self) {
        self.is_valid = false;
    }

    /// Mark as valid
    pub fn mark_valid(&mut self) {
        self.is_valid = true;
    }

    /// Update metadata (recalculates hash)
    pub fn update_metadata(&mut self, metadata: serde_json::Value) {
        self.metadata = metadata;
        self.data_hash = Self::calculate_hash(&self.metadata);
    }

    /// Calculate hash of metadata
    fn calculate_hash(metadata: &serde_json::Value) -> String {
        let mut hasher = Sha256::new();
        hasher.update(metadata.to_string());
        format!("{:x}", hasher.finalize())
    }

    /// Get the metadata as a specific type
    pub fn get_metadata<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        serde_json::from_value(self.metadata.clone()).ok()
    }

    /// Set metadata from a value
    pub fn set_metadata<T: serde::Serialize>(&mut self, value: &T) -> Result<(), serde_json::Error> {
        self.metadata = serde_json::to_value(value)?;
        self.data_hash = Self::calculate_hash(&self.metadata);
        Ok(())
    }
}

/// Checkpoint metadata for job recovery
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    pub job_state: String,
    pub progress: u8,
    pub input_hash: Option<String>,
    pub output_hash: Option<String>,
    pub current_step: Option<String>,
    pub step_data: serde_json::Value,
    pub custom: serde_json::Value,
}

impl CheckpointMetadata {
    pub fn new() -> Self {
        CheckpointMetadata {
            job_state: "Running".to_string(),
            progress: 0,
            input_hash: None,
            output_hash: None,
            current_step: None,
            step_data: serde_json::Value::Null,
            custom: serde_json::Value::Null,
        }
    }

    pub fn with_job_state(mut self, state: impl Into<String>) -> Self {
        self.job_state = state.into();
        self
    }

    pub fn with_progress(mut self, progress: u8) -> Self {
        self.progress = progress;
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

    pub fn with_current_step(mut self, step: impl Into<String>) -> Self {
        self.current_step = Some(step.into());
        self
    }

    pub fn with_step_data(mut self, data: serde_json::Value) -> Self {
        self.step_data = data;
        self
    }

    pub fn with_custom(mut self, custom: serde_json::Value) -> Self {
        self.custom = custom;
        self
    }
}

impl Default for CheckpointMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Checkpoint manager for a specific job
#[derive(Debug, Clone)]
pub struct CheckpointManager {
    job_id: JobId,
    checkpoints: Arc<RwLock<Vec<Checkpoint>>>,
    max_checkpoints: usize,
}

impl CheckpointManager {
    pub fn new(job_id: JobId) -> Self {
        CheckpointManager {
            job_id,
            checkpoints: Arc::new(RwLock::new(Vec::new())),
            max_checkpoints: 10,
        }
    }

    pub fn with_max_checkpoints(mut self, max: usize) -> Self {
        self.max_checkpoints = max;
        self
    }

    /// Create a new checkpoint
    pub fn create(&self, metadata: serde_json::Value) -> Checkpoint {
        let mut checkpoints = self.checkpoints.write().unwrap();
        
        // If we have too many checkpoints, remove the oldest
        if checkpoints.len() >= self.max_checkpoints {
            checkpoints.remove(0);
        }

        let checkpoint = Checkpoint::new(self.job_id.clone(), metadata);
        checkpoints.push(checkpoint.clone());
        checkpoint
    }

    /// Get the latest checkpoint
    pub fn latest(&self) -> Option<Checkpoint> {
        let checkpoints = self.checkpoints.read().unwrap();
        checkpoints.last().cloned()
    }

    /// Get the latest valid checkpoint
    pub fn latest_valid(&self) -> Option<Checkpoint> {
        let checkpoints = self.checkpoints.read().unwrap();
        checkpoints.iter().rev().find(|c| c.is_valid).cloned()
    }

    /// Get checkpoint by ID
    pub fn get(&self, id: &CheckpointId) -> Option<Checkpoint> {
        let checkpoints = self.checkpoints.read().unwrap();
        checkpoints.iter().find(|c| &c.id == id).cloned()
    }

    /// Get all checkpoints
    pub fn all(&self) -> Vec<Checkpoint> {
        let checkpoints = self.checkpoints.read().unwrap();
        checkpoints.clone()
    }

    /// Get valid checkpoints
    pub fn valid(&self) -> Vec<Checkpoint> {
        let checkpoints = self.checkpoints.read().unwrap();
        checkpoints.iter().filter(|c| c.is_valid).cloned().collect()
    }

    /// Invalidate a checkpoint
    pub fn invalidate(&self, id: &CheckpointId) -> bool {
        let mut checkpoints = self.checkpoints.write().unwrap();
        if let Some(cp) = checkpoints.iter_mut().find(|c| &c.id == id) {
            cp.invalidate();
            true
        } else {
            false
        }
    }

    /// Delete a checkpoint
    pub fn delete(&self, id: &CheckpointId) -> bool {
        let mut checkpoints = self.checkpoints.write().unwrap();
        let len_before = checkpoints.len();
        checkpoints.retain(|c| &c.id != id);
        len_before != checkpoints.len()
    }

    /// Delete all checkpoints
    pub fn clear(&self) {
        let mut checkpoints = self.checkpoints.write().unwrap();
        checkpoints.clear();
    }

    /// Get checkpoint count
    pub fn len(&self) -> usize {
        let checkpoints = self.checkpoints.read().unwrap();
        checkpoints.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the checkpoint at a specific index
    pub fn get_at(&self, index: usize) -> Option<Checkpoint> {
        let checkpoints = self.checkpoints.read().unwrap();
        checkpoints.get(index).cloned()
    }

    /// Find checkpoint by data hash
    pub fn find_by_hash(&self, hash: &str) -> Option<Checkpoint> {
        let checkpoints = self.checkpoints.read().unwrap();
        checkpoints.iter().find(|c| c.data_hash == hash).cloned()
    }
}

/// Global checkpoint registry
#[derive(Debug, Clone)]
pub struct CheckpointRegistry {
    managers: Arc<RwLock<HashMap<JobId, Arc<CheckpointManager>>>>,
}

impl CheckpointRegistry {
    pub fn new() -> Self {
        CheckpointRegistry {
            managers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or create a checkpoint manager for a job
    pub fn get_manager(&self, job_id: JobId) -> Arc<CheckpointManager> {
        let mut managers = self.managers.write().unwrap();
        managers
            .entry(job_id.clone())
            .or_insert_with(|| Arc::new(CheckpointManager::new(job_id)))
            .clone()
    }

    /// Get all checkpoint managers
    pub fn all_managers(&self) -> Vec<Arc<CheckpointManager>> {
        let managers = self.managers.read().unwrap();
        managers.values().cloned().collect()
    }

    /// Get checkpoint manager for a job
    pub fn get(&self, job_id: &JobId) -> Option<Arc<CheckpointManager>> {
        let managers = self.managers.read().unwrap();
        managers.get(job_id).cloned()
    }

    /// Remove checkpoint manager for a job
    pub fn remove(&self, job_id: &JobId) -> Option<Arc<CheckpointManager>> {
        let mut managers = self.managers.write().unwrap();
        managers.remove(job_id)
    }

    /// Clear all checkpoint managers
    pub fn clear(&self) {
        let mut managers = self.managers.write().unwrap();
        managers.clear();
    }

    /// Get total checkpoint count across all jobs
    pub fn total_checkpoints(&self) -> usize {
        let managers = self.managers.read().unwrap();
        managers.values().map(|m| m.len()).sum()
    }

    /// Get all checkpoints across all jobs
    pub fn all_checkpoints(&self) -> Vec<Checkpoint> {
        let managers = self.managers.read().unwrap();
        managers
            .values()
            .flat_map(|m| m.all())
            .collect()
    }

    /// Get all valid checkpoints across all jobs
    pub fn all_valid_checkpoints(&self) -> Vec<Checkpoint> {
        let managers = self.managers.read().unwrap();
        managers
            .values()
            .flat_map(|m| m.valid())
            .collect()
    }
}

impl Default for CheckpointRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Checkpoint recovery information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveryInfo {
    pub checkpoint: Checkpoint,
    pub job_id: JobId,
    pub can_recover: bool,
    pub recovery_steps: Vec<String>,
}

impl RecoveryInfo {
    pub fn new(checkpoint: Checkpoint, job_id: JobId) -> Self {
        RecoveryInfo {
            checkpoint,
            job_id,
            can_recover: true,
            recovery_steps: Vec::new(),
        }
    }

    pub fn with_can_recover(mut self, can_recover: bool) -> Self {
        self.can_recover = can_recover;
        self
    }

    pub fn with_recovery_steps(mut self, steps: Vec<String>) -> Self {
        self.recovery_steps = steps;
        self
    }

    pub fn add_recovery_step(mut self, step: impl Into<String>) -> Self {
        self.recovery_steps.push(step.into());
        self
    }
}

/// Checkpoint validation result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckpointValidation {
    pub is_valid: bool,
    pub checkpoint_id: CheckpointId,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl CheckpointValidation {
    pub fn valid(checkpoint_id: CheckpointId) -> Self {
        CheckpointValidation {
            is_valid: true,
            checkpoint_id,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn invalid(checkpoint_id: CheckpointId, errors: Vec<String>) -> Self {
        CheckpointValidation {
            is_valid: false,
            checkpoint_id,
            errors,
            warnings: Vec::new(),
        }
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.errors.push(error.into());
        self.is_valid = false;
        self
    }

    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }
}

/// Checkpoint strategy for automatic checkpointing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointStrategy {
    /// Never create checkpoints
    Never,
    /// Create checkpoints at regular intervals
    Interval(Duration),
    /// Create checkpoints at specific progress percentages
    ProgressPoints(Vec<u8>),
    /// Create checkpoints at both intervals and progress points
    Combined(Duration, Vec<u8>),
}

impl CheckpointStrategy {
    /// Check if a checkpoint should be created based on current state
    pub fn should_checkpoint(&self, progress: u8, elapsed: Duration) -> bool {
        match self {
            CheckpointStrategy::Never => false,
            CheckpointStrategy::Interval(interval) => elapsed >= *interval,
            CheckpointStrategy::ProgressPoints(points) => points.contains(&progress),
            CheckpointStrategy::Combined(interval, points) => {
                elapsed >= *interval || points.contains(&progress)
            }
        }
    }

    /// Get the next checkpoint trigger
    pub fn next_trigger(&self, progress: u8, elapsed: Duration) -> Option<(u8, Duration)> {
        match self {
            CheckpointStrategy::Never => None,
            CheckpointStrategy::Interval(interval) => {
                let remaining = *interval - elapsed;
                if remaining > Duration::ZERO {
                    Some((progress, remaining))
                } else {
                    Some((progress, Duration::from_secs(0)))
                }
            }
            CheckpointStrategy::ProgressPoints(points) => {
                // Find the next progress point greater than current
                points.iter()
                    .filter(|&&p| p > progress)
                    .min()
                    .map(|&p| (p, Duration::from_secs(0)))
            }
            CheckpointStrategy::Combined(interval, points) => {
                let interval_trigger = CheckpointStrategy::Interval(*interval).next_trigger(progress, elapsed);
                let progress_trigger = CheckpointStrategy::ProgressPoints(points.clone()).next_trigger(progress, elapsed);
                
                match (interval_trigger, progress_trigger) {
                    (Some(i), Some(p)) => {
                        // Return the sooner trigger
                        if i.1 <= p.1 {
                            Some(i)
                        } else {
                            Some(p)
                        }
                    }
                    (Some(i), None) => Some(i),
                    (None, Some(p)) => Some(p),
                    (None, None) => None,
                }
            }
        }
    }
}

impl Default for CheckpointStrategy {
    fn default() -> Self {
        CheckpointStrategy::Interval(Duration::from_secs(60))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use malverde_core::ids::JobId;

    #[test]
    fn test_checkpoint_creation() {
        let job_id = JobId::from_string("test-job").unwrap();
        let metadata = serde_json::json!({"progress": 50, "step": "processing"});
        
        let checkpoint = Checkpoint::new(job_id.clone(), metadata);
        
        assert_eq!(checkpoint.job_id, job_id);
        assert!(checkpoint.is_valid);
        assert!(!checkpoint.data_hash.is_empty());
    }

    #[test]
    fn test_checkpoint_validation() {
        let job_id = JobId::from_string("test-job").unwrap();
        let metadata = serde_json::json!({"test": "data"});
        
        let mut checkpoint = Checkpoint::new(job_id, metadata.clone());
        
        // Should be valid
        assert!(checkpoint.validate());
        
        // Modify metadata and recalculate hash
        checkpoint.update_metadata(serde_json::json!({"test": "modified"}));
        assert!(checkpoint.validate());
        
        // Invalidate
        checkpoint.invalidate();
        assert!(!checkpoint.is_valid);
    }

    #[test]
    fn test_checkpoint_manager() {
        let job_id = JobId::from_string("test-job").unwrap();
        let manager = CheckpointManager::new(job_id.clone());
        
        // Create checkpoints
        manager.create(serde_json::json!({"step": 1}));
        manager.create(serde_json::json!({"step": 2}));
        
        assert_eq!(manager.len(), 2);
        
        // Get latest
        let latest = manager.latest();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().job_id, job_id);
        
        // Get latest valid
        let latest_valid = manager.latest_valid();
        assert!(latest_valid.is_some());
        
        // Clear
        manager.clear();
        assert!(manager.is_empty());
    }

    #[test]
    fn test_checkpoint_registry() {
        let registry = CheckpointRegistry::new();
        let job_id1 = JobId::from_string("job1").unwrap();
        let job_id2 = JobId::from_string("job2").unwrap();
        
        // Get managers
        let manager1 = registry.get_manager(job_id1.clone());
        let manager2 = registry.get_manager(job_id2.clone());
        
        // Create checkpoints
        manager1.create(serde_json::json!({"test": 1}));
        manager2.create(serde_json::json!({"test": 2}));
        manager2.create(serde_json::json!({"test": 3}));
        
        assert_eq!(registry.total_checkpoints(), 3);
        
        // Get all checkpoints
        let all = registry.all_checkpoints();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_checkpoint_strategy() {
        // Interval strategy
        let interval_strategy = CheckpointStrategy::Interval(Duration::from_secs(10));
        assert!(!interval_strategy.should_checkpoint(0, Duration::from_secs(5)));
        assert!(interval_strategy.should_checkpoint(0, Duration::from_secs(10)));
        assert!(interval_strategy.should_checkpoint(0, Duration::from_secs(15)));
        
        // Progress points strategy
        let progress_strategy = CheckpointStrategy::ProgressPoints(vec![25, 50, 75]);
        assert!(!progress_strategy.should_checkpoint(20, Duration::ZERO));
        assert!(progress_strategy.should_checkpoint(25, Duration::ZERO));
        assert!(!progress_strategy.should_checkpoint(30, Duration::ZERO));
        assert!(progress_strategy.should_checkpoint(50, Duration::ZERO));
        
        // Combined strategy
        let combined = CheckpointStrategy::Combined(Duration::from_secs(10), vec![25, 50, 75]);
        assert!(!combined.should_checkpoint(20, Duration::from_secs(5)));
        assert!(combined.should_checkpoint(25, Duration::from_secs(5))); // Progress point
        assert!(combined.should_checkpoint(20, Duration::from_secs(10))); // Interval
    }

    #[test]
    fn test_checkpoint_metadata() {
        let metadata = CheckpointMetadata::new()
            .with_job_state("Running")
            .with_progress(50)
            .with_current_step("processing")
            .with_step_data(serde_json::json!({"data": "test"}))
            .with_custom(serde_json::json!({"custom": "value"}));
        
        assert_eq!(metadata.progress, 50);
        assert_eq!(metadata.current_step, Some("processing".to_string()));
    }
}
