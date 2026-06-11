//! Generation options & sampling modes.

use serde::Serialize;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingModeKind {
    Greedy,
    Random,
}

impl SamplingModeKind {
    fn as_str(self) -> &'static str {
        match self {
            SamplingModeKind::Greedy => "greedy",
            SamplingModeKind::Random => "random",
        }
    }
}

/// Sampling strategy.
#[derive(Debug, Clone, Default)]
pub struct SamplingMode {
    pub kind: Option<SamplingModeKind>,
    pub top: Option<u32>,
    pub probability_threshold: Option<f64>,
    pub seed: Option<u64>,
}

impl SamplingMode {
    pub fn greedy() -> Self {
        Self { kind: Some(SamplingModeKind::Greedy), ..Default::default() }
    }

    /// Random sampling. Either `top` (top-k) or `probability_threshold` may be set, not both.
    pub fn random(top: Option<u32>, probability_threshold: Option<f64>, seed: Option<u64>) -> Result<Self> {
        if top.is_some() && probability_threshold.is_some() {
            return Err(Error::Native(
                "Cannot specify both 'top' and 'probability_threshold'".into(),
            ));
        }
        if let Some(p) = probability_threshold {
            if !(0.0..=1.0).contains(&p) {
                return Err(Error::Native("'probability_threshold' must be in [0.0, 1.0]".into()));
            }
        }
        Ok(Self {
            kind: Some(SamplingModeKind::Random),
            top,
            probability_threshold,
            seed,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct GenerationOptions {
    pub sampling: Option<SamplingMode>,
    pub temperature: Option<f64>,
    pub maximum_response_tokens: Option<u32>,
}

impl GenerationOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_temperature(mut self, t: f64) -> Self {
        self.temperature = Some(t);
        self
    }

    pub fn with_maximum_response_tokens(mut self, n: u32) -> Self {
        self.maximum_response_tokens = Some(n);
        self
    }

    pub fn with_sampling(mut self, s: SamplingMode) -> Self {
        self.sampling = Some(s);
        self
    }

    /// Render to the JSON form expected by the C bridge (mirrors `to_dict` in Python).
    pub(crate) fn to_json(&self) -> Result<Option<String>> {
        if self.sampling.is_none() && self.temperature.is_none() && self.maximum_response_tokens.is_none() {
            return Ok(None);
        }
        #[derive(Serialize)]
        struct Sampling<'a> {
            mode: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            top_k: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            top_p: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            seed: Option<String>,
        }
        #[derive(Serialize, Default)]
        struct Opts<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            sampling: Option<Sampling<'a>>,
            #[serde(skip_serializing_if = "Option::is_none")]
            temperature: Option<f64>,
            #[serde(skip_serializing_if = "Option::is_none")]
            maximum_response_tokens: Option<u32>,
        }

        let sampling = self.sampling.as_ref().and_then(|s| {
            let kind = s.kind?;
            let (top_k, top_p, seed) = if kind == SamplingModeKind::Random {
                (
                    s.top.map(|v| v.to_string()),
                    s.probability_threshold.map(|v| v.to_string()),
                    s.seed.map(|v| v.to_string()),
                )
            } else {
                (None, None, None)
            };
            Some(Sampling { mode: kind.as_str(), top_k, top_p, seed })
        });
        let opts = Opts {
            sampling,
            temperature: self.temperature,
            maximum_response_tokens: self.maximum_response_tokens,
        };
        Ok(Some(serde_json::to_string(&opts)?))
    }
}
