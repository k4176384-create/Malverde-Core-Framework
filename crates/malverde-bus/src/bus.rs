//! Event bus implementation

use crate::error::BusError;
use crate::subscriber::{SubscriberConfig, SubscriberId, SubscriberInfo, topic_matches, EventHandler};
use malverde_core::ids::{ActorId, EventId, ProjectId};
use malverde_core::states::{ActorType, EventType};
use malverde_events::event::{Event, EventMetadata, EventPayload};
use malverde_events::EventError;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::{self, Sender, Receiver};
use tokio::sync::Mutex;

/// Event bus for publishing and subscribing to events
#[derive(Debug)]
pub struct EventBus {
    /// Map of topic to list of subscribers
    subscribers: Arc<RwLock<HashMap<String, Vec<SubscriberInfo>>>>,
    /// Map of subscriber ID to their subscriptions
    subscriber_topics: Arc<RwLock<HashMap<SubscriberId, HashSet<String>>>>,
    /// Channel for publishing events
    event_sender: Option<Sender<BusEvent>>,
    /// Channel receiver for processing
    event_receiver: Option<Mutex<Receiver<BusEvent>>>,
    /// Whether the bus is running
    is_running: Arc<RwLock<bool>>,
    /// Bus ID
    id: String,
}

/// Internal bus event wrapper
#[derive(Debug, Clone)]
struct BusEvent {
    event: Event,
    topic: String,
    published_at: chrono::DateTime<chrono::Utc>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new(id: impl Into<String>) -> Self {
        EventBus {
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            subscriber_topics: Arc::new(RwLock::new(HashMap::new())),
            event_sender: None,
            event_receiver: None,
            is_running: Arc::new(RwLock::new(false)),
            id: id.into(),
        }
    }

    /// Start the event bus
    pub fn start(&mut self) -> Result<(), BusError> {
        let (sender, receiver) = mpsc::channel(1000);
        self.event_sender = Some(sender);
        self.event_receiver = Some(Mutex::new(receiver));
        *self.is_running.write().unwrap() = true;
        Ok(())
    }

    /// Stop the event bus
    pub fn stop(&mut self) -> Result<(), BusError> {
        *self.is_running.write().unwrap() = false;
        self.event_sender = None;
        self.event_receiver = None;
        Ok(())
    }

    /// Check if bus is running
    pub fn is_running(&self) -> bool {
        *self.is_running.read().unwrap()
    }

    /// Subscribe to a topic
    pub fn subscribe(&self, config: SubscriberConfig, handler: Arc<dyn EventHandler>) -> Result<SubscriberId, BusError> {
        if !self.is_running() {
            return Err(BusError::BusClosed);
        }

        let subscriber = SubscriberInfo::new(config.clone(), handler);
        
        // Add to subscribers map
        {
            let mut subscribers = self.subscribers.write().unwrap();
            subscribers
                .entry(config.topic.clone())
                .or_insert_with(Vec::new)
                .push(subscriber);
        }

        // Track subscriber topics
        {
            let mut topics = self.subscriber_topics.write().unwrap();
            topics
                .entry(config.id.clone())
                .or_insert_with(HashSet::new)
                .insert(config.topic);
        }

        Ok(config.id)
    }

    /// Subscribe to multiple topics with the same handler
    pub fn subscribe_all(&self, subscriber_id: SubscriberId, topics: Vec<String>, handler: Arc<dyn EventHandler>) -> Result<(), BusError> {
        if !self.is_running() {
            return Err(BusError::BusClosed);
        }

        for topic in topics {
            let config = SubscriberConfig::new(subscriber_id.as_str(), topic.clone());
            self.subscribe(config, handler.clone())?;
        }

        Ok(())
    }

