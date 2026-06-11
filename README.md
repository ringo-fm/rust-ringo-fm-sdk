# rust-ringo-fm-sdk

Rust bindings for Apple's on-device [Foundation Models](https://developer.apple.com/documentation/foundationmodels) framework. Ported from [`python-apple-fm-sdk`](../python-apple-fm-sdk).

## Workspace layout

- `crates/apple-fm-sdk-sys` — raw FFI bindings (`bindgen`) to the Swift bridge from `python-apple-fm-sdk/foundation-models-c`.
- `crates/apple-fm-sdk` — idiomatic, async (`tokio`) wrapper. Public surface mirrors the Python package: `SystemLanguageModel`, `LanguageModelSession`, `Prompt`, `GenerationSchema`, `Tool`, `Transcript`, …
- `examples/` — Rust ports of the three Python examples.

## Requirements

- macOS 26+ with Apple Intelligence enabled.
- Full Xcode 26+ install (`xcode-select` pointing at `Xcode.app`, not just the CLI tools).
- The sibling `python-apple-fm-sdk` checkout at `../python-apple-fm-sdk`, or set `APPLE_FM_SDK_SWIFT_PKG` to the absolute path of its `foundation-models-c/` directory.

## Quick start

```rust
use apple_fm_sdk::{LanguageModelSession, SystemLanguageModel};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = SystemLanguageModel::default()?;
    let (ok, reason) = model.is_available();
    if !ok { eprintln!("model unavailable: {reason:?}"); return Ok(()); }

    let session = LanguageModelSession::new(
        Some(&model),
        Some("You are concise."),
        Vec::new(),
    )?;
    let response = session.respond("What is the capital of France?").await?;
    println!("{response}");
    Ok(())
}
```

## Build & run

```bash
cargo build --workspace
cargo run --example simple_inference
cargo run --example streaming_example
cargo run --example transcript_processing -- path/to/transcript.json
```

## Status

V1 ships everything from the Python public API except:

- `#[derive(Generable)]` proc macro — implement `Generable` manually for now.
- Multi-tool sessions — the C bridge maps every tool call through a single trampoline symbol, so v1 reliably supports **one** registered tool per session. Multi-tool dispatch needs a libffi-style closure trampoline; tracked as a follow-up.

## License

Apache-2.0
