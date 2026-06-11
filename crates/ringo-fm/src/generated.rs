//! `GeneratedContent` — opaque parsed structured-generation output.

use std::ffi::CString;

use ringo_fm_sys as sys;

use crate::error::{Error, Result};
use crate::handle::{check_error, FmString, ManagedRef};

pub(crate) struct GeneratedContentTag;

/// Parsed structured-generation output. Backed by a JSON document on the C side.
pub struct GeneratedContent {
    pub(crate) handle: ManagedRef<GeneratedContentTag>,
}

impl GeneratedContent {
    pub fn from_json(json: &str) -> Result<Self> {
        let c = CString::new(json).map_err(|e| Error::Native(e.to_string()))?;
        let mut code: i32 = 0;
        let mut desc: *mut i8 = std::ptr::null_mut();
        let ptr = unsafe { sys::FMGeneratedContentCreateFromJSON(c.as_ptr(), &mut code, &mut desc) };
        check_error(code, desc)?;
        Ok(Self { handle: ManagedRef::from_owned(ptr)? })
    }

    pub fn to_json(&self) -> Result<String> {
        let ptr = unsafe { sys::FMGeneratedContentGetJSONString(self.handle.as_ptr()) };
        FmString::from_raw(ptr)
            .ok_or_else(|| Error::Native("generated content JSON null".into()))?
            .to_string()
    }

    pub fn get_property(&self, name: &str) -> Result<String> {
        let c = CString::new(name).map_err(|e| Error::Native(e.to_string()))?;
        let mut code: i32 = 0;
        let mut desc: *mut i8 = std::ptr::null_mut();
        let ptr = unsafe {
            sys::FMGeneratedContentGetPropertyValue(self.handle.as_ptr(), c.as_ptr(), &mut code, &mut desc)
        };
        check_error(code, desc)?;
        FmString::from_raw(ptr)
            .ok_or_else(|| Error::Native("property value null".into()))?
            .to_string()
    }

    pub fn is_complete(&self) -> bool {
        unsafe { sys::FMGeneratedContentIsComplete(self.handle.as_ptr()) }
    }
}

impl std::fmt::Debug for GeneratedContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.to_json() {
            Ok(j) => write!(f, "GeneratedContent({j})"),
            Err(_) => f.debug_struct("GeneratedContent").finish_non_exhaustive(),
        }
    }
}