    /// Unsubscribe a subscriber from a topic
    pub fn unsubscribe(&self, subscriber_id: &SubscriberId, topic: &str) -> Result<(), BusError> {
        if !self.is_running() {
            return Err(BusError::BusClosed);
        }

        // Remove from subscribers map
        {
            let mut subscribers = self.subscribers.write().unwrap();
            if let Some(subs) = subscribers.get_mut(topic) {
                subs.retain(|s| s.config.id != *subscriber_id);
                if subs.is_empty() {
                    subscribers.remove(topic);
                }
            }
        }

        // Remove from subscriber topics tracking
        {
            let mut topics = self.subscriber_topics.write().unwrap();
            if let Some(sub_topics) = topics.get_mut(subscriber_id) {
                sub_topics.remove(topic);
                if sub_topics.is_empty() {
                    topics.remove(subscriber_id);
                }
            }
        }

        Ok(())
    }

    /// Unsubscribe a subscriber from all topics
    pub fn unsubscribe_all(&self, subscriber_id: &SubscriberId) -> Result<(), BusError> {
        if !self.is_running() {
            return Err(BusError::BusClosed);
        }

        // Get all topics for this subscriber
        let topics_to_remove: Vec<String>;
        {
            let topics = self.subscriber_topics.read().unwrap();
            if let Some(sub_topics) = topics.get(subscriber_id) {
                topics_to_remove = sub_topics.iter().cloned().collect();
            } else {
                return Ok(());
            }
        }

        // Remove from all topics
        for topic in &topics_to_remove {
            self.unsubscribe(subscriber_id, topic)?;
        }

        Ok(())
    }

    /// Publish an event to a topic
    pub async fn publish(&self, topic: &str, event: Event) -> Result<(), BusError> {
        if !self.is_running() {
            return Err(BusError::BusClosed);
        }

        let bus_event = BusEvent {
            event: event.clone(),
            topic: topic.to_string(),
            published_at: chrono::Utc::now(),
        };

        // Send to channel
        if let Some(sender) = &self.event_sender {
            sender.send(bus_event).await.map_err(|e| {
                BusError::ChannelError(e.to_string())
            })?;
        }

        // Also directly notify subscribers (for immediate delivery)
        self.notify_subscribers(&event, topic).await?;

        Ok(())
    }

    /// Publish a simple event with builder pattern
    pub async fn publish_simple(
        &self,
        topic: &str,
        event_type: EventType,
        actor: ActorId,
        actor_type: ActorType,
    ) -> Result<(), BusError> {
        let payload = EventPayload::with_data(serde_json::Value::Null);
        let event = Event::new(event_type, actor, actor_type, payload)
            .with_metadata(EventMetadata::new().with_source("bus".to_string()));
        self.publish(topic, event).await
    }

    /// Publish with payload
    pub async fn publish_with_payload(
        &self,
        topic: &str,
        event_type: EventType,
        actor: ActorId,
        actor_type: ActorType,
        payload: serde_json::Value,
    ) -> Result<(), BusError> {
        let event = Event::new(
            event_type,
            actor,
            actor_type,
            EventPayload::with_data(payload),
        )
        .with_metadata(EventMetadata::new().with_source("bus".to_string()));
        self.publish(topic, event).await
    }

    /// Notify subscribers for an event
    async fn notify_subscribers(&self, event: &Event, topic: &str) -> Result<(), BusError> {
        let subscribers = self.subscribers.read().unwrap();
        
        // Find matching subscribers
        let matching_subscribers: Vec<&SubscriberInfo> = subscribers
            .iter()
            .flat_map(|(t, subs)| {
                if topic_matches(t, topic) {
                    subs.iter()
                } else {
                    Vec::new().into_iter()
                }
            })
            .collect();

        if matching_subscribers.is_empty() {
            log::debug!("No subscribers for topic: {}", topic);
            return Ok(());
        }

        // Notify each subscriber
        for subscriber in matching_subscribers {
            let handler = subscriber.handler.clone();
            let event = event.clone();
            
            tokio::spawn(async move {
                if let Err(e) = handler.handle(&event).await {
                    log::error!("Event handler error: {}", e);
                }
            });
        }

        Ok(())
    }

