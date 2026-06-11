//! Simple inference example — Rust port of `examples/simple_inference.py`.

use ringo_fm::{LanguageModelSession, SystemLanguageModel};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Simple Inference Example ===\n");

    let model = SystemLanguageModel::default()?;
    let (is_available, reason) = model.is_available();
    if !is_available {
        println!("Model not available: {reason:?}");
        return Ok(());
    }

    let session = LanguageModelSession::new(
        Some(&model),
        Some("You are a helpful assistant that provides concise answers."),
        Vec::new(),
    )?;

    let prompt = "What is the capital of France?";
    println!("User: {prompt}");
    let response = session.respond(prompt).await?;
    println!("Assistant: {response}\n");

    let follow_up = "What is its population?";
    println!("User: {follow_up}");
    let response = session.respond(follow_up).await?;
    println!("Assistant: {response}\n");

    Ok(())
}
