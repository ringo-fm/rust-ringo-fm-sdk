//! RAII wrappers around the C-managed opaque pointers.
//!
//! Every `FM*Ref` in the C ABI is reference-counted via `FMRetain`/`FMRelease`,
//! and every `char*` returned by the C side must be freed with `FMFreeString`.

use std::ffi::{c_char, c_void, CStr};
use std::marker::PhantomData;
use std::ptr::NonNull;

use ringo_fm_sys as sys;

use crate::error::{Error, Result};

/// RAII handle to an FM-reference-counted opaque object.
///
/// On drop, `FMRelease` is called once. Construct via [`ManagedRef::from_owned`]
/// for objects whose ownership is being transferred to us (the common case for
/// `FM*Create`/`FM*GetDefault` returns), or [`ManagedRef::retain`] when we are
/// taking shared ownership of a borrowed reference.
pub(crate) struct ManagedRef<T> {
    ptr: NonNull<c_void>,
    _t: PhantomData<T>,
}

impl<T> ManagedRef<T> {
    /// Wrap a pointer whose +1 reference count was transferred to us by the C side.
    pub fn from_owned(ptr: *const c_void) -> Result<Self> {
        NonNull::new(ptr as *mut c_void)
            .map(|p| Self { ptr: p, _t: PhantomData })
            .ok_or_else(|| Error::Native("null pointer from FFI".into()))
    }

    /// Take shared ownership of a borrowed pointer by retaining it.
    #[allow(dead_code)]
    pub fn retain(ptr: *const c_void) -> Result<Self> {
        let p = NonNull::new(ptr as *mut c_void)
            .ok_or_else(|| Error::Native("null pointer from FFI".into()))?;
        unsafe { sys::FMRetain(p.as_ptr()) };
        Ok(Self { ptr: p, _t: PhantomData })
    }

    pub fn as_ptr(&self) -> *const c_void {
        self.ptr.as_ptr()
    }
}

impl<T> Drop for ManagedRef<T> {
    fn drop(&mut self) {
        unsafe { sys::FMRelease(self.ptr.as_ptr()) };
    }
}

// All FM objects are documented as Sendable on the Swift side (`@Sendable` attributes).
unsafe impl<T: Send> Send for ManagedRef<T> {}
unsafe impl<T: Sync> Sync for ManagedRef<T> {}

/// Wrap a `char*` returned by the C side; on drop calls `FMFreeString`.
pub(crate) struct FmString {
    ptr: *mut c_char,
}

impl FmString {
    /// Build from a `*mut char*` out-pointer; returns `None` if the pointer is null.
    pub fn from_raw(ptr: *mut c_char) -> Option<Self> {
        if ptr.is_null() { None } else { Some(Self { ptr }) }
    }

    pub fn to_string(&self) -> Result<String> {
        let cstr = unsafe { CStr::from_ptr(self.ptr) };
        cstr.to_str()
            .map(|s| s.to_owned())
            .map_err(|e| Error::Native(format!("invalid utf8 in C string: {e}")))
    }
}

impl Drop for FmString {
    fn drop(&mut self) {
        unsafe { sys::FMFreeString(self.ptr) };
    }
}

/// Read an optional `(int* outErrorCode, char** outErrorDescription)` pair from a C call
/// and turn it into a `Result<()>`.
pub(crate) fn check_error(code: i32, desc_ptr: *mut c_char) -> Result<()> {
    if code == crate::error::status::SUCCESS {
        if !desc_ptr.is_null() {
            // Defensive: free any spurious description on success.
            drop(FmString::from_raw(desc_ptr));
        }
        return Ok(());
    }
    let message = FmString::from_raw(desc_ptr)
        .and_then(|s| s.to_string().ok())
        .unwrap_or_default();
    Err(Error::from_status(code, message))
}
