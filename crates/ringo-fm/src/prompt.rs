//! Prompt builder — composes text and image attachments into an `FMComposedPrompt`.

use std::ffi::CString;
use std::ffi::c_void;
use std::path::PathBuf;

use ringo_fm_sys as sys;

use crate::error::{Error, Result};

/// A composable prompt.
///
/// Quick path: `Prompt::text("Hello")` or `Prompt::from("Hello")`.
/// Builder path: `Prompt::builder().text("..").image("photo.png").build()`.
#[derive(Debug, Clone, Default)]
pub struct Prompt {
    components: Vec<Component>,
}

#[derive(Debug, Clone)]
enum Component {
    Text(String),
    Image(ImageAttachment),
    Attachment(Attachment),
}

/// An image attached to a prompt.
#[derive(Debug, Clone)]
pub struct ImageAttachment {
    pub path: PathBuf,
    /// Optional identifier referenced from the text.
    pub identifier: Option<String>,
}

impl ImageAttachment {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into(), identifier: None }
    }
    pub fn identified(path: impl Into<PathBuf>, identifier: impl Into<String>) -> Self {
        Self { path: path.into(), identifier: Some(identifier.into()) }
    }
}

/// A generic attachment with an optional label (macOS 27+ only).
#[derive(Debug, Clone)]
pub struct Attachment {
    pub path: PathBuf,
    pub label: Option<String>,
}

impl Attachment {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into(), label: None }
    }
    pub fn labeled(path: impl Into<PathBuf>, label: impl Into<String>) -> Self {
        Self { path: path.into(), label: Some(label.into()) }
    }
}

impl Prompt {
    pub fn builder() -> Self {
        Self::default()
    }

    pub fn text(text: impl Into<String>) -> Self {
        Self { components: vec![Component::Text(text.into())] }
    }

    pub fn add_text(mut self, text: impl Into<String>) -> Self {
        self.components.push(Component::Text(text.into()));
        self
    }

    pub fn add_image(mut self, image: ImageAttachment) -> Self {
        self.components.push(Component::Image(image));
        self
    }

    pub fn add_attachment(mut self, attachment: Attachment) -> Self {
        self.components.push(Component::Attachment(attachment));
        self
    }

    /// Build the FFI-side composed prompt. The returned raw pointer is owned by the
    /// caller and must be passed straight to one of the `FMLanguageModelSession*` C
    /// functions, which take ownership.
    pub(crate) fn into_composed(self) -> Result<ComposedPrompt> {
        let ptr = unsafe { sys::FMComposedPromptInitialize() };
        let composed = ComposedPrompt { ptr };
        for component in self.components {
            match component {
                Component::Text(t) => {
                    let cstr = CString::new(t).map_err(|e| Error::Native(e.to_string()))?;
                    unsafe { sys::FMComposedPromptAddText(composed.ptr, cstr.as_ptr()) };
                }
                Component::Image(img) => {
                    if img.identifier.is_some() {
                        return Err(Error::IdentifiedImageUnsupported);
                    }
                    let path = path_to_cstring(&img.path)?;
                    let mut err: sys::FMComposedPromptAddImageError = 0;
                    let ok = unsafe {
                        sys::FMComposedPromptAddAttachment(
                            composed.ptr,
                            path.as_ptr(),
                            std::ptr::null(),
                            &mut err,
                        )
                    };
                    if !ok {
                        return Err(map_image_error(err));
                    }
                }
                Component::Attachment(att) => {
                    let path = path_to_cstring(&att.path)?;
                    let label_c = match att.label {
                        Some(l) => Some(CString::new(l).map_err(|e| Error::Native(e.to_string()))?),
                        None => None,
                    };
                    let label_ptr = label_c.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
                    let mut err: sys::FMComposedPromptAddImageError = 0;
                    let ok = unsafe {
                        sys::FMComposedPromptAddAttachment(composed.ptr, path.as_ptr(), label_ptr, &mut err)
                    };
                    if !ok {
                        return Err(map_image_error(err));
                    }
                }
            }
        }
        Ok(composed)
    }
}

impl From<&str> for Prompt {
    fn from(s: &str) -> Self {
        Prompt::text(s)
    }
}

impl From<String> for Prompt {
    fn from(s: String) -> Self {
        Prompt::text(s)
    }
}

/// Internal: ownership-carrying wrapper for an `FMComposedPrompt`.
///
/// The C side takes ownership when the pointer is passed to a `Respond*` call, so
/// `into_raw()` is mandatory before that handoff and there is no `Drop` impl —
/// otherwise we'd double-free.
pub(crate) struct ComposedPrompt {
    ptr: *const c_void,
}

impl ComposedPrompt {
    pub(crate) fn into_raw(self) -> *const c_void {
        self.ptr
    }
}

fn path_to_cstring(path: &std::path::Path) -> Result<CString> {
    let s = path
        .to_str()
        .ok_or_else(|| Error::Native(format!("non-utf8 path: {}", path.display())))?;
    CString::new(s).map_err(|e| Error::Native(e.to_string()))
}

fn map_image_error(err: sys::FMComposedPromptAddImageError) -> Error {
    match err {
        x if x == sys::FMComposedPromptAddImageError_FMComposedPromptAddImageErrorUnsupported => {
            Error::AttachmentUnsupported
        }
        _ => Error::Native("failed to add image to prompt".into()),
    }
}