    /// Process events from the channel (internal)
    pub async fn process_events(&self) -> Result<(), BusError> {
        if !self.is_running() {
            return Err(BusError::BusClosed);
        }

        if let Some(receiver) = &self.event_receiver {
            let mut receiver = receiver.lock().await;
            while let Some(bus_event) = receiver.recv().await {
                self.notify_subscribers(&bus_event.event, &bus_event.topic).await?;
            }
        }

        Ok(())
    }

    /// Get all subscribers for a topic
    pub fn get_subscribers(&self, topic: &str) -> Vec<SubscriberId> {
        self.subscribers
            .read()
            .unwrap()
            .get(topic)
            .map(|subs| subs.iter().map(|s| s.config.id.clone()).collect())
            .unwrap_or_default()
    }

    /// Get all topics
    pub fn get_topics(&self) -> Vec<String> {
        self.subscribers
            .read()
            .unwrap()
            .keys()
            .cloned()
            .collect()
    }

    /// Get all subscribers for a subscriber ID
    pub fn get_subscriber_topics(&self, subscriber_id: &SubscriberId) -> Vec<String> {
        self.subscriber_topics
            .read()
            .unwrap()
            .get(subscriber_id)
            .map(|topics| topics.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get bus ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get count of subscribers
    pub fn subscriber_count(&self) -> usize {
        self.subscribers
            .read()
            .unwrap()
            .values()
            .map(|v| v.len())
            .sum()
    }

    /// Get count of topics
    pub fn topic_count(&self) -> usize {
        self.subscribers.read().unwrap().len()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        EventBus::new("default")
    }
}

/// Global event bus singleton
#[derive(Debug)]
pub struct GlobalEventBus {
    bus: Option<Arc<Mutex<EventBus>>>,
}

impl GlobalEventBus {
    pub fn new() -> Self {
        GlobalEventBus { bus: None }
    }

    pub fn init(&mut self) -> Arc<Mutex<EventBus>> {
        let mut bus = EventBus::new("global");
        bus.start().unwrap();
        let arc_bus = Arc::new(Mutex::new(bus));
        self.bus = Some(arc_bus.clone());
        arc_bus
    }

    pub fn get(&self) -> Option<Arc<Mutex<EventBus>>> {
        self.bus.clone()
    }

    pub fn publish(
        &self,
        topic: &str,
        event: Event,
    ) -> Result<(), BusError> {
        if let Some(bus) = &self.bus {
            let bus = bus.clone();
            tokio::spawn(async move {
                let b = bus.lock().await;
                b.publish(topic, event).await
            });
            Ok(())
        } else {
            Err(BusError::BusClosed)
        }
    }
}

impl Default for GlobalEventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper trait for publishing events
#[async_trait::async_trait]
pub trait EventPublisher {
    async fn publish_event(
        &self,
        topic: &str,
        event_type: EventType,
        actor: ActorId,
        actor_type: ActorType,
        payload: serde_json::Value,
    ) -> Result<(), BusError>;
}

#[async_trait::async_trait]
impl EventPublisher for EventBus {
    async fn publish_event(
        &self,
        topic: &str,
        event_type: EventType,
        actor: ActorId,
        actor_type: ActorType,
        payload: serde_json::Value,
    ) -> Result<(), BusError> {
        let event = Event::new(
            event_type,
            actor,
            actor_type,
            EventPayload::with_data(payload),
        );
        self.publish(topic, event).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use malverde_core::ids::ActorId;
    use malverde_core::states::{ActorType, EventType};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_bus_creation() {
        let mut bus = EventBus::new("test");
        assert!(!bus.is_running());
        
        bus.start().unwrap();
        assert!(bus.is_running());
        
        bus.stop().unwrap();
        assert!(!bus.is_running());
    }

    #[tokio::test]
    async fn test_subscribe_and_publish() {
        let mut bus = EventBus::new("test");
        bus.start().unwrap();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let handler = Arc::new(FnHandler {
            func: move |_event: &Event| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                async { Ok(()) }
            },
        });

        let config = SubscriberConfig::new("test-sub", "test.topic");
        bus.subscribe(config, handler).unwrap();

        let actor = ActorId::from_string("test-actor").unwrap();
        let payload = EventPayload::with_data(serde_json::json!({"test": "data"}));
        let event = Event::new(EventType::Info, actor, ActorType::System, payload);

        bus.publish("test.topic", event).await.unwrap();

        // Give time for async processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_wildcard_subscription() {
        let mut bus = EventBus::new("test");
        bus.start().unwrap();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let handler = Arc::new(FnHandler {
            func: move |_event: &Event| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                async { Ok(()) }
            },
        });

        let config = SubscriberConfig::new("test-sub", "test.*");
        bus.subscribe(config, handler).unwrap();

        let actor = ActorId::from_string("test-actor").unwrap();
        let payload = EventPayload::with_data(serde_json::json!({"test": "data"}));
        
        // Publish to matching topic
        let event1 = Event::new(EventType::Info, actor.clone(), ActorType::System, payload.clone());
        bus.publish("test.sub1", event1).await.unwrap();

        // Publish to non-matching topic
        let event2 = Event::new(EventType::Info, actor, ActorType::System, payload);
        bus.publish("other.topic", event2).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Only one event should have been received
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let mut bus = EventBus::new("test");
        bus.start().unwrap();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let handler = Arc::new(FnHandler {
            func: move |_event: &Event| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                async { Ok(()) }
            },
        });

        let sub_id = SubscriberId::new("test-sub");
        let config = SubscriberConfig::new(sub_id.clone(), "test.topic");
        bus.subscribe(config, handler).unwrap();

        let actor = ActorId::from_string("test-actor").unwrap();
        let payload = EventPayload::with_data(serde_json::json!({"test": "data"}));
        let event = Event::new(EventType::Info, actor, ActorType::System, payload);

        // First publish
        bus.publish("test.topic", event).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Unsubscribe
        bus.unsubscribe(&sub_id, "test.topic").unwrap();

        // Second publish (should not be received)
        let event2 = Event::new(EventType::Info, actor, ActorType::System, EventPayload::with_data(serde_json::json!({"test": "data2"})));
        bus.publish("test.topic", event2).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Count should still be 1
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_get_subscribers() {
        let mut bus = EventBus::new("test");
        bus.start().unwrap();

        let handler = Arc::new(FnHandler {
            func: |_| async { Ok(()) },
        });

        let sub_id1 = SubscriberId::new("sub1");
        let sub_id2 = SubscriberId::new("sub2");
        
        bus.subscribe(SubscriberConfig::new(sub_id1.clone(), "topic1".to_string()), handler.clone()).unwrap();
        bus.subscribe(SubscriberConfig::new(sub_id2.clone(), "topic1".to_string()), handler.clone()).unwrap();
        bus.subscribe(SubscriberConfig::new(sub_id1.clone(), "topic2".to_string()), handler).unwrap();

        let subs = bus.get_subscribers("topic1");
        assert_eq!(subs.len(), 2);
        assert!(subs.contains(&sub_id1));
        assert!(subs.contains(&sub_id2));

        let subs2 = bus.get_subscribers("topic2");
        assert_eq!(subs2.len(), 1);
        assert!(subs2.contains(&sub_id1));
    }

    #[tokio::test]
    async fn test_publish_simple() {
        let mut bus = EventBus::new("test");
        bus.start().unwrap();

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let handler = Arc::new(FnHandler {
            func: move |_event: &Event| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                async { Ok(()) }
            },
        });

        let config = SubscriberConfig::new("test-sub", "test.topic");
        bus.subscribe(config, handler).unwrap();

        let actor = ActorId::from_string("test-actor").unwrap();
        bus.publish_simple("test.topic", EventType::Info, actor, ActorType::System)
            .await
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
