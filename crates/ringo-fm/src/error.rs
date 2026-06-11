//! Error types matching `ringo_fm.errors`.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// All errors surfaced by the SDK.
#[derive(Error, Debug)]
pub enum Error {
    #[error("Context window size exceeded: {0}")]
    ExceededContextWindow(String),

    #[error("Required assets are unavailable: {0}")]
    AssetsUnavailable(String),

    #[error("Guardrail violation occurred: {0}")]
    GuardrailViolation(String),

    #[error("Unsupported guide used: {0}")]
    UnsupportedGuide(String),

    #[error("Unsupported language or locale: {0}")]
    UnsupportedLanguageOrLocale(String),

    #[error("Failed to decode response: {0}")]
    DecodingFailure(String),

    #[error("Request was rate limited: {0}")]
    RateLimited(String),

    #[error("Too many concurrent requests: {0}")]
    ConcurrentRequests(String),

    #[error("Model refused to generate content: {0}")]
    Refusal(String),

    #[error("Invalid generation schema provided: {0}")]
    InvalidSchema(String),

    #[error("Tool '{tool}' failed: {message}")]
    ToolCall { tool: String, message: String },

    #[error("Generation error (status {code}): {message}")]
    Generation { code: i32, message: String },

    #[error("Native FFI error: {0}")]
    Native(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Image attachment unsupported on this OS")]
    AttachmentUnsupported,

    #[error("Identified image attachments are not supported by the current native bridge")]
    IdentifiedImageUnsupported,
}

/// Status codes — mirror `GenerationErrorCode` in errors.py.
pub mod status {
    pub const SUCCESS: i32 = 0;
    pub const EXCEEDED_CONTEXT_WINDOW_SIZE: i32 = 1;
    pub const ASSETS_UNAVAILABLE: i32 = 2;
    pub const GUARDRAIL_VIOLATION: i32 = 3;
    pub const UNSUPPORTED_GUIDE: i32 = 4;
    pub const UNSUPPORTED_LANGUAGE_OR_LOCALE: i32 = 5;
    pub const DECODING_FAILURE: i32 = 6;
    pub const RATE_LIMITED: i32 = 7;
    pub const CONCURRENT_REQUESTS: i32 = 8;
    pub const REFUSAL: i32 = 9;
    pub const INVALID_SCHEMA: i32 = 10;
    pub const UNKNOWN: i32 = 255;
}

impl Error {
    pub(crate) fn from_status(code: i32, debug: impl Into<String>) -> Self {
        let debug = debug.into();
        match code {
            status::EXCEEDED_CONTEXT_WINDOW_SIZE => Error::ExceededContextWindow(debug),
            status::ASSETS_UNAVAILABLE => Error::AssetsUnavailable(debug),
            status::GUARDRAIL_VIOLATION => Error::GuardrailViolation(debug),
            status::UNSUPPORTED_GUIDE => Error::UnsupportedGuide(debug),
            status::UNSUPPORTED_LANGUAGE_OR_LOCALE => Error::UnsupportedLanguageOrLocale(debug),
            status::DECODING_FAILURE => Error::DecodingFailure(debug),
            status::RATE_LIMITED => Error::RateLimited(debug),
            status::CONCURRENT_REQUESTS => Error::ConcurrentRequests(debug),
            status::REFUSAL => Error::Refusal(debug),
            status::INVALID_SCHEMA => Error::InvalidSchema(debug),
            other => Error::Generation { code: other, message: debug },
        }
    }
}
