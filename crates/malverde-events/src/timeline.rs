//! Timeline reconstruction and event sequence management

use crate::event::Event;
use malverde_core::ids::EventId;
use malverde_core::states::EventType;
use std::collections::{HashMap, HashSet, VecDeque};

/// Timeline represents a sequence of events with temporal ordering
#[derive(Debug, Clone)]
pub struct Timeline {
    events: Vec<Event>,
    event_map: HashMap<EventId, usize>, // Event ID to index in events vector
    type_index: HashMap<EventType, Vec<usize>>, // Event type to list of indices
    project_index: HashMap<String, Vec<usize>>, // Project ID to list of indices
}

impl Timeline {
    pub fn new() -> Self {
        Timeline {
            events: Vec::new(),
            event_map: HashMap::new(),
            type_index: HashMap::new(),
            project_index: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Timeline {
            events: Vec::with_capacity(capacity),
            event_map: HashMap::with_capacity(capacity),
            type_index: HashMap::new(),
            project_index: HashMap::new(),
        }
    }

    /// Add an event to the timeline
    pub fn add_event(&mut self, event: Event) {
        let index = self.events.len();
        self.event_map.insert(event.id.clone(), index);
        
        // Index by type
        self.type_index
            .entry(event.event_type)
            .or_insert_with(Vec::new)
            .push(index);
        
        // Index by project
        if let Some(ref project) = event.project {
            self.project_index
                .entry(project.as_str().to_string())
                .or_insert_with(Vec::new)
                .push(index);
        }
        
        self.events.push(event);
    }

    /// Add multiple events to the timeline
    pub fn add_events(&mut self, events: impl IntoIterator<Item = Event>) {
        for event in events {
            self.add_event(event);
        }
    }

    /// Get event by ID
    pub fn get_event(&self, id: &EventId) -> Option<&Event> {
        self.event_map.get(id).and_then(|&idx| self.events.get(idx))
    }

    /// Get all events
    pub fn all_events(&self) -> &[Event] {
        &self.events
    }

    /// Get events of a specific type
    pub fn get_events_by_type(&self, event_type: EventType) -> Vec<&Event> {
        self.type_index
            .get(&event_type)
            .map(|indices| indices.iter().map(|&idx| &self.events[idx]).collect())
            .unwrap_or_default()
    }

    /// Get events for a specific project
    pub fn get_events_by_project(&self, project_id: &str) -> Vec<&Event> {
        self.project_index
            .get(project_id)
            .map(|indices| indices.iter().map(|&idx| &self.events[idx]).collect())
            .unwrap_or_default()
    }

    /// Get events within a time range
    pub fn get_events_in_range(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| e.timestamp.0 >= start && e.timestamp.0 <= end)
            .collect()
    }

