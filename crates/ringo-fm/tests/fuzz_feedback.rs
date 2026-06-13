use ringo_fm::{FeedbackAttachmentOptions, FeedbackSentiment, LanguageModelSession};

#[cfg(test)]
mod feedback_fuzz {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn fuzz_feedback_sentiment_roundtrip(
            sentiment_val in 0u32..5,
        ) {
            let session = LanguageModelSession::default().expect("session");
            let sentiment = match sentiment_val {
                0 => FeedbackSentiment::None,
                1 => FeedbackSentiment::Positive,
                2 => FeedbackSentiment::Negative,
                3 => FeedbackSentiment::Neutral,
                _ => FeedbackSentiment::None,
            };
            let result = session.log_feedback_attachment(&FeedbackAttachmentOptions {
                sentiment,
                issues: vec![],
                desired_response_text: None,
                desired_response_content: None,
            });
            let _ = result;
        }

        #[test]
        fn fuzz_feedback_with_desired_text(
            text in ".*",
        ) {
            let session = LanguageModelSession::default().expect("session");
            let result = session.log_feedback_attachment(&FeedbackAttachmentOptions {
                sentiment: FeedbackSentiment::Positive,
                issues: vec![],
                desired_response_text: if text.is_empty() { None } else { Some(text) },
                desired_response_content: None,
            });
            let _ = result;
        }
    }
}

#[test]
fn feedback_rejects_text_and_content_conflict() {
    let session = LanguageModelSession::default().expect("session");
    let content = ringo_fm::GeneratedContent::from_json(r#"{"a":"b"}"#).expect("content");
    let result = session.log_feedback_attachment(&FeedbackAttachmentOptions {
        sentiment: FeedbackSentiment::Positive,
        issues: vec![],
        desired_response_text: Some("text".into()),
        desired_response_content: Some(content),
    });
    assert!(result.is_err());
}
