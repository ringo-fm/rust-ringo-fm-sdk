use proptest::prelude::*;
use ringo_fm::{GeneratedContent, LanguageModelSession};

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
    session.prewarm(Some("short prefix")).expect("prewarm short");
    assert!(!session.is_responding());
}

#[test]
fn prewarm_rejects_interior_nul() {
    let session = LanguageModelSession::default().expect("session");
    assert!(session.prewarm(Some("bad\0prefix")).is_err());
}

#[test]
fn generated_content_empty_json() {
    let c = GeneratedContent::from_json("{}").expect("from_json empty object");
    assert!(!c.has_property("anything"));
    let names = c.property_names().expect("property_names");
    assert!(names.is_empty());
}