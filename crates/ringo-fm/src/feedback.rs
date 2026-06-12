//! Feedback attachment support.

use std::ffi::{c_char, CString};

use ringo_fm_sys as sys;
use serde::Serialize;

use crate::error::Result;
use crate::generated::GeneratedContent;
use crate::handle::{check_error, FmString};
use crate::session::LanguageModelSession;
use crate::Error;

/// Overall sentiment for a model response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedbackSentiment {
    None,
    Positive,
    Negative,
    Neutral,
}

impl FeedbackSentiment {
    fn as_ffi(self) -> sys::FMFeedbackSentiment {
        match self {
            FeedbackSentiment::None => sys::FMFeedbackSentiment_FMFeedbackSentimentNone,
            FeedbackSentiment::Positive => sys::FMFeedbackSentiment_FMFeedbackSentimentPositive,
            FeedbackSentiment::Negative => sys::FMFeedbackSentiment_FMFeedbackSentimentNegative,
            FeedbackSentiment::Neutral => sys::FMFeedbackSentiment_FMFeedbackSentimentNeutral,
        }
    }
}

/// Feedback issue category names accepted by FoundationModels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FeedbackIssueCategory {
    Unhelpful,
    TooVerbose,
    DidNotFollowInstructions,
    Incorrect,
    StereotypeOrBias,
    SuggestiveOrSexual,
    VulgarOrOffensive,
    TriggeredGuardrailUnexpectedly,
}

/// A categorized feedback issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeedbackIssue {
    pub category: FeedbackIssueCategory,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
}

/// Options for creating a feedback attachment.
#[derive(Debug)]
pub struct FeedbackAttachmentOptions {
    pub sentiment: FeedbackSentiment,
    pub issues: Vec<FeedbackIssue>,
    pub desired_response_text: Option<String>,
    pub desired_response_content: Option<GeneratedContent>,
}

impl Default for FeedbackAttachmentOptions {
    fn default() -> Self {
        Self {
            sentiment: FeedbackSentiment::None,
            issues: Vec::new(),
            desired_response_text: None,
            desired_response_content: None,
        }
    }
}

impl LanguageModelSession {
    /// Return a FoundationModels feedback attachment payload for this session.
    pub fn log_feedback_attachment(&self, options: &FeedbackAttachmentOptions) -> Result<Vec<u8>> {
        if options.desired_response_text.is_some() && options.desired_response_content.is_some() {
            return Err(Error::Native(
                "feedback attachment: desired response text and content are mutually exclusive"
                    .into(),
            ));
        }
        let issues_json = if options.issues.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&options.issues)?)
        };
        let issues_c = match issues_json {
            Some(s) => Some(CString::new(s).map_err(|e| Error::Native(e.to_string()))?),
            None => None,
        };
        let desired_c = match &options.desired_response_text {
            Some(s) => Some(CString::new(s.as_str()).map_err(|e| Error::Native(e.to_string()))?),
            None => None,
        };
        let desired_content_json = options
            .desired_response_content
            .as_ref()
            .map(GeneratedContent::to_json)
            .transpose()?;
        let desired_content_c = match desired_content_json {
            Some(s) => Some(CString::new(s).map_err(|e| Error::Native(e.to_string()))?),
            None => None,
        };

        let mut len: usize = 0;
        let mut code: i32 = 0;
        let mut desc: *mut c_char = std::ptr::null_mut();
        let ptr = if let Some(desired_content_c) = desired_content_c.as_ref() {
            unsafe {
                sys::FMLanguageModelSessionLogFeedbackAttachmentWithDesiredResponseContent(
                    self.handle.as_ptr(),
                    options.sentiment.as_ffi(),
                    issues_c.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                    desired_content_c.as_ptr(),
                    &mut len,
                    &mut code,
                    &mut desc,
                )
            }
        } else {
            unsafe {
                sys::FMLanguageModelSessionLogFeedbackAttachment(
                    self.handle.as_ptr(),
                    options.sentiment.as_ffi(),
                    issues_c.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                    desired_c.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()),
                    &mut len,
                    &mut code,
                    &mut desc,
                )
            }
        };
        check_error(code, desc)?;
        let _owned = FmString::from_raw(ptr)
            .ok_or_else(|| Error::Native("feedback attachment null".into()))?;
        let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) }.to_vec();
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_feedback_attachment_returns_payload() {
        let session = LanguageModelSession::default().expect("session");
        let payload = session
            .log_feedback_attachment(&FeedbackAttachmentOptions {
                sentiment: FeedbackSentiment::Negative,
                issues: vec![FeedbackIssue {
                    category: FeedbackIssueCategory::Incorrect,
                    explanation: Some("Expected a shorter response.".into()),
                }],
                desired_response_text: Some("A shorter desired response.".into()),
                desired_response_content: None,
            })
            .expect("feedback attachment");
        assert!(!payload.is_empty());
    }

    #[test]
    fn log_feedback_attachment_with_desired_response_content_returns_payload() {
        let session = LanguageModelSession::default().expect("session");
        let content = GeneratedContent::from_json(r#"{"answer":"A concise desired response."}"#)
            .expect("content");
        let payload = session
            .log_feedback_attachment(&FeedbackAttachmentOptions {
                sentiment: FeedbackSentiment::Positive,
                issues: Vec::new(),
                desired_response_text: None,
                desired_response_content: Some(content),
            })
            .expect("feedback attachment");
        assert!(!payload.is_empty());
    }

    #[test]
    fn log_feedback_attachment_rejects_text_and_content() {
        let session = LanguageModelSession::default().expect("session");
        let content = GeneratedContent::from_json(r#"{"answer":"A concise desired response."}"#)
            .expect("content");
        let err = session
            .log_feedback_attachment(&FeedbackAttachmentOptions {
                sentiment: FeedbackSentiment::Positive,
                issues: Vec::new(),
                desired_response_text: Some("A text response.".into()),
                desired_response_content: Some(content),
            })
            .expect_err("expected conflict error");
        assert!(err.to_string().contains("mutually exclusive"));
    }
}
