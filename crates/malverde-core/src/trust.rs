//! Trust and confidence types for the Malverde Framework

use serde::{Deserialize, Serialize};
use strum::EnumIter;

/// Confidence level for knowledge, events, and operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, EnumIter)]
pub enum Confidence {
    None = 0,
    Minimal = 10,
    Low = 30,
    Medium = 50,
    High = 70,
    VeryHigh = 90,
    Absolute = 100,
}

impl Confidence {
    pub fn as_u8(self) -> u8 { self as u8 }
    pub fn as_f32(self) -> f32 { (self as u8) as f32 / 100.0 }
    pub fn from_u8(value: u8) -> Self {
        match value {
            0..=5 => Confidence::None,
            6..=15 => Confidence::Minimal,
            16..=40 => Confidence::Low,
            41..=60 => Confidence::Medium,
            61..=80 => Confidence::High,
            81..=95 => Confidence::VeryHigh,
            _ => Confidence::Absolute,
        }
    }
    pub fn from_f32(value: f32) -> Self {
        let scaled = (value * 100.0).round() as u8;
        Self::from_u8(scaled)
    }
    pub fn is_at_least(self, other: Confidence) -> bool { self >= other }
    pub fn is_at_most(self, other: Confidence) -> bool { self <= other }
    pub fn next(self) -> Option<Self> {
        match self {
            Confidence::None => Some(Confidence::Minimal),
            Confidence::Minimal => Some(Confidence::Low),
            Confidence::Low => Some(Confidence::Medium),
            Confidence::Medium => Some(Confidence::High),
            Confidence::High => Some(Confidence::VeryHigh),
            Confidence::VeryHigh => Some(Confidence::Absolute),
            Confidence::Absolute => None,
        }
    }
    pub fn prev(self) -> Option<Self> {
        match self {
            Confidence::None => None,
            Confidence::Minimal => Some(Confidence::None),
            Confidence::Low => Some(Confidence::Minimal),
            Confidence::Medium => Some(Confidence::Low),
            Confidence::High => Some(Confidence::Medium),
            Confidence::VeryHigh => Some(Confidence::High),
            Confidence::Absolute => Some(Confidence::VeryHigh),
        }
    }
    pub fn all() -> &'static [Confidence] {
        &[Self::None, Self::Minimal, Self::Low, Self::Medium, Self::High, Self::VeryHigh, Self::Absolute]
    }
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Default for Confidence {
    fn default() -> Self { Confidence::Medium }
}

/// Trust level for actors and sources
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, EnumIter)]
pub enum TrustLevel {
    Untrusted = 0,
    Suspicious = 20,
    Neutral = 40,
    Trusted = 60,
    HighlyTrusted = 80,
    AbsoluteTrust = 100,
}

impl TrustLevel {
    pub fn as_u8(self) -> u8 { self as u8 }
    pub fn from_u8(value: u8) -> Self {
        match value {
            0..=10 => TrustLevel::Untrusted,
            11..=30 => TrustLevel::Suspicious,
            31..=50 => TrustLevel::Neutral,
            51..=70 => TrustLevel::Trusted,
            71..=90 => TrustLevel::HighlyTrusted,
            _ => TrustLevel::AbsoluteTrust,
        }
    }
    pub fn is_at_least(self, other: TrustLevel) -> bool { self >= other }
}

impl std::fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Default for TrustLevel {
    fn default() -> Self { TrustLevel::Neutral }
}

/// Provenance information for tracking the origin of data
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Provenance {
    pub source: String,
    pub method: String,
    pub version: Option<String>,
    pub metadata: serde_json::Value,
}

impl Provenance {
    pub fn new(source: impl Into<String>, method: impl Into<String>) -> Self {
        Provenance { source: source.into(), method: method.into(), version: None, metadata: serde_json::Value::Null }
    }
    pub fn with_version(mut self, version: impl Into<String>) -> Self { self.version = Some(version.into()); self }
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self { self.metadata = metadata; self }
}

impl std::fmt::Display for Provenance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.source, self.method)
    }
}

/// Evidence type for supporting claims
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub evidence_type: String,
    pub data: serde_json::Value,
    pub confidence: Confidence,
    pub provenance: Provenance,
    pub collected_at: chrono::DateTime<chrono::Utc>,
    pub description: Option<String>,
}

impl Evidence {
    pub fn new(id: impl Into<String>, evidence_type: impl Into<String>, data: serde_json::Value, provenance: Provenance) -> Self {
        Evidence {
            id: id.into(), evidence_type: evidence_type.into(), data,
            confidence: Confidence::default(), provenance,
            collected_at: chrono::Utc::now(), description: None,
        }
    }
    pub fn with_confidence(mut self, confidence: Confidence) -> Self { self.confidence = confidence; self }
    pub fn with_description(mut self, description: impl Into<String>) -> Self { self.description = Some(description.into()); self }
}

impl std::fmt::Display for Evidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Evidence({}:{} @ {})", self.evidence_type, self.id, self.collected_at.to_rfc3339())
    }
}
