//! Streaming example — Rust port of `examples/streaming_example.py`.

use std::io::Write;

use apple_fm_sdk::{LanguageModelSession, SystemLanguageModel};
use futures_util::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Streaming Response Example ===\n");

    let model = SystemLanguageModel::default()?;
    let (is_available, reason) = model.is_available();
    if !is_available {
        println!("Model not available: {reason:?}");
        return Ok(());
    }

    let session = LanguageModelSession::new(
        Some(&model),
        Some("You are a helpful assistant."),
        Vec::new(),
    )?;

    let prompt = "Tell me a short story about a cat.";
    println!("User: {prompt}\n");
    print!("Assistant: ");
    std::io::stdout().flush()?;

    let mut last = String::new();
    let mut stream = session.stream(prompt)?;
    while let Some(chunk) = stream.next().await {
        let snapshot = chunk?;
        // The Swift bridge yields cumulative snapshots; print only the new suffix.
        if let Some(delta) = snapshot.strip_prefix(&last) {
            print!("{delta}");
            std::io::stdout().flush()?;
        } else {
            print!("{snapshot}");
            std::io::stdout().flush()?;
        }
        last = snapshot;
    }
    println!("\n");
    Ok(())
}