    /// Get events by actor
    pub fn get_events_by_actor(&self, actor_id: &str) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| e.actor.as_str() == actor_id)
            .collect()
    }

    /// Get root events (events with no parent)
    pub fn get_root_events(&self) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| e.parent_event.is_none())
            .collect()
    }

    /// Get child events of a specific event
    pub fn get_child_events(&self, parent_id: &EventId) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| e.is_child_of(parent_id))
            .collect()
    }

    /// Get the full lineage tree for an event (parent chain and all descendants)
    pub fn get_event_tree(&self, event_id: &EventId) -> Vec<&Event> {
        let mut result = Vec::new();
        let mut to_visit = VecDeque::new();
        
        // Start with the event itself
        if let Some(event) = self.get_event(event_id) {
            to_visit.push_back(event);
        }
        
        // Collect all descendants
        while let Some(event) = to_visit.pop_front() {
            result.push(event);
            // Add all children
            let children = self.get_child_events(&event.id);
            for child in children {
                to_visit.push_back(child);
            }
        }
        
        result
    }

    /// Reconstruct the chronological sequence of events
    pub fn get_chronological_sequence(&self) -> Vec<&Event> {
        let mut sorted: Vec<&Event> = self.events.iter().collect();
        sorted.sort_by(|a, b| a.timestamp.0.cmp(&b.timestamp.0));
        sorted
    }

    /// Get events sorted by type and then by timestamp
    pub fn get_sorted_by_type(&self) -> Vec<&Event> {
        let mut sorted: Vec<&Event> = self.events.iter().collect();
        sorted.sort_by(|a, b| {
            let type_cmp = a.event_type.to_string().cmp(&b.event_type.to_string());
            if type_cmp == std::cmp::Ordering::Equal {
                a.timestamp.0.cmp(&b.timestamp.0)
            } else {
                type_cmp
            }
        });
        sorted
    }

    /// Count events by type
    pub fn count_by_type(&self) -> HashMap<EventType, usize> {
        let mut counts = HashMap::new();
        for event in &self.events {
            *counts.entry(event.event_type).or_insert(0) += 1;
        }
        counts
    }

    /// Count events by project
    pub fn count_by_project(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for event in &self.events {
            if let Some(ref project) = event.project {
                *counts.entry(project.as_str().to_string()).or_insert(0) += 1;
            }
        }
        counts
    }

    /// Get the most recent event of each type
    pub fn get_latest_by_type(&self) -> HashMap<EventType, &Event> {
        let mut latest = HashMap::new();
        for event in &self.events {
            match latest.get(&event.event_type) {
                Some(existing) => {
                    if event.timestamp.0 > existing.timestamp.0 {
                        latest.insert(event.event_type, event);
                    }
                }
                None => {
                    latest.insert(event.event_type, event);
                }
            }
        }
        latest
    }

    /// Check if timeline contains an event
    pub fn contains(&self, event_id: &EventId) -> bool {
        self.event_map.contains_key(event_id)
    }

    /// Get the number of events
    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Clear all events
    pub fn clear(&mut self) {
        self.events.clear();
        self.event_map.clear();
        self.type_index.clear();
        self.project_index.clear();
    }

    /// Get events with confidence above a threshold
    pub fn get_events_by_confidence(&self, min_confidence: malverde_core::trust::Confidence) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| e.confidence >= min_confidence)
            .collect()
    }

    /// Get events with specific tags
    pub fn get_events_by_tag(&self, tag: &str) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| e.metadata.tags.contains(&tag.to_string()))
            .collect()
    }

    /// Get correlation groups (events with the same correlation ID)
    pub fn get_correlation_groups(&self) -> HashMap<String, Vec<&Event>> {
        let mut groups = HashMap::new();
        for event in &self.events {
            if let Some(ref correlation_id) = event.metadata.correlation_id {
                groups
                    .entry(correlation_id.clone())
                    .or_insert_with(Vec::new)
                    .push(event);
            }
        }
        groups
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Timeline::new()
    }
}

/// Event sequence for ordered event processing
#[derive(Debug, Clone)]
pub struct EventSequence {
    events: Vec<Event>,
    current_index: usize,
}

impl EventSequence {
    pub fn new(events: Vec<Event>) -> Self {
        EventSequence {
            events,
            current_index: 0,
        }
    }

    pub fn from_timeline(timeline: &Timeline) -> Self {
        EventSequence {
            events: timeline.get_chronological_sequence().into_iter().cloned().collect(),
            current_index: 0,
        }
    }

    /// Get the next event in sequence
    pub fn next(&mut self) -> Option<&Event> {
        if self.current_index < self.events.len() {
            let event = &self.events[self.current_index];
            self.current_index += 1;
            Some(event)
        } else {
            None
        }
    }

    /// Peek at the next event without consuming it
    pub fn peek(&self) -> Option<&Event> {
        self.events.get(self.current_index)
    }

    /// Get the previous event
    pub fn prev(&mut self) -> Option<&Event> {
        if self.current_index > 0 {
            self.current_index -= 1;
            Some(&self.events[self.current_index])
        } else {
            None
        }
    }

    /// Reset to the beginning
    pub fn reset(&mut self) {
        self.current_index = 0;
    }

    /// Get current position
    pub fn position(&self) -> usize {
        self.current_index
    }

    /// Check if there are more events
    pub fn has_next(&self) -> bool {
        self.current_index < self.events.len()
    }

