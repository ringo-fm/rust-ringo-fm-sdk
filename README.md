# rust-ringo-fm-sdk

Rust bindings for Apple's on-device [Foundation Models](https://developer.apple.com/documentation/foundationmodels) framework.

## Workspace layout

- `crates/apple-fm-sdk-sys` — raw FFI bindings (`bindgen`) to the Swift bridge from `ringo-fm-bridge`.
- `crates/apple-fm-sdk` — idiomatic, async (`tokio`) wrapper. Public surface mirrors the Python package: `SystemLanguageModel`, `LanguageModelSession`, `Prompt`, `GenerationSchema`, `Tool`, `Transcript`, …
- `examples/` — Rust ports of the three Python examples.

## Requirements

- macOS 26+ with Apple Intelligence enabled.
- Full Xcode 26+ install (`xcode-select` pointing at `Xcode.app`, not just the CLI tools).
- The vendored `vendor/foundation-models-c` Swift package included in this repository, or set `APPLE_FM_SDK_SWIFT_PKG` to another compatible `foundation-models-c/` directory.

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
cd vendor/foundation-models-c
swift build -c release

cd ../..
cargo build --workspace
cargo run --example simple_inference
cargo run --example streaming_example
cargo run --example transcript_processing -- path/to/transcript.json
```

The Rust examples automatically add an `rpath` to `vendor/foundation-models-c/.build/release`.
If your Swift package checkout lives elsewhere, set `APPLE_FM_SDK_SWIFT_PKG` to the absolute path
of that `foundation-models-c` directory before building or running.

## Schema Discovery

`LanguageModelSession::discover_schema` uses guided generation to infer a
reviewable schema candidate from text, JSON, or already-extracted documents.
The response includes field candidates, evidence, warnings, metrics, and review
findings; treat it as a draft for human review, not an approved schema.

```rust
use apple_fm_sdk::{
    DiscoveryDocument, DiscoveryDocumentSource, DiscoverSchemaRequest,
    DiscoveryOptions, GenerationOptions, LanguageModelSession,
};

# async fn example() -> apple_fm_sdk::Result<()> {
let session = LanguageModelSession::default()?;
let response = session.discover_schema(
    DiscoverSchemaRequest {
        documents: vec![DiscoveryDocument {
            id: "doc-1".into(),
            source: DiscoveryDocumentSource {
                source_type: "text".into(),
                content: Some("請求日 2026-01-01\n合計 12,000円".into()),
                ..Default::default()
            },
            metadata: None,
        }],
        options: Some(DiscoveryOptions::default()),
        ..Default::default()
    },
    &GenerationOptions::default(),
).await?;
# let _ = response;
# Ok(())
# }
```

## Status

V1 ships everything from the Python public API except:

- `#[derive(Generable)]` proc macro — implement `Generable` manually for now.
- Multi-tool sessions — the C bridge maps every tool call through a single trampoline symbol, so v1 reliably supports **one** registered tool per session. Multi-tool dispatch needs a libffi-style closure trampoline; tracked as a follow-up.

## License

Apache-2.0

## Third-party code

This repository vendors `vendor/foundation-models-c` from `ringo-fm-bridge`, which is
licensed under Apache-2.0. The vendored package retains its original source headers, and a copy of
the bridge license is included at `vendor/foundation-models-c/LICENSE.md`.
