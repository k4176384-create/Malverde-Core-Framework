//! Event subscriber types and traits

use malverde_events::event::Event;
use std::sync::Arc;

/// Trait for event handlers
#[async_trait::async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &Event) -> Result<(), malverde_events::EventError>;
}

/// Simple function handler wrapper
pub struct FnHandler<F> {
    func: F,
}

#[async_trait::async_trait]
impl<F, Fut> EventHandler for FnHandler<F>
where
    F: Send + Sync + Fn(&Event) -> Fut,
    Fut: std::future::Future<Output = Result<(), malverde_events::EventError>> + Send,
{
    async fn handle(&self, event: &Event) -> Result<(), malverde_events::EventError> {
        (self.func)(event).await
    }
}

/// Subscriber ID type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubscriberId(String);

impl SubscriberId {
    pub fn new(id: impl Into<String>) -> Self {
        SubscriberId(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SubscriberId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Subscriber configuration
#[derive(Debug, Clone)]
pub struct SubscriberConfig {
    pub id: SubscriberId,
    pub topic: String,
    pub priority: u8,
    pub max_retries: u32,
    pub timeout_ms: u64,
}

impl SubscriberConfig {
    pub fn new(id: impl Into<String>, topic: impl Into<String>) -> Self {
        SubscriberConfig {
            id: SubscriberId::new(id),
            topic: topic.into(),
            priority: 50,
            max_retries: 3,
            timeout_ms: 5000,
        }
    }
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority.clamp(0, 100);
        self
    }
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }
    pub fn with_timeout_ms(mut self, timeout: u64) -> Self {
        self.timeout_ms = timeout;
        self
    }
}

impl Default for SubscriberConfig {
    fn default() -> Self {
        SubscriberConfig::new("default", "default")
    }
}

/// Subscriber information
#[derive(Debug, Clone)]
pub struct SubscriberInfo {
    pub config: SubscriberConfig,
    pub handler: Arc<dyn EventHandler>,
}

impl SubscriberInfo {
    pub fn new(config: SubscriberConfig, handler: Arc<dyn EventHandler>) -> Self {
        SubscriberInfo { config, handler }
    }
}

/// Create a simple subscriber from a function
pub fn create_subscriber<F, Fut>(
    id: impl Into<String>,
    topic: impl Into<String>,
    handler: F,
) -> SubscriberInfo
where
    F: Send + Sync + 'static + Fn(&Event) -> Fut,
    Fut: std::future::Future<Output = Result<(), malverde_events::EventError>> + Send,
{
    SubscriberInfo::new(
        SubscriberConfig::new(id, topic),
        Arc::new(FnHandler { func: handler }),
    )
}

/// Wildcard topic patterns
pub struct TopicPattern;

impl TopicPattern {
    /// Match all topics
    pub const ALL: &'static str = "*";
    /// Match system topics
    pub const SYSTEM: &'static str = "system.*";
    /// Match knowledge topics
    pub const KNOWLEDGE: &'static str = "knowledge.*";
    /// Match memory topics
    pub const MEMORY: &'static str = "memory.*";
    /// Match job topics
    pub const JOB: &'static str = "job.*";
    /// Match audit topics
    pub const AUDIT: &'static str = "audit.*";
}

/// Check if a topic matches a pattern
pub fn topic_matches(pattern: &str, topic: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern == topic {
        return true;
    }
    if pattern.ends_with(".*") {
        let prefix = &pattern[..pattern.len() - 2];
        return topic.starts_with(prefix) && topic.len() > prefix.len() && topic.chars().nth(prefix.len()) == Some('.');
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topic_matching() {
        assert!(topic_matches("*", "any.topic"));
        assert!(topic_matches("system.*", "system.init"));
        assert!(topic_matches("system.*", "system.shutdown"));
        assert!(!topic_matches("system.*", "system"));
        assert!(!topic_matches("system.*", "other.system"));
        assert!(topic_matches("exact.topic", "exact.topic"));
        assert!(!topic_matches("exact.topic", "exact.topic.other"));
    }

    #[test]
    fn test_subscriber_config() {
        let config = SubscriberConfig::new("test", "test.topic")
            .with_priority(75)
            .with_max_retries(5)
            .with_timeout_ms(10000);

        assert_eq!(config.id.as_str(), "test");
        assert_eq!(config.topic, "test.topic");
        assert_eq!(config.priority, 75);
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.timeout_ms, 10000);
    }

    #[test]
    fn test_subscriber_id() {
        let id = SubscriberId::new("test-123");
        assert_eq!(id.as_str(), "test-123");
        assert_eq!(format!("{}", id), "test-123");
    }
}
