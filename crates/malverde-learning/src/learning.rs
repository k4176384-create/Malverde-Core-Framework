//! Learning system core functionality

use malverde_core::ids::{KnowledgeId, MemoryId, PatternId};
use malverde_core::models::{KnowledgeItem, MemoryItem, Pattern};
use malverde_core::states::{KnowledgeState, MemoryType};
use malverde_core::timestamps::CreatedAt;
use malverde_core::trust::{Confidence, Evidence, Provenance};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use crate::error::{LearningError, LearningResult};

/// Learning configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearningConfig {
    pub learning_rate: f32,
    pub max_iterations: usize,
    pub batch_size: usize,
    pub min_confidence: Confidence,
    pub convergence_threshold: f32,
    pub enabled: bool,
}

impl LearningConfig {
    pub fn new() -> Self {
        LearningConfig {
            learning_rate: 0.01,
            max_iterations: 1000,
            batch_size: 32,
            min_confidence: Confidence::Medium,
            convergence_threshold: 0.001,
            enabled: true,
        }
    }

    pub fn with_learning_rate(mut self, rate: f32) -> Self {
        self.learning_rate = rate.clamp(0.0, 1.0);
        self
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    pub fn with_min_confidence(mut self, confidence: Confidence) -> Self {
        self.min_confidence = confidence;
        self
    }

    pub fn with_convergence_threshold(mut self, threshold: f32) -> Self {
        self.convergence_threshold = threshold;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Learning session for tracking learning progress
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearningSession {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub config: LearningConfig,
    pub started_at: CreatedAt,
    pub completed_at: Option<CreatedAt>,
    pub iterations: usize,
    pub current_loss: f32,
    pub initial_loss: f32,
    pub is_converged: bool,
    pub patterns_discovered: Vec<PatternId>,
    pub knowledge_updated: Vec<KnowledgeId>,
    pub memory_updated: Vec<MemoryId>,
    pub errors: Vec<String>,
    pub metadata: serde_json::Value,
}

impl LearningSession {
    pub fn new(name: impl Into<String>) -> Self {
        LearningSession {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            description: None,
            config: LearningConfig::default(),
            started_at: CreatedAt::now(),
            completed_at: None,
            iterations: 0,
            current_loss: 0.0,
            initial_loss: 0.0,
            is_converged: false,
            patterns_discovered: Vec::new(),
            knowledge_updated: Vec::new(),
            memory_updated: Vec::new(),
            errors: Vec::new(),
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_config(mut self, config: LearningConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_initial_loss(mut self, loss: f32) -> Self {
        self.initial_loss = loss;
        self.current_loss = loss;
        self
    }

    pub fn increment_iteration(&mut self) {
        self.iterations += 1;
    }

    pub fn update_loss(&mut self, loss: f32) {
        self.current_loss = loss;
    }

    pub fn add_discovered_pattern(&mut self, pattern_id: PatternId) {
        self.patterns_discovered.push(pattern_id);
    }

    pub fn add_updated_knowledge(&mut self, knowledge_id: KnowledgeId) {
        self.knowledge_updated.push(knowledge_id);
    }

    pub fn add_updated_memory(&mut self, memory_id: MemoryId) {
        self.memory_updated.push(memory_id);
    }

    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
    }

    pub fn mark_converged(&mut self) {
        self.is_converged = true;
    }

    pub fn mark_completed(&mut self) {
        self.completed_at = Some(CreatedAt::now());
    }

    pub fn is_complete(&self) -> bool {
        self.completed_at.is_some()
    }

    pub fn convergence_rate(&self) -> f32 {
        if self.initial_loss == 0.0 {
            return 0.0;
        }
        1.0 - (self.current_loss / self.initial_loss)
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Learning statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearningStats {
    pub total_sessions: usize,
    pub completed_sessions: usize,
    pub total_iterations: usize,
    pub total_patterns_discovered: usize,
    pub total_knowledge_updated: usize,
    pub total_memory_updated: usize,
    pub avg_convergence_rate: f32,
    pub error_rate: f32,
}

impl LearningStats {
    pub fn new() -> Self {
        LearningStats {
            total_sessions: 0,
            completed_sessions: 0,
            total_iterations: 0,
            total_patterns_discovered: 0,
            total_knowledge_updated: 0,
            total_memory_updated: 0,
            avg_convergence_rate: 0.0,
            error_rate: 0.0,
        }
    }
}

/// Learning manager for coordinating learning operations
#[derive(Debug, Clone)]
pub struct LearningManager {
    sessions: Arc<RwLock<Vec<LearningSession>>>,
    config: LearningConfig,
    stats: Arc<RwLock<LearningStats>>,
}

impl LearningManager {
    pub fn new() -> Self {
        LearningManager {
            sessions: Arc::new(RwLock::new(Vec::new())),
            config: LearningConfig::default(),
            stats: Arc::new(RwLock::new(LearningStats::new())),
        }
    }

    pub fn with_config(mut self, config: LearningConfig) -> Self {
        self.config = config;
        self
    }

    /// Create a new learning session
    pub fn create_session(&self, name: impl Into<String>) -> LearningSession {
        let session = LearningSession::new(name);
        let mut sessions = self.sessions.write().unwrap();
        sessions.push(session.clone());
        session
    }

    /// Get a learning session by ID
    pub fn get_session(&self, id: &str) -> Option<LearningSession> {
        let sessions = self.sessions.read().unwrap();
        sessions.iter().find(|s| s.id == id).cloned()
    }

    /// Get all learning sessions
    pub fn all_sessions(&self) -> Vec<LearningSession> {
        let sessions = self.sessions.read().unwrap();
        sessions.clone()
    }

    /// Get active (incomplete) sessions
    pub fn active_sessions(&self) -> Vec<LearningSession> {
        let sessions = self.sessions.read().unwrap();
        sessions.iter().filter(|s| !s.is_complete()).cloned().collect()
    }

    /// Get completed sessions
    pub fn completed_sessions(&self) -> Vec<LearningSession> {
        let sessions = self.sessions.read().unwrap();
        sessions.iter().filter(|s| s.is_complete()).cloned().collect()
    }

    /// Update session
    pub fn update_session(&self, session: LearningSession) -> LearningResult<()> {
        let mut sessions = self.sessions.write().unwrap();
        if let Some(index) = sessions.iter().position(|s| s.id == session.id) {
            sessions[index] = session;
            Ok(())
        } else {
            Err(LearningError::not_found(session.id))
        }
    }

    /// Delete a session
    pub fn delete_session(&self, id: &str) -> LearningResult<bool> {
        let mut sessions = self.sessions.write().unwrap();
        let len_before = sessions.len();
        sessions.retain(|s| s.id != id);
        Ok(len_before != sessions.len())
    }

    /// Clear all sessions
    pub fn clear_sessions(&self) {
        let mut sessions = self.sessions.write().unwrap();
        sessions.clear();
    }

    /// Get learning statistics
    pub fn get_stats(&self) -> LearningStats {
        let sessions = self.sessions.read().unwrap();
        let mut stats = LearningStats::new();

        stats.total_sessions = sessions.len();
        stats.completed_sessions = sessions.iter().filter(|s| s.is_complete()).count();
        stats.total_iterations = sessions.iter().map(|s| s.iterations).sum();
        stats.total_patterns_discovered = sessions.iter().map(|s| s.patterns_discovered.len()).sum();
        stats.total_knowledge_updated = sessions.iter().map(|s| s.knowledge_updated.len()).sum();
        stats.total_memory_updated = sessions.iter().map(|s| s.memory_updated.len()).sum();

        let mut total_convergence: f32 = 0.0;
        let mut convergence_count = 0;
        for session in sessions.iter().filter(|s| s.initial_loss > 0.0) {
            total_convergence += session.convergence_rate();
            convergence_count += 1;
        }

        if convergence_count > 0 {
            stats.avg_convergence_rate = total_convergence / convergence_count as f32;
        }

        let error_sessions = sessions.iter().filter(|s| s.has_errors()).count();
        if stats.total_sessions > 0 {
            stats.error_rate = error_sessions as f32 / stats.total_sessions as f32;
        }

        stats
    }

    /// Get the current configuration
    pub fn config(&self) -> &LearningConfig {
        &self.config
    }

    /// Update the configuration
    pub fn set_config(&mut self, config: LearningConfig) {
        self.config = config;
    }

    /// Get the number of sessions
    pub fn len(&self) -> usize {
        let sessions = self.sessions.read().unwrap();
        sessions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for LearningManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Learning result from a single iteration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearningIterationResult {
    pub iteration: usize,
    pub loss: f32,
    pub loss_change: f32,
    pub patterns_discovered: Vec<PatternId>,
    pub knowledge_updated: Vec<KnowledgeId>,
    pub memory_updated: Vec<MemoryId>,
    pub is_converged: bool,
    pub errors: Vec<String>,
}

impl LearningIterationResult {
    pub fn new(iteration: usize, loss: f32, previous_loss: f32) -> Self {
        LearningIterationResult {
            iteration,
            loss,
            loss_change: previous_loss - loss,
            patterns_discovered: Vec::new(),
            knowledge_updated: Vec::new(),
            memory_updated: Vec::new(),
            is_converged: false,
            errors: Vec::new(),
        }
    }

    pub fn with_patterns_discovered(mut self, patterns: Vec<PatternId>) -> Self {
        self.patterns_discovered = patterns;
        self
    }

    pub fn with_knowledge_updated(mut self, knowledge: Vec<KnowledgeId>) -> Self {
        self.knowledge_updated = knowledge;
        self
    }

    pub fn with_memory_updated(mut self, memory: Vec<MemoryId>) -> Self {
        self.memory_updated = memory;
        self
    }

    pub fn with_converged(mut self, converged: bool) -> Self {
        self.is_converged = converged;
        self
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.errors.push(error.into());
        self
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Learning strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LearningStrategy {
    /// Supervised learning from labeled data
    Supervised,
    /// Unsupervised learning (pattern discovery)
    Unsupervised,
    /// Reinforcement learning
    Reinforcement,
    /// Hybrid approach
    Hybrid,
}

impl std::fmt::Display for LearningStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Default for LearningStrategy {
    fn default() -> Self {
        LearningStrategy::Unsupervised
    }
}

/// Learning mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LearningMode {
    /// Online learning (continuous)
    Online,
    /// Batch learning (periodic)
    Batch,
    /// Incremental learning
    Incremental,
}

impl std::fmt::Display for LearningMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Default for LearningMode {
    fn default() -> Self {
        LearningMode::Incremental
    }
}

/// Learning task
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearningTask {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub strategy: LearningStrategy,
    pub mode: LearningMode,
    pub input_data: serde_json::Value,
    pub expected_output: Option<serde_json::Value>,
    pub config: LearningConfig,
    pub priority: u8,
    pub created_at: CreatedAt,
    pub started_at: Option<CreatedAt>,
    pub completed_at: Option<CreatedAt>,
    pub state: LearningTaskState,
    pub result: Option<LearningTaskResult>,
    pub errors: Vec<String>,
}

impl LearningTask {
    pub fn new(
        name: impl Into<String>,
        strategy: LearningStrategy,
        input_data: serde_json::Value,
    ) -> Self {
        LearningTask {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            description: None,
            strategy,
            mode: LearningMode::default(),
            input_data,
            expected_output: None,
            config: LearningConfig::default(),
            priority: 50,
            created_at: CreatedAt::now(),
            started_at: None,
            completed_at: None,
            state: LearningTaskState::Pending,
            result: None,
            errors: Vec::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_mode(mut self, mode: LearningMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_expected_output(mut self, output: serde_json::Value) -> Self {
        self.expected_output = Some(output);
        self
    }

    pub fn with_config(mut self, config: LearningConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.clamp(0, 100);
        self
    }

    pub fn start(&mut self) {
        self.state = LearningTaskState::Running;
        self.started_at = Some(CreatedAt::now());
    }

    pub fn mark_completed(&mut self, result: LearningTaskResult) {
        self.state = LearningTaskState::Completed;
        self.completed_at = Some(CreatedAt::now());
        self.result = Some(result);
    }

    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.state = LearningTaskState::Failed;
        self.completed_at = Some(CreatedAt::now());
        self.errors.push(error.into());
    }

    pub fn mark_cancelled(&mut self) {
        self.state = LearningTaskState::Cancelled;
        self.completed_at = Some(CreatedAt::now());
    }

    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            LearningTaskState::Completed | LearningTaskState::Failed | LearningTaskState::Cancelled
        )
    }
}

/// Learning task state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LearningTaskState {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for LearningTaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Default for LearningTaskState {
    fn default() -> Self {
        LearningTaskState::Pending
    }
}

/// Learning task result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearningTaskResult {
    pub accuracy: Option<f32>,
    pub loss: Option<f32>,
    pub patterns_discovered: Vec<PatternId>,
    pub knowledge_updated: Vec<KnowledgeId>,
    pub memory_updated: Vec<MemoryId>,
    pub confidence: Confidence,
    pub iterations: usize,
    pub duration: Option<std::time::Duration>,
    pub metadata: serde_json::Value,
}

impl LearningTaskResult {
    pub fn new() -> Self {
        LearningTaskResult {
            accuracy: None,
            loss: None,
            patterns_discovered: Vec::new(),
            knowledge_updated: Vec::new(),
            memory_updated: Vec::new(),
            confidence: Confidence::Medium,
            iterations: 0,
            duration: None,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_accuracy(mut self, accuracy: f32) -> Self {
        self.accuracy = Some(accuracy.clamp(0.0, 1.0));
        self
    }

    pub fn with_loss(mut self, loss: f32) -> Self {
        self.loss = Some(loss);
        self
    }

    pub fn with_patterns_discovered(mut self, patterns: Vec<PatternId>) -> Self {
        self.patterns_discovered = patterns;
        self
    }

    pub fn with_knowledge_updated(mut self, knowledge: Vec<KnowledgeId>) -> Self {
        self.knowledge_updated = knowledge;
        self
    }

    pub fn with_memory_updated(mut self, memory: Vec<MemoryId>) -> Self {
        self.memory_updated = memory;
        self
    }

    pub fn with_confidence(mut self, confidence: Confidence) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    pub fn with_duration(mut self, duration: std::time::Duration) -> Self {
        self.duration = Some(duration);
        self
    }
}

impl Default for LearningTaskResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Learning task queue
#[derive(Debug, Clone)]
pub struct LearningTaskQueue {
    tasks: Arc<RwLock<Vec<LearningTask>>>,
}

impl LearningTaskQueue {
    pub fn new() -> Self {
        LearningTaskQueue {
            tasks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a task to the queue
    pub fn add_task(&self, task: LearningTask) -> String {
        let mut tasks = self.tasks.write().unwrap();
        tasks.push(task);
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        tasks[0].id.clone()
    }

    /// Get the next task (highest priority)
    pub fn next_task(&self) -> Option<LearningTask> {
        let mut tasks = self.tasks.write().unwrap();
        if tasks.is_empty() {
            return None;
        }
        let task = tasks.remove(0);
        Some(task)
    }

    /// Peek at the next task without removing it
    pub fn peek_next(&self) -> Option<LearningTask> {
        let tasks = self.tasks.read().unwrap();
        tasks.first().cloned()
    }

    /// Get all tasks
    pub fn all_tasks(&self) -> Vec<LearningTask> {
        let tasks = self.tasks.read().unwrap();
        tasks.clone()
    }

    /// Get tasks by state
    pub fn tasks_by_state(&self, state: LearningTaskState) -> Vec<LearningTask> {
        let tasks = self.tasks.read().unwrap();
        tasks.iter().filter(|t| t.state == state).cloned().collect()
    }

    /// Update a task
    pub fn update_task(&self, task: LearningTask) -> LearningResult<()> {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(index) = tasks.iter().position(|t| t.id == task.id) {
            tasks[index] = task;
            tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
            Ok(())
        } else {
            Err(LearningError::not_found(task.id))
        }
    }

    /// Remove a task
    pub fn remove_task(&self, id: &str) -> LearningResult<bool> {
        let mut tasks = self.tasks.write().unwrap();
        let len_before = tasks.len();
        tasks.retain(|t| t.id != id);
        Ok(len_before != tasks.len())
    }

    /// Clear all tasks
    pub fn clear(&self) {
        let mut tasks = self.tasks.write().unwrap();
        tasks.clear();
    }

    /// Get the number of tasks
    pub fn len(&self) -> usize {
        let tasks = self.tasks.read().unwrap();
        tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for LearningTaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_learning_config() {
        let config = LearningConfig::new()
            .with_learning_rate(0.05)
            .with_max_iterations(500)
            .with_batch_size(64)
            .with_min_confidence(Confidence::High);

        assert_eq!(config.learning_rate, 0.05);
        assert_eq!(config.max_iterations, 500);
        assert_eq!(config.batch_size, 64);
        assert_eq!(config.min_confidence, Confidence::High);
    }

    #[test]
    fn test_learning_session() {
        let session = LearningSession::new("test-session")
            .with_description("Test learning session")
            .with_initial_loss(1.0);

        assert_eq!(session.name, "test-session");
        assert_eq!(session.description, Some("Test learning session".to_string()));
        assert_eq!(session.initial_loss, 1.0);
        assert_eq!(session.current_loss, 1.0);

        let mut session = session;
        session.increment_iteration();
        session.update_loss(0.5);

        assert_eq!(session.iterations, 1);
        assert_eq!(session.current_loss, 0.5);
        assert!((session.convergence_rate() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_learning_manager() {
        let manager = LearningManager::new();

        let session1 = manager.create_session("session1");
        let session2 = manager.create_session("session2");

        assert_eq!(manager.len(), 2);

        let session = manager.get_session(&session1.id);
        assert!(session.is_some());
        assert_eq!(session.unwrap().name, "session1");

        let all = manager.all_sessions();
        assert_eq!(all.len(), 2);

        assert!(manager.delete_session(&session1.id).unwrap());
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_learning_task() {
        let task = LearningTask::new(
            "test-task",
            LearningStrategy::Unsupervised,
            serde_json::json!({"data": "test"}),
        )
        .with_description("Test task")
        .with_priority(75);

        assert_eq!(task.name, "test-task");
        assert_eq!(task.strategy, LearningStrategy::Unsupervised);
        assert_eq!(task.priority, 75);

        let mut task = task;
        task.start();
        assert_eq!(task.state, LearningTaskState::Running);

        let result = LearningTaskResult::new()
            .with_accuracy(0.95)
            .with_loss(0.05)
            .with_iterations(100);

        task.mark_completed(result);
        assert_eq!(task.state, LearningTaskState::Completed);
        assert!(task.is_terminal());
    }

    #[test]
    fn test_learning_task_queue() {
        let queue = LearningTaskQueue::new();

        let task1 = LearningTask::new(
            "task1",
            LearningStrategy::Unsupervised,
            serde_json::Value::Null,
        ).with_priority(50);

        let task2 = LearningTask::new(
            "task2",
            LearningStrategy::Unsupervised,
            serde_json::Value::Null,
        ).with_priority(75);

        queue.add_task(task1);
        queue.add_task(task2);

        assert_eq!(queue.len(), 2);

        // Higher priority task should be next
        let next = queue.peek_next();
        assert!(next.is_some());
        assert_eq!(next.unwrap().name, "task2");

        // Remove the next task
        let next = queue.next_task();
        assert!(next.is_some());
        assert_eq!(next.unwrap().name, "task2");
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_learning_stats() {
        let manager = LearningManager::new();

        let mut session1 = LearningSession::new("session1");
        session1.with_initial_loss(1.0);
        session1.update_loss(0.3);
        session1.add_discovered_pattern(PatternId::from_string("pattern1").unwrap());
        session1.add_updated_knowledge(KnowledgeId::from_string("knowledge1").unwrap());
        session1.mark_completed();

        let mut session2 = LearningSession::new("session2");
        session2.with_initial_loss(1.0);
        session2.update_loss(0.5);
        session2.add_error("Test error");

        manager.create_session("session1");
        manager.create_session("session2");

        let stats = manager.get_stats();
        assert_eq!(stats.total_sessions, 2);
        assert_eq!(stats.completed_sessions, 0); // We didn't actually add the modified sessions
    }
}
