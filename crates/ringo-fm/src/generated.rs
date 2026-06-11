//! `GeneratedContent` — opaque parsed structured-generation output.

use std::ffi::CString;

use ringo_fm_sys as sys;
use serde::de::DeserializeOwned;

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

    /// Deserialize the content into any `serde::DeserializeOwned` type.
    pub fn decode<T: DeserializeOwned>(&self) -> Result<T> {
        let json = self.to_json()?;
        serde_json::from_str(&json).map_err(|e| Error::Native(e.to_string()))
    }

    /// Parse the content as a generic JSON value.
    pub fn as_value(&self) -> Result<serde_json::Value> {
        let json = self.to_json()?;
        serde_json::from_str(&json).map_err(|e| Error::Native(e.to_string()))
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

    /// Returns the value of a numeric property as `f64`, or `None` if absent or wrong type.
    pub fn value_as_f64(&self, name: &str) -> Option<f64> {
        let c = CString::new(name).ok()?;
        let mut out: f64 = 0.0;
        let mut code: i32 = 0;
        let ok = unsafe {
            sys::FMGeneratedContentGetPropertyValueAsDouble(
                self.handle.as_ptr(),
                c.as_ptr(),
                &mut out,
                &mut code,
            )
        };
        if ok { Some(out) } else { None }
    }

    /// Returns the value of an integer property as `i64`, or `None` if absent or wrong type.
    pub fn value_as_i64(&self, name: &str) -> Option<i64> {
        let c = CString::new(name).ok()?;
        let mut out: i64 = 0;
        let mut code: i32 = 0;
        let ok = unsafe {
            sys::FMGeneratedContentGetPropertyValueAsInt(
                self.handle.as_ptr(),
                c.as_ptr(),
                &mut out,
                &mut code,
            )
        };
        if ok { Some(out) } else { None }
    }

    /// Returns the value of a boolean property, or `None` if absent or wrong type.
    pub fn value_as_bool(&self, name: &str) -> Option<bool> {
        let c = CString::new(name).ok()?;
        let mut out: bool = false;
        let mut code: i32 = 0;
        let ok = unsafe {
            sys::FMGeneratedContentGetPropertyValueAsBool(
                self.handle.as_ptr(),
                c.as_ptr(),
                &mut out,
                &mut code,
            )
        };
        if ok { Some(out) } else { None }
    }

    /// Reports whether the content has a top-level property with `name`.
    pub fn has_property(&self, name: &str) -> bool {
        let Ok(c) = CString::new(name) else { return false };
        unsafe { sys::FMGeneratedContentHasProperty(self.handle.as_ptr(), c.as_ptr()) }
    }

    /// Returns the sorted list of top-level property names.
    pub fn property_names(&self) -> Result<Vec<String>> {
        let ptr = unsafe { sys::FMGeneratedContentGetPropertyNames(self.handle.as_ptr()) };
        let json = FmString::from_raw(ptr)
            .ok_or_else(|| Error::Native("property names null".into()))?
            .to_string()?;
        serde_json::from_str::<Vec<String>>(&json).map_err(|e| Error::Native(e.to_string()))
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
