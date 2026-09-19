//! State types for the Malverde Framework

use serde::{Deserialize, Serialize};
use strum::EnumIter;

/// Knowledge state - represents the certainty level of knowledge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter)]
pub enum KnowledgeState {
    Known,
    Inferred,
    Uncertain,
    Unknown,
}

impl KnowledgeState {
    pub fn all() -> &'static [KnowledgeState] {
        &[Self::Known, Self::Inferred, Self::Uncertain, Self::Unknown]
    }
    pub fn is_confirmed(&self) -> bool { matches!(self, KnowledgeState::Known) }
    pub fn is_uncertain(&self) -> bool { matches!(self, KnowledgeState::Uncertain | KnowledgeState::Unknown) }
    pub fn is_inferred(&self) -> bool { matches!(self, KnowledgeState::Inferred) }
    pub fn next(&self) -> Option<KnowledgeState> {
        match self {
            KnowledgeState::Known => None,
            KnowledgeState::Inferred => Some(KnowledgeState::Known),
            KnowledgeState::Uncertain => Some(KnowledgeState::Inferred),
            KnowledgeState::Unknown => Some(KnowledgeState::Uncertain),
        }
    }
    pub fn prev(&self) -> Option<KnowledgeState> {
        match self {
            KnowledgeState::Known => Some(KnowledgeState::Inferred),
            KnowledgeState::Inferred => Some(KnowledgeState::Uncertain),
            KnowledgeState::Uncertain => Some(KnowledgeState::Unknown),
            KnowledgeState::Unknown => None,
        }
    }
}

impl std::fmt::Display for KnowledgeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Default for KnowledgeState {
    fn default() -> Self { KnowledgeState::Unknown }
}

/// Memory type for categorizing memory items
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter)]
pub enum MemoryType {
    Semantic,
    Episodic,
    Procedural,
    Pattern,
}

impl MemoryType {
    pub fn all() -> &'static [MemoryType] {
        &[Self::Semantic, Self::Episodic, Self::Procedural, Self::Pattern]
    }
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Job state for tracking job execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter)]
pub enum JobState {
    Pending,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
    Recovering,
}

impl JobState {
    pub fn all() -> &'static [JobState] {
        &[Self::Pending, Self::Running, Self::Paused, Self::Completed, Self::Failed, Self::Cancelled, Self::Recovering]
    }
    pub fn is_running(&self) -> bool { matches!(self, JobState::Running) }
    pub fn is_terminal(&self) -> bool { matches!(self, JobState::Completed | JobState::Failed | JobState::Cancelled) }
    pub fn can_cancel(&self) -> bool { matches!(self, JobState::Pending | JobState::Running | JobState::Paused | JobState::Recovering) }
    pub fn can_pause(&self) -> bool { matches!(self, JobState::Running) }
    pub fn can_resume(&self) -> bool { matches!(self, JobState::Paused) }
    pub fn can_retry(&self) -> bool { matches!(self, JobState::Failed | JobState::Cancelled) }
    pub fn can_start(&self) -> bool { matches!(self, JobState::Pending | JobState::Recovering) }
}

impl std::fmt::Display for JobState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Default for JobState {
    fn default() -> Self { JobState::Pending }
}

/// Event type for categorizing events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter)]
pub enum EventType {
    SystemInit, SystemShutdown, ConfigChange,
    KnowledgeCreated, KnowledgeUpdated, KnowledgeDeleted,
    MemoryCreated, MemoryUpdated, MemoryDeleted,
    JobCreated, JobStarted, JobCompleted, JobFailed, JobCancelled, JobPaused, JobResumed,
    CheckpointCreated, CheckpointRestored, CheckpointDeleted,
    AuditCreated,
    PluginLoaded, PluginUnloaded, PluginError,
    CommandExecuted,
    Error, Warning, Info, Debug,
}

impl EventType {
    pub fn all() -> &'static [EventType] {
        &[
            Self::SystemInit, Self::SystemShutdown, Self::ConfigChange,
            Self::KnowledgeCreated, Self::KnowledgeUpdated, Self::KnowledgeDeleted,
            Self::MemoryCreated, Self::MemoryUpdated, Self::MemoryDeleted,
            Self::JobCreated, Self::JobStarted, Self::JobCompleted, Self::JobFailed,
            Self::JobCancelled, Self::JobPaused, Self::JobResumed,
            Self::CheckpointCreated, Self::CheckpointRestored, Self::CheckpointDeleted,
            Self::AuditCreated,
            Self::PluginLoaded, Self::PluginUnloaded, Self::PluginError,
            Self::CommandExecuted,
            Self::Error, Self::Warning, Self::Info, Self::Debug,
        ]
    }
    pub fn is_system(&self) -> bool { matches!(self, Self::SystemInit | Self::SystemShutdown | Self::ConfigChange) }
    pub fn is_knowledge(&self) -> bool { matches!(self, Self::KnowledgeCreated | Self::KnowledgeUpdated | Self::KnowledgeDeleted) }
    pub fn is_memory(&self) -> bool { matches!(self, Self::MemoryCreated | Self::MemoryUpdated | Self::MemoryDeleted) }
    pub fn is_job(&self) -> bool { matches!(self, Self::JobCreated | Self::JobStarted | Self::JobCompleted | Self::JobFailed | Self::JobCancelled | Self::JobPaused | Self::JobResumed) }
    pub fn is_checkpoint(&self) -> bool { matches!(self, Self::CheckpointCreated | Self::CheckpointRestored | Self::CheckpointDeleted) }
    pub fn is_plugin(&self) -> bool { matches!(self, Self::PluginLoaded | Self::PluginUnloaded | Self::PluginError) }
    pub fn is_log(&self) -> bool { matches!(self, Self::Error | Self::Warning | Self::Info | Self::Debug) }
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Actor type for identifying who performed an action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter)]
pub enum ActorType {
    System, User, Plugin, Job, ExternalService, ScheduledTask, Cli, Tui, Unknown,
}

impl ActorType {
    pub fn all() -> &'static [ActorType] {
        &[Self::System, Self::User, Self::Plugin, Self::Job, Self::ExternalService, Self::ScheduledTask, Self::Cli, Self::Tui, Self::Unknown]
    }
}

impl std::fmt::Display for ActorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Default for ActorType {
    fn default() -> Self { ActorType::System }
}

/// Operation type for categorizing operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter)]
pub enum OperationType {
    Create, Read, Update, Delete, List, Search, Execute, Validate,
    Import, Export, Sync, Backup, Restore, Check, Analyze, Learn, Infer, Unknown,
}

impl OperationType {
    pub fn all() -> &'static [OperationType] {
        &[Self::Create, Self::Read, Self::Update, Self::Delete, Self::List, Self::Search, Self::Execute, Self::Validate, Self::Import, Self::Export, Self::Sync, Self::Backup, Self::Restore, Self::Check, Self::Analyze, Self::Learn, Self::Infer, Self::Unknown]
    }
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Default for OperationType {
    fn default() -> Self { OperationType::Unknown }
}

/// Project state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProjectState { Active, Archived, Deleted }
impl Default for ProjectState { fn default() -> Self { ProjectState::Active } }

/// Operation state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationState { Pending, Running, Completed, Failed, Cancelled }
impl Default for OperationState { fn default() -> Self { OperationState::Pending } }
