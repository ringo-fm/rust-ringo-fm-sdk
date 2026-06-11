//! `SystemLanguageModel` — the on-device foundation model handle.

use apple_fm_sdk_sys as sys;

use crate::error::Result;
use crate::handle::ManagedRef;

/// Tagging marker for the [`ManagedRef`] holding an `FMSystemLanguageModelRef`.
pub(crate) struct SystemModelTag;

/// What the model is tuned for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum UseCase {
    General = 0,
    ContentTagging = 1,
}

/// Safety/guardrail configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Guardrails {
    Default = 0,
    PermissiveContentTransformations = 1,
}

/// Reasons the model may be unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnavailableReason {
    AppleIntelligenceNotEnabled,
    DeviceNotEligible,
    ModelNotReady,
    Unknown,
}

impl UnavailableReason {
    fn from_raw(raw: u32) -> Self {
        match raw {
            0 => Self::AppleIntelligenceNotEnabled,
            1 => Self::DeviceNotEligible,
            2 => Self::ModelNotReady,
            _ => Self::Unknown,
        }
    }
}

/// On-device system language model.
pub struct SystemLanguageModel {
    pub(crate) handle: ManagedRef<SystemModelTag>,
}

impl SystemLanguageModel {
    /// Default system model (equivalent to `SystemLanguageModel()` in Python).
    pub fn default() -> Result<Self> {
        let ptr = unsafe { sys::FMSystemLanguageModelGetDefault() };
        Ok(Self { handle: ManagedRef::from_owned(ptr)? })
    }

    /// Configured system model.
    pub fn new(use_case: UseCase, guardrails: Guardrails) -> Result<Self> {
        let ptr = unsafe {
            sys::FMSystemLanguageModelCreate(use_case as sys::FMSystemLanguageModelUseCase,
                                             guardrails as sys::FMSystemLanguageModelGuardrails)
        };
        Ok(Self { handle: ManagedRef::from_owned(ptr)? })
    }

    /// Check availability. Returns `Ok(())` when the model can be used,
    /// or `Err(reason)` describing why it cannot.
    pub fn availability(&self) -> std::result::Result<(), UnavailableReason> {
        let mut reason: sys::FMSystemLanguageModelUnavailableReason = 0;
        let ok = unsafe { sys::FMSystemLanguageModelIsAvailable(self.handle.as_ptr(), &mut reason) };
        if ok {
            Ok(())
        } else {
            Err(UnavailableReason::from_raw(reason as u32))
        }
    }

    /// Convenience: `(is_available, reason_if_unavailable)`, matching the Python tuple return.
    pub fn is_available(&self) -> (bool, Option<UnavailableReason>) {
        match self.availability() {
            Ok(()) => (true, None),
            Err(r) => (false, Some(r)),
        }
    }
}

impl std::fmt::Debug for SystemLanguageModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemLanguageModel").finish_non_exhaustive()
    }
}

