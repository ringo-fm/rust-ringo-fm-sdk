use proptest::prelude::*;
use ringo_fm::{
    export_schema, DiscoverSchemaRequest, DiscoveryDocument, DiscoveryDocumentSource,
    DiscoveryHints, DiscoveryOptions, ExportSchemaRequest, GeneratedContent, GenerationOptions,
    GenerationSchema, GenerationSchemaProperty, LanguageModelSession, SamplingMode,
    SchemaCandidate,
};
use serde_json::json;

proptest! {
    #[test]
    fn fuzz_generated_content_from_json(json_str in ".*") {
        let result = GeneratedContent::from_json(&json_str);
        if let Ok(content) = result {
            let _ = content.to_json();
            let _ = content.value_as_f64("x");
            let _ = content.value_as_i64("x");
            let _ = content.value_as_bool("x");
            let _ = content.has_property("x");
            let _ = content.has_property("");
            let _ = content.is_complete();
            let _ = content.property_names();
        }
    }

    #[test]
    fn fuzz_generated_content_property_access(
        json_str in r#"\{[a-z]{1,3}:(1|true|"x"|3\.14|null)\}"#,
        prop_name in "[a-z]{1,10}",
    ) {
        if let Ok(content) = GeneratedContent::from_json(&json_str) {
            let _ = content.has_property(&prop_name);
            let _ = content.value_as_f64(&prop_name);
            let _ = content.value_as_i64(&prop_name);
            let _ = content.value_as_bool(&prop_name);
        }
    }

    #[test]
    fn fuzz_transcript_from_json(json_str in ".*") {
        let result = LanguageModelSession::from_transcript(&json_str, None, Vec::new());
        if let Ok(session) = result {
            let _ = session.entry_count();
            if let Ok(transcript) = session.transcript() {
                let _ = transcript.entry_count();
                let _ = transcript.to_json();
            }
        }
    }

    #[test]
    fn fuzz_schema_creation(
        name in "[a-zA-Z]{1,20}",
        desc in "[a-zA-Z ]{0,50}",
        type_name in prop_oneof![
            Just("String"),
            Just("Int"),
            Just("Double"),
            Just("Bool"),
            Just("Array<String>"),
            Just("[a-z]{1,10}"),
        ],
    ) {
        use ringo_fm::{GenerationGuide, GenerationSchema, GenerationSchemaProperty};

if let Ok(mut schema) = GenerationSchema::new(&name, if desc.is_empty() { None } else { Some(&desc) }) {
                if let Ok(prop) = GenerationSchemaProperty::new("field1", None, &type_name, true) {
                    if let Ok(prop) = prop.add_guide(&GenerationGuide::any_of(vec!["a".to_string()])) {
                        schema.add_property(prop);
                    }
                }
            let _ = schema.to_json();
        }
    }

    #[test]
    fn fuzz_schema_with_regex(
        pattern in ".*",
        type_name in prop_oneof![Just("String"), Just("Int")],
    ) {
        use ringo_fm::{GenerationGuide, GenerationSchema, GenerationSchemaProperty};

        if let Ok(mut schema) = GenerationSchema::new("FuzzSchema", Some("fuzz test")) {
            if let Ok(prop) = GenerationSchemaProperty::new("field1", None, &type_name, false) {
                if let Ok(prop) = prop.add_guide(&GenerationGuide::regex(&pattern)) {
                    schema.add_property(prop);
                }
            }
        }
    }

    #[test]
    fn fuzz_schema_guide_families(
        name in ".{0,20}",
        type_name in ".{0,20}",
        choice in ".{0,20}",
        pattern in ".*",
        count in -3i32..8,
        min in -1000.0f64..1000.0,
        max in -1000.0f64..1000.0,
    ) {
        use ringo_fm::GenerationGuide;

        if let Ok(mut schema) = GenerationSchema::new("GuideFuzzSchema", None) {
            if let Ok(prop) = GenerationSchemaProperty::new(&name, Some("fuzz"), &type_name, count % 2 == 0) {
                let guides = [
                    GenerationGuide::any_of(vec![choice.clone(), String::new()]),
                    GenerationGuide::constant(choice.clone()),
                    GenerationGuide::count(count),
                    GenerationGuide::min_items(count),
                    GenerationGuide::max_items(count + 1),
                    GenerationGuide::minimum(min),
                    GenerationGuide::maximum(max),
                    GenerationGuide::range(min, max),
                    GenerationGuide::regex(pattern.clone()),
                    GenerationGuide::element(GenerationGuide::regex(pattern)),
                ];
                let mut maybe_prop = Some(prop);
                for guide in guides {
                    if let Some(prop) = maybe_prop.take() {
                        maybe_prop = prop.add_guide(&guide).ok();
                    }
                }
                if let Some(prop) = maybe_prop {
                    schema.add_property(prop);
                }
            }
            if let Ok(reference) = GenerationSchema::new("ReferenceSchema", Some("nested")) {
                schema.add_reference_schema(&reference);
            }
            let _ = schema.to_json();
        }
    }

    #[test]
    fn fuzz_sampling_mode_validation(
        top in 0u32..100,
        probability_threshold in -1.0f64..2.0,
        seed in any::<u64>(),
        use_top in any::<bool>(),
        use_probability in any::<bool>(),
    ) {
        let top = use_top.then_some(top);
        let probability_threshold = use_probability.then_some(probability_threshold);
        let result = SamplingMode::random(top, probability_threshold, Some(seed));
        if use_top && use_probability {
            prop_assert!(result.is_err());
        } else if let Some(p) = probability_threshold {
            prop_assert_eq!(result.is_ok(), (0.0..=1.0).contains(&p));
        }
    }

    #[test]
    fn fuzz_schema_discovery_wire_json(
        id in ".{0,20}",
        source_type in ".{0,12}",
        content in ".*",
        language in ".{0,8}",
    ) {
        let request = DiscoverSchemaRequest {
            documents: vec![DiscoveryDocument {
                id,
                source: DiscoveryDocumentSource {
                    source_type,
                    media_type: Some("text/plain".into()),
                    name: Some("fuzz.txt".into()),
                    content: Some(content),
                    uri: None,
                },
                metadata: Some(json!({"kind": "fuzz"})),
            }],
            hints: Some(DiscoveryHints {
                language: Some(language),
                domain: Some("fuzz".into()),
                ..Default::default()
            }),
            options: Some(DiscoveryOptions::default()),
            existing_schema: Some(json!({"type": "object"})),
        };
        let value = serde_json::to_value(&request)?;
        prop_assert!(value["documents"].is_array());
        prop_assert_eq!(&value["documents"][0]["source"]["media_type"], "text/plain");
    }
}

