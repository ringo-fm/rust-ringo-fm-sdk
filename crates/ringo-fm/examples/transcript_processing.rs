//! Transcript processing example — Rust port of `examples/transcript_processing.py`.
//!
//! Loads a transcript JSON exported from a Swift app and reports basic stats.
//! Usage: `cargo run --example transcript_processing -- path/to/transcript.json`

use ringo_fm::{Transcript, TranscriptEntry};
use serde_json::Value;
use std::env;
use std::fs;

fn extract_text(contents: &[Value]) -> String {
    contents
        .iter()
        .filter_map(|c| match c.get("type").and_then(|t| t.as_str()) {
            Some("text") => c.get("text").and_then(|t| t.as_str()).map(str::to_owned),
            Some("structure") => c
                .get("structure")
                .and_then(|s| s.get("content"))
                .map(|v| v.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args().nth(1).expect("usage: transcript_processing <path>");
    let json = fs::read_to_string(&path)?;
    let transcript = Transcript::from_json(&json)?;

    let entries = transcript.entries();
    let (mut instructions, mut user, mut response, mut tool) = (0, 0, 0, 0);
    let (mut user_chars, mut response_chars) = (0usize, 0usize);

    for entry in &entries {
        match entry {
            TranscriptEntry::Instructions(_) => instructions += 1,
            TranscriptEntry::User(v) => {
                user += 1;
                if let Some(arr) = v.get("contents").and_then(Value::as_array) {
                    user_chars += extract_text(arr).len();
                }
            }
            TranscriptEntry::Response(v) => {
                response += 1;
                if let Some(arr) = v.get("contents").and_then(Value::as_array) {
                    response_chars += extract_text(arr).len();
                }
            }
            TranscriptEntry::Tool(_) => tool += 1,
            TranscriptEntry::Other { .. } => {}
        }
    }

    println!("============================================================");
    println!("TRANSCRIPT SUMMARY");
    println!("============================================================");
    println!("Total entries: {}", entries.len());
    println!("  Instructions: {instructions}");
    println!("  User:         {user}");
    println!("  Response:     {response}");
    println!("  Tool:         {tool}");
    println!("Total user chars:     {user_chars}");
    println!("Total response chars: {response_chars}");
    Ok(())
}
