//! Timestamp types for the Malverde Framework

use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::time;

/// Timestamp wrapper for creation time
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CreatedAt(pub DateTime<Utc>);

impl CreatedAt {
    pub fn now() -> Self { CreatedAt(Utc::now()) }
    pub fn as_datetime(&self) -> &DateTime<Utc> { &self.0 }
    pub fn to_rfc3339(&self) -> String { self.0.to_rfc3339() }
    pub fn as_unix_timestamp(&self) -> i64 { self.0.timestamp() }
    pub fn as_unix_timestamp_millis(&self) -> i64 { self.0.timestamp_millis() }
}

impl fmt::Display for CreatedAt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0.to_rfc3339()) }
}

impl From<DateTime<Utc>> for CreatedAt { fn from(dt: DateTime<Utc>) -> Self { CreatedAt(dt) } }
impl From<CreatedAt> for DateTime<Utc> { fn from(ca: CreatedAt) -> Self { ca.0 } }

/// Timestamp wrapper for update time
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UpdatedAt(pub DateTime<Utc>);

impl UpdatedAt {
    pub fn now() -> Self { UpdatedAt(Utc::now()) }
    pub fn as_datetime(&self) -> &DateTime<Utc> { &self.0 }
    pub fn to_rfc3339(&self) -> String { self.0.to_rfc3339() }
    pub fn as_unix_timestamp(&self) -> i64 { self.0.timestamp() }
}

impl fmt::Display for UpdatedAt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0.to_rfc3339()) }
}

impl From<DateTime<Utc>> for UpdatedAt { fn from(dt: DateTime<Utc>) -> Self { UpdatedAt(dt) } }
impl From<UpdatedAt> for DateTime<Utc> { fn from(ua: UpdatedAt) -> Self { ua.0 } }

/// Timestamp wrapper for access time
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AccessedAt(pub DateTime<Utc>);

impl AccessedAt {
    pub fn now() -> Self { AccessedAt(Utc::now()) }
    pub fn as_datetime(&self) -> &DateTime<Utc> { &self.0 }
    pub fn to_rfc3339(&self) -> String { self.0.to_rfc3339() }
}

impl fmt::Display for AccessedAt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{}", self.0.to_rfc3339()) }
}

impl From<DateTime<Utc>> for AccessedAt { fn from(dt: DateTime<Utc>) -> Self { AccessedAt(dt) } }

/// Timestamp wrapper for expiration time
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ExpiresAt(pub Option<DateTime<Utc>>);

impl ExpiresAt {
    pub fn after(duration: time::Duration) -> Self {
        ExpiresAt(Some(Utc::now() + Duration::from_std(duration).unwrap_or_default()))
    }
    pub fn never() -> Self { ExpiresAt(None) }
    pub fn is_expired(&self) -> bool {
        match self.0 { Some(dt) => dt <= Utc::now(), None => false }
    }
    pub fn as_datetime(&self) -> Option<&DateTime<Utc>> { self.0.as_ref() }
    pub fn to_rfc3339(&self) -> String {
        match self.0 { Some(dt) => dt.to_rfc3339(), None => "never".to_string() }
    }
}

impl fmt::Display for ExpiresAt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 { Some(dt) => write!(f, "{}", dt.to_rfc3339()), None => write!(f, "never") }
    }
}

impl From<Option<DateTime<Utc>>> for ExpiresAt { fn from(dt: Option<DateTime<Utc>>) -> Self { ExpiresAt(dt) } }

/// Timestamp range for queries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimestampRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimestampRange {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        TimestampRange { start, end }
    }
    pub fn last(duration: time::Duration) -> Self {
        let now = Utc::now();
        let start = now - Duration::from_std(duration).unwrap_or_default();
        TimestampRange { start, end: now }
    }
    pub fn contains(&self, ts: &DateTime<Utc>) -> bool {
        ts >= &self.start && ts <= &self.end
    }
    pub fn overlaps(&self, other: &TimestampRange) -> bool {
        self.start <= other.end && other.start <= self.end
    }
}

impl fmt::Display for TimestampRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} to {}", self.start.to_rfc3339(), self.end.to_rfc3339())
    }
}