proptest! {
    #[test]
    fn fuzz_export_schema(
        format in prop_oneof![Just("json_schema"), Just("openapi_schema"), Just("markdown_report"), Just("unknown")],
        name in ".{0,32}",
        property in "[a-z]{0,12}",
    ) {
        let candidate = SchemaCandidate {
            id: "fuzz-schema".into(),
            name,
            version: "0.0.0".into(),
            format: "json_schema".into(),
            status: "candidate".into(),
            schema: json!({
                "type": "object",
                "properties": {
                    property: {"type": "string"}
                }
            }),
            metadata: None,
        };
        let response = export_schema(&ExportSchemaRequest {
            schema_candidate: candidate,
            format: format.to_string(),
            options: None,
        });
        prop_assert_eq!(response.format, format);
        prop_assert!(!response.content.is_empty());
    }
}

#[test]
fn after_close_safety() {
    let c = GeneratedContent::from_json(r#"{"x":1}"#).expect("from_json");

    assert_eq!(c.value_as_i64("x"), Some(1));
    assert!(c.has_property("x"));

    drop(c);

    let session = LanguageModelSession::default().expect("session");
    let count = session.entry_count();
    assert_eq!(count, 0);
}

#[test]
fn double_close_safety() {
    let c = GeneratedContent::from_json(r#"{"x":1}"#).expect("from_json");
    drop(c);
}

#[test]
fn prewarm_edge_cases() {
    let session = LanguageModelSession::default().expect("session");
    session.prewarm(None).expect("prewarm none");
    session.prewarm(Some("")).expect("prewarm empty");
    session
        .prewarm(Some("short prefix"))
        .expect("prewarm short");
    assert!(!session.is_responding());
}

#[test]
fn prewarm_rejects_interior_nul() {
    let session = LanguageModelSession::default().expect("session");
    assert!(session.prewarm(Some("bad\0prefix")).is_err());
}

#[test]
fn generation_options_builder_accepts_extreme_public_values() {
    let random = SamplingMode::random(Some(1), None, Some(u64::MAX)).expect("random sampling");
    let _ = GenerationOptions::new()
        .with_temperature(f64::MAX)
        .with_maximum_response_tokens(u32::MAX)
        .with_sampling(random);

    assert!(SamplingMode::random(Some(1), Some(0.5), None).is_err());
    assert!(SamplingMode::random(None, Some(-0.1), None).is_err());
    assert!(SamplingMode::random(None, Some(1.1), None).is_err());
}

#[test]
fn generated_content_empty_json() {
    let c = GeneratedContent::from_json("{}").expect("from_json empty object");
    assert!(!c.has_property("anything"));
    let names = c.property_names().expect("property_names");
    assert!(names.is_empty());
}
