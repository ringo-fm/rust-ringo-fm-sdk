//! Schema Discovery helpers built on structured generation.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::Result;
use crate::options::GenerationOptions;
use crate::prompt::Prompt;
use crate::session::LanguageModelSession;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiscoverSchemaRequest {
    pub documents: Vec<DiscoveryDocument>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hints: Option<DiscoveryHints>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<DiscoveryOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub existing_schema: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiscoveryDocument {
    pub id: String,
    pub source: DiscoveryDocumentSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiscoveryDocumentSource {
    #[serde(rename = "type")]
    pub source_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiscoveryHints {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_schema_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryOptions {
    pub include_evidence: bool,
    pub include_layout: bool,
    pub include_raw_extractions: bool,
    pub min_presence_rate_for_required: f64,
    pub min_confidence: f64,
    pub max_field_candidates: u32,
    pub merge_similar_fields: bool,
    pub infer_constraints: bool,
    pub infer_arrays: bool,
    pub infer_nested_objects: bool,
}

impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            include_evidence: true,
            include_layout: true,
            include_raw_extractions: false,
            min_presence_rate_for_required: 0.8,
            min_confidence: 0.5,
            max_field_candidates: 200,
            merge_similar_fields: true,
            infer_constraints: true,
            infer_arrays: true,
            infer_nested_objects: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiscoverSchemaResponse {
    pub schema_candidate: SchemaCandidate,
    #[serde(default)]
    pub field_candidates: Vec<FieldCandidate>,
    #[serde(default)]
    pub document_summaries: Vec<Value>,
    #[serde(default)]
    pub conflicts: Vec<Conflict>,
    #[serde(default)]
    pub warnings: Vec<Warning>,
    #[serde(default)]
    pub review_findings: Vec<ReviewFinding>,
    pub metrics: DiscoveryMetrics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_diff: Option<SchemaDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SchemaCandidate {
    pub id: String,
    pub name: String,
    pub version: String,
    pub format: String,
    pub status: String,
    pub schema: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FieldCandidate {
    pub id: String,
    pub canonical_name: String,
    pub display_name: String,
    pub path: String,
    #[serde(default)]
    pub type_candidates: Vec<TypeCandidate>,
    #[serde(default)]
    pub labels: Vec<String>,
    pub presence: Presence,
    pub required_candidate: bool,
    pub array_candidate: bool,
    pub nullable_candidate: bool,
    #[serde(default)]
    pub examples: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<Evidence>,
    pub confidence: f64,
    pub review_required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_constraints: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TypeCandidate {
    #[serde(rename = "type")]
    pub value_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Presence {
    pub document_count: u32,
    pub present_count: u32,
    pub presence_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Evidence {
    pub document_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounding_box: Option<BoundingBox>,
    pub confidence: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Conflict {
    pub id: String,
    #[serde(rename = "type")]
    pub conflict_type: String,
    pub severity: String,
    pub message: String,
    #[serde(default)]
    pub field_candidate_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Warning {
    pub id: String,
    pub severity: String,
    pub message: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReviewFinding {
    pub id: String,
    pub target: String,
    pub reason: String,
    pub description: String,
    pub suggested_decision: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiscoveryMetrics {
    pub document_count: u32,
    pub field_candidate_count: u32,
    pub average_confidence: f64,
    pub low_confidence_field_count: u32,
    #[serde(default)]
    pub conflict_count: u32,
    #[serde(default)]
    pub review_required_count: u32,
    #[serde(default)]
    pub extraction_success_rate: f64,
    #[serde(default)]
    pub schema_coverage_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SchemaDiff {
    #[serde(default)]
    pub added_fields: Vec<Value>,
    #[serde(default)]
    pub removed_fields: Vec<Value>,
    #[serde(default)]
    pub changed_fields: Vec<Value>,
    #[serde(default)]
    pub renamed_field_candidates: Vec<Value>,
    #[serde(default)]
    pub alias_changes: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportSchemaRequest {
    pub schema_candidate: SchemaCandidate,
    pub format: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<ExportSchemaOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExportSchemaOptions {
    pub include_extensions: bool,
    pub include_descriptions: bool,
    pub include_examples: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExportSchemaResponse {
    pub format: String,
    pub content: String,
}

pub fn discovery_response_schema_json() -> String {
    discovery_response_schema().to_string()
}

pub fn export_schema(request: &ExportSchemaRequest) -> ExportSchemaResponse {
    let content = match request.format.as_str() {
        "markdown_report" => markdown_report(&request.schema_candidate),
        _ => serde_json::to_string_pretty(&request.schema_candidate.schema).unwrap_or_else(|_| "{}".into()),
    };
    ExportSchemaResponse { format: request.format.clone(), content }
}

impl LanguageModelSession {
    pub async fn discover_schema(
        &self,
        request: DiscoverSchemaRequest,
        options: &GenerationOptions,
    ) -> Result<DiscoverSchemaResponse> {
        let prompt = build_discovery_prompt(&request)?;
        let content = self
            .respond_with_json_schema(Prompt::text(prompt), &discovery_response_schema_json(), options)
            .await?;
        let json = content.to_json()?;
        Ok(serde_json::from_str(&json)?)
    }
}

fn build_discovery_prompt(request: &DiscoverSchemaRequest) -> Result<String> {
    Ok(format!(
        "Discover a reusable schema candidate from the provided documents.\n\
         Treat every result as a candidate, not an approved schema.\n\
         Return only data that matches the supplied JSON Schema.\n\
         Preserve evidence only when requested. Mark ambiguous or low-confidence fields for review.\n\
         Do not add raw document text to warnings or logs unless it is evidence requested by the caller.\n\n\
         Request JSON:\n{}",
        serde_json::to_string_pretty(request)?
    ))
}

fn markdown_report(candidate: &SchemaCandidate) -> String {
    format!(
        "# Schema Candidate: {}\n\n- ID: {}\n- Version: {}\n- Format: {}\n- Status: {}\n",
        candidate.name, candidate.id, candidate.version, candidate.format, candidate.status
    )
}

fn discovery_response_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "additionalProperties": false,
        "required": ["schema_candidate", "field_candidates", "document_summaries", "conflicts", "warnings", "review_findings", "metrics"],
        "properties": {
            "schema_candidate": {"type": "object"},
            "field_candidates": {"type": "array", "items": {"type": "object"}},
            "document_summaries": {"type": "array", "items": {"type": "object"}},
            "conflicts": {"type": "array", "items": {"type": "object"}},
            "warnings": {"type": "array", "items": {"type": "object"}},
            "review_findings": {"type": "array", "items": {"type": "object"}},
            "metrics": {"type": "object"},
            "schema_diff": {"type": "object"}
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_uses_snake_case_wire_shape() {
        let request = DiscoverSchemaRequest {
            documents: vec![DiscoveryDocument {
                id: "doc-1".into(),
                source: DiscoveryDocumentSource {
                    source_type: "text".into(),
                    content: Some("請求日 2026-01-01".into()),
                    ..Default::default()
                },
                metadata: None,
            }],
            options: Some(DiscoveryOptions::default()),
            ..Default::default()
        };
        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["documents"][0]["source"]["type"], "text");
        assert!(value["options"]["min_presence_rate_for_required"].is_number());
    }

    #[test]
    fn exports_json_schema_content() {
        let candidate = SchemaCandidate {
            id: "schema-1".into(),
            name: "Invoice".into(),
            version: "0.1.0".into(),
            format: "json_schema".into(),
            status: "candidate".into(),
            schema: json!({"type": "object"}),
            metadata: None,
        };
        let response = export_schema(&ExportSchemaRequest {
            schema_candidate: candidate,
            format: "json_schema".into(),
            options: None,
        });
        assert!(response.content.contains("\"type\": \"object\""));
    }
}