    /// Get all events
    pub fn all(&self) -> &[Event] {
        &self.events
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl Iterator for EventSequence {
    type Item = Event;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_index < self.events.len() {
            let event = self.events[self.current_index].clone();
            self.current_index += 1;
            Some(event)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Event, EventPayload};
    use malverde_core::ids::{ActorId, EventId, ProjectId};
    use malverde_core::states::EventType;
    use malverde_core::trust::Provenance;

    #[test]
    fn test_timeline_creation() {
        let timeline = Timeline::new();
        assert!(timeline.is_empty());
        assert_eq!(timeline.len(), 0);
    }

    #[test]
    fn test_timeline_add_and_get() {
        let mut timeline = Timeline::new();
        let actor = ActorId::from_string("test-actor").unwrap();
        let payload = EventPayload::with_data(serde_json::json!({"test": 1}));
        let event = Event::new(EventType::Info, actor, malverde_core::states::ActorType::System, payload);

        timeline.add_event(event.clone());
        
        assert_eq!(timeline.len(), 1);
        assert!(timeline.contains(&event.id));
        assert!(timeline.get_event(&event.id).is_some());
    }

    #[test]
    fn test_timeline_get_by_type() {
        let mut timeline = Timeline::new();
        let actor = ActorId::from_string("test-actor").unwrap();
        
        timeline.add_event(Event::new(
            EventType::Info,
            actor.clone(),
            malverde_core::states::ActorType::System,
            EventPayload::with_data(serde_json::json!({"test": 1})),
        ));
        timeline.add_event(Event::new(
            EventType::Warning,
            actor.clone(),
            malverde_core::states::ActorType::System,
            EventPayload::with_data(serde_json::json!({"test": 2})),
        ));
        timeline.add_event(Event::new(
            EventType::Info,
            actor,
            malverde_core::states::ActorType::System,
            EventPayload::with_data(serde_json::json!({"test": 3})),
        ));

        let info_events = timeline.get_events_by_type(EventType::Info);
        assert_eq!(info_events.len(), 2);
    }

    #[test]
    fn test_timeline_chronological_sequence() {
        let mut timeline = Timeline::new();
        let actor = ActorId::from_string("test-actor").unwrap();
        
        // Add events in non-chronological order
        let later = Event {
            id: EventId::from_string("later").unwrap(),
            event_type: EventType::Info,
            timestamp: CreatedAt::from(chrono::Utc::now() + chrono::Duration::hours(1)),
            actor: actor.clone(),
            actor_type: malverde_core::states::ActorType::System,
            project: None,
            operation: None,
            input_hash: None,
            output_hash: None,
            evidence: Vec::new(),
            confidence: malverde_core::trust::Confidence::Medium,
            provenance: None,
            parent_event: None,
            payload: EventPayload::with_data(serde_json::json!({"test": "later"})),
            metadata: crate::event::EventMetadata::new(),
        };
        
        let earlier = Event {
            id: EventId::from_string("earlier").unwrap(),
            event_type: EventType::Info,
            timestamp: CreatedAt::from(chrono::Utc::now() - chrono::Duration::hours(1)),
            actor: actor.clone(),
            actor_type: malverde_core::states::ActorType::System,
            project: None,
            operation: None,
            input_hash: None,
            output_hash: None,
            evidence: Vec::new(),
            confidence: malverde_core::trust::Confidence::Medium,
            provenance: None,
            parent_event: None,
            payload: EventPayload::with_data(serde_json::json!({"test": "earlier"})),
            metadata: crate::event::EventMetadata::new(),
        };

        timeline.add_event(later);
        timeline.add_event(earlier);

        let sequence = timeline.get_chronological_sequence();
        assert_eq!(sequence.len(), 2);
        // Earlier should come first
        assert!(sequence[0].timestamp.0 < sequence[1].timestamp.0);
    }

    #[test]
    fn test_event_sequence() {
        let actor = ActorId::from_string("test-actor").unwrap();
        let events = vec![
            Event::new(
                EventType::Info,
                actor.clone(),
                malverde_core::states::ActorType::System,
                EventPayload::with_data(serde_json::json!({"test": 1})),
            ),
            Event::new(
                EventType::Info,
                actor,
                malverde_core::states::ActorType::System,
                EventPayload::with_data(serde_json::json!({"test": 2})),
            ),
        ];

        let mut sequence = EventSequence::new(events);
        assert!(sequence.has_next());
        assert_eq!(sequence.position(), 0);

        let first = sequence.next();
        assert!(first.is_some());
        assert_eq!(sequence.position(), 1);

        let second = sequence.next();
        assert!(second.is_some());
        assert_eq!(sequence.position(), 2);

        assert!(!sequence.has_next());
        assert!(sequence.next().is_none());
    }
}
