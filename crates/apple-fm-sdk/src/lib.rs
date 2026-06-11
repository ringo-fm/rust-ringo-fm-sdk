//! Idiomatic, async Rust bindings for Apple's on-device Foundation Models framework.
//!
//! This crate is a Rust port of the `apple_fm_sdk` Python package and links to the
//! same Swift/C bridge (`FoundationModelsCBindings`) through the [`apple_fm_sdk_sys`] crate.
//!
//! Requires macOS 26+ with Apple Intelligence enabled.

pub mod error;
pub(crate) mod handle;
pub mod model;
pub mod prompt;
pub mod options;
pub mod schema;
pub mod schema_discovery;
pub mod generated;
pub mod generable;
pub mod session;
pub mod stream;
pub mod tool;
pub mod transcript;

pub use error::{Error, Result};
pub use generable::Generable;
pub use generated::GeneratedContent;
pub use model::{Guardrails, SystemLanguageModel, UnavailableReason, UseCase};
pub use options::{GenerationOptions, SamplingMode};
pub use prompt::{Attachment, ImageAttachment, Prompt};
pub use schema::{GenerationGuide, GenerationSchema, GenerationSchemaProperty};
pub use schema_discovery::*;
pub use session::LanguageModelSession;
pub use stream::ResponseStream;
pub use tool::Tool;
pub use transcript::{Transcript, TranscriptEntry};
