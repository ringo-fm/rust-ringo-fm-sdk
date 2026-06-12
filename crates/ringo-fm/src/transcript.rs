//! Transcript: JSON history of a session.

use serde_json::Value;

use crate::error::{Error, Result};

/// Roles in a transcript entry.
#[derive(Debug, Clone)]
pub enum TranscriptEntry {
    Instructions(Value),
    User(Value),
    Response(Value),
    Tool(Value),
    Other { role: String, raw: Value },
}

impl TranscriptEntry {
    fn from_value(v: Value) -> Self {
        let role = v.get("role").and_then(|r| r.as_str()).unwrap_or("").to_string();
        match role.as_str() {
            "instructions" => TranscriptEntry::Instructions(v),
            "user" => TranscriptEntry::User(v),
            "response" => TranscriptEntry::Response(v),
            "tool" => TranscriptEntry::Tool(v),
            other => TranscriptEntry::Other { role: other.to_string(), raw: v },
        }
    }
}

/// Full session transcript.
#[derive(Debug, Clone)]
pub struct Transcript {
    raw: Value,
}

impl Transcript {
    pub fn from_json(json: &str) -> Result<Self> {
        let raw = serde_json::from_str::<Value>(json).map_err(Error::from)?;
        Ok(Self { raw })
    }

    /// Raw JSON value (the full `{version, type, transcript: {entries: [...]}}` envelope).
    pub fn value(&self) -> &Value {
        &self.raw
    }

    /// Serialize back to JSON text.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string(&self.raw)?)
    }

    /// Number of entries in this transcript.
    pub fn entry_count(&self) -> usize {
        self.raw
            .get("transcript")
            .and_then(|t| t.get("entries"))
            .and_then(|e| e.as_array())
            .map(|a| a.len())
            .unwrap_or(0)
    }

    /// Parsed entries.
    pub fn entries(&self) -> Vec<TranscriptEntry> {
        self.raw
            .get("transcript")
            .and_then(|t| t.get("entries"))
            .and_then(|e| e.as_array())
            .map(|arr| arr.iter().cloned().map(TranscriptEntry::from_value).collect())
            .unwrap_or_default()
    }
}
