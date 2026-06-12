//! GeneratedContent tests: typed accessors and entry_count.

use ringo_fm::GeneratedContent;

#[test]
fn generated_content_value_as_f64() {
    let c = GeneratedContent::from_json(r#"{"price": 3.14, "count": 7, "label": "hi"}"#)
        .expect("from_json");
    assert_eq!(c.value_as_f64("price"), Some(3.14));
    assert_eq!(c.value_as_f64("missing"), None);
    assert_eq!(c.value_as_f64("label"), None);
}

#[test]
fn generated_content_value_as_i64() {
    let c = GeneratedContent::from_json(r#"{"count": 7, "price": 3.14, "label": "hi"}"#)
        .expect("from_json");
    assert_eq!(c.value_as_i64("count"), Some(7));
    assert_eq!(c.value_as_i64("missing"), None);
    assert_eq!(c.value_as_i64("label"), None);
}

#[test]
fn generated_content_value_as_bool() {
    let c = GeneratedContent::from_json(r#"{"active": true, "disabled": false, "label": "hi"}"#)
        .expect("from_json");
    assert_eq!(c.value_as_bool("active"), Some(true));
    assert_eq!(c.value_as_bool("disabled"), Some(false));
    assert_eq!(c.value_as_bool("label"), None);
    assert_eq!(c.value_as_bool("missing"), None);
}

#[test]
fn generated_content_has_property() {
    let c = GeneratedContent::from_json(r#"{"x": 1, "y": 2.5}"#).expect("from_json");
    assert!(c.has_property("x"));
    assert!(c.has_property("y"));
    assert!(!c.has_property("z"));
}

#[test]
fn generated_content_property_names() {
    let c =
        GeneratedContent::from_json(r#"{"a": 1, "b": 2, "c": 3}"#).expect("from_json");
    let names = c.property_names().expect("property_names");
    assert_eq!(names.len(), 3);
    assert!(names.contains(&"a".to_string()));
    assert!(names.contains(&"b".to_string()));
    assert!(names.contains(&"c".to_string()));
}

#[test]
fn generated_content_entry_count_new_session() {
    use ringo_fm::LanguageModelSession;
    let session = LanguageModelSession::default().expect("create default session");
    assert_eq!(session.entry_count(), 0);
    let transcript = session.transcript().expect("get transcript");
    assert_eq!(transcript.entry_count(), 0);
}
