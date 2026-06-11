//! `LanguageModelSession` — async wrapper around `FMLanguageModelSession*`.

use std::ffi::{c_char, c_void, CStr, CString};
use std::sync::Arc;

use ringo_fm_sys as sys;
use tokio::sync::oneshot;

use crate::error::{Error, Result};
use crate::generated::{GeneratedContent, GeneratedContentTag};
use crate::handle::{check_error, FmString, ManagedRef};
use crate::model::SystemLanguageModel;
use crate::options::GenerationOptions;
use crate::prompt::Prompt;
use crate::schema::GenerationSchema;
use crate::stream::ResponseStream;
use crate::tool::ToolHandle;
use crate::transcript::Transcript;

pub(crate) struct SessionTag;

/// Multi-turn session with the language model.
pub struct LanguageModelSession {
    pub(crate) handle: Arc<ManagedRef<SessionTag>>,
    // Keep tool handles alive for as long as the session lives.
    _tools: Vec<ToolHandle>,
}

impl LanguageModelSession {
    /// Default session, equivalent to `LanguageModelSession()` in Python.
    pub fn default() -> Result<Self> {
        let ptr = unsafe { sys::FMLanguageModelSessionCreateDefault() };
        Ok(Self {
            handle: Arc::new(ManagedRef::from_owned(ptr)?),
            _tools: Vec::new(),
        })
    }

    /// Build a session bound to a specific model, optional instructions, and tools.
    pub fn new(
        model: Option<&SystemLanguageModel>,
        instructions: Option<&str>,
        tools: Vec<ToolHandle>,
    ) -> Result<Self> {
        let instr_c = match instructions {
            Some(s) => Some(CString::new(s).map_err(|e| Error::Native(e.to_string()))?),
            None => None,
        };
        let instr_ptr = instr_c.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
        let model_ptr = model.map_or(std::ptr::null(), |m| m.handle.as_ptr());

        let mut tool_ptrs: Vec<*const c_void> = tools.iter().map(|t| t.as_ptr()).collect();
        let (tools_arg, tool_count) = if tool_ptrs.is_empty() {
            (std::ptr::null_mut(), 0)
        } else {
            (tool_ptrs.as_mut_ptr(), tool_ptrs.len() as i32)
        };

        let ptr = unsafe {
            sys::FMLanguageModelSessionCreateFromSystemLanguageModel(
                model_ptr,
                instr_ptr,
                tools_arg,
                tool_count,
            )
        };
        Ok(Self {
            handle: Arc::new(ManagedRef::from_owned(ptr)?),
            _tools: tools,
        })
    }

    /// Whether a respond is currently in flight.
    pub fn is_responding(&self) -> bool {
        unsafe { sys::FMLanguageModelSessionIsResponding(self.handle.as_ptr()) }
    }

    /// Reset the session (clears history).
    pub fn reset(&self) {
        unsafe { sys::FMLanguageModelSessionReset(self.handle.as_ptr()) };
    }

    /// Ask the system to pre-load resources for this session so the first
    /// request has lower latency. `prompt_prefix`, when provided, is the
    /// prefix the next prompt is expected to start with. Prewarm is a
    /// fire-and-forget hint and is safe to call regardless of model
    /// availability.
    pub fn prewarm(&self, prompt_prefix: Option<&str>) -> Result<()> {
        let prefix_c = match prompt_prefix {
            Some(s) => Some(CString::new(s).map_err(|e| Error::Native(e.to_string()))?),
            None => None,
        };
        let prefix_ptr = prefix_c.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());
        unsafe { sys::FMLanguageModelSessionPrewarm(self.handle.as_ptr(), prefix_ptr) };
        Ok(())
    }

    /// One-shot text response.
    pub async fn respond<P: Into<Prompt>>(&self, prompt: P) -> Result<String> {
        self.respond_with(prompt, &GenerationOptions::default()).await
    }

    /// One-shot text response with options.
    pub async fn respond_with<P: Into<Prompt>>(
        &self,
        prompt: P,
        options: &GenerationOptions,
    ) -> Result<String> {
        let composed = prompt.into().into_composed()?;
        let opts_json = options.to_json()?;
        let opts_c = match opts_json {
            Some(s) => Some(CString::new(s).map_err(|e| Error::Native(e.to_string()))?),
            None => None,
        };
        let opts_ptr = opts_c.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());

        let (tx, rx) = oneshot::channel::<Result<String>>();
        let user_info = Box::into_raw(Box::new(tx)) as *mut c_void;

        let task = unsafe {
            sys::FMLanguageModelSessionRespond(
                self.handle.as_ptr(),
                composed.into_raw(),
                opts_ptr,
                user_info,
                Some(text_trampoline),
            )
        };
        let _cancel = CancelOnDrop::new(task);

        match rx.await {
            Ok(r) => r,
            Err(_) => Err(Error::Native("response callback dropped".into())),
        }
    }

    /// Streaming response. Each yielded item is the cumulative snapshot from the model.
    pub fn stream<P: Into<Prompt>>(&self, prompt: P) -> Result<ResponseStream> {
        self.stream_with(prompt, &GenerationOptions::default())
    }

    pub fn stream_with<P: Into<Prompt>>(
        &self,
        prompt: P,
        options: &GenerationOptions,
    ) -> Result<ResponseStream> {
        let composed = prompt.into().into_composed()?;
        let opts_json = options.to_json()?;
        let opts_c = match opts_json {
            Some(s) => Some(CString::new(s).map_err(|e| Error::Native(e.to_string()))?),
            None => None,
        };
        let opts_ptr = opts_c.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());

        let stream_ptr = unsafe {
            sys::FMLanguageModelSessionStreamResponse(
                self.handle.as_ptr(),
                composed.into_raw(),
                opts_ptr,
            )
        };
        ResponseStream::start(stream_ptr)
    }

    /// Structured response. Caller supplies a [`GenerationSchema`] and receives a
    /// [`GeneratedContent`] containing the parsed JSON.
    pub async fn respond_with_schema<P: Into<Prompt>>(
        &self,
        prompt: P,
        schema: &GenerationSchema,
        options: &GenerationOptions,
    ) -> Result<GeneratedContent> {
        let composed = prompt.into().into_composed()?;
        let opts_json = options.to_json()?;
        let opts_c = match opts_json {
            Some(s) => Some(CString::new(s).map_err(|e| Error::Native(e.to_string()))?),
            None => None,
        };
        let opts_ptr = opts_c.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());

        let (tx, rx) = oneshot::channel::<Result<GeneratedContent>>();
        let user_info = Box::into_raw(Box::new(tx)) as *mut c_void;

        let task = unsafe {
            sys::FMLanguageModelSessionRespondWithSchema(
                self.handle.as_ptr(),
                composed.into_raw(),
                schema.as_ptr(),
                opts_ptr,
                user_info,
                Some(structured_trampoline),
            )
        };
        let _cancel = CancelOnDrop::new(task);

        match rx.await {
            Ok(r) => r,
            Err(_) => Err(Error::Native("structured response callback dropped".into())),
        }
    }

    /// Structured response using a raw JSON Schema document.
    pub async fn respond_with_json_schema<P: Into<Prompt>>(
        &self,
        prompt: P,
        schema_json: &str,
        options: &GenerationOptions,
    ) -> Result<GeneratedContent> {
        let composed = prompt.into().into_composed()?;
        let schema_c = CString::new(schema_json).map_err(|e| Error::Native(e.to_string()))?;
        let opts_json = options.to_json()?;
        let opts_c = match opts_json {
            Some(s) => Some(CString::new(s).map_err(|e| Error::Native(e.to_string()))?),
            None => None,
        };
        let opts_ptr = opts_c.as_ref().map_or(std::ptr::null(), |c| c.as_ptr());

        let (tx, rx) = oneshot::channel::<Result<GeneratedContent>>();
        let user_info = Box::into_raw(Box::new(tx)) as *mut c_void;

        let task = unsafe {
            sys::FMLanguageModelSessionRespondWithSchemaFromJSON(
                self.handle.as_ptr(),
                composed.into_raw(),
                schema_c.as_ptr(),
                opts_ptr,
                user_info,
                Some(structured_trampoline),
            )
        };
        let _cancel = CancelOnDrop::new(task);

        match rx.await {
            Ok(r) => r,
            Err(_) => Err(Error::Native("structured response callback dropped".into())),
        }
    }

    /// Full transcript as JSON.
    pub fn transcript(&self) -> Result<Transcript> {
        let mut code: i32 = 0;
        let mut desc: *mut c_char = std::ptr::null_mut();
        let ptr = unsafe {
            sys::FMLanguageModelSessionGetTranscriptJSONString(self.handle.as_ptr(), &mut code, &mut desc)
        };
        check_error(code, desc)?;
        let json = FmString::from_raw(ptr)
            .ok_or_else(|| Error::Native("transcript JSON null".into()))?
            .to_string()?;
        Transcript::from_json(&json)
    }

    /// Build a new session pre-populated with a stored transcript.
    pub fn from_transcript(
        transcript_json: &str,
        model: Option<&SystemLanguageModel>,
        tools: Vec<ToolHandle>,
    ) -> Result<Self> {
        let c = CString::new(transcript_json).map_err(|e| Error::Native(e.to_string()))?;
        let mut code: i32 = 0;
        let mut desc: *mut c_char = std::ptr::null_mut();
        let transcript_session = unsafe {
            sys::FMTranscriptCreateFromJSONString(c.as_ptr(), &mut code, &mut desc)
        };
        check_error(code, desc)?;
        if transcript_session.is_null() {
            return Err(Error::Native("transcript create returned null".into()));
        }
        let _drop_intermediate = ManagedRef::<SessionTag>::from_owned(transcript_session)?;

        let model_ptr = model.map_or(std::ptr::null(), |m| m.handle.as_ptr());
        let mut tool_ptrs: Vec<*const c_void> = tools.iter().map(|t| t.as_ptr()).collect();
        let (tools_arg, tool_count) = if tool_ptrs.is_empty() {
            (std::ptr::null_mut(), 0)
        } else {
            (tool_ptrs.as_mut_ptr(), tool_ptrs.len() as i32)
        };

        let ptr = unsafe {
            sys::FMLanguageModelSessionCreateFromTranscript(
                _drop_intermediate.as_ptr(),
                model_ptr,
                tools_arg,
                tool_count,
            )
        };
        Ok(Self {
            handle: Arc::new(ManagedRef::from_owned(ptr)?),
            _tools: tools,
        })
    }
}

impl std::fmt::Debug for LanguageModelSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LanguageModelSession").finish_non_exhaustive()
    }
}

/// RAII cancel: if dropped while in flight, cancels the underlying FM task.
pub(crate) struct CancelOnDrop {
    task: sys::FMTaskRef,
}

impl CancelOnDrop {
    pub(crate) fn new(task: sys::FMTaskRef) -> Self {
        Self { task }
    }
}

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        if !self.task.is_null() {
            unsafe { sys::FMTaskCancel(self.task) };
        }
    }
}

// === Trampolines ===

pub(crate) unsafe extern "C" fn text_trampoline(
    status: i32,
    content: *const c_char,
    length: usize,
    user_info: *mut c_void,
) {
    let tx = unsafe { Box::from_raw(user_info as *mut oneshot::Sender<Result<String>>) };
    let result = if status == crate::error::status::SUCCESS {
        if content.is_null() {
            Ok(String::new())
        } else {
            let slice = unsafe { std::slice::from_raw_parts(content as *const u8, length) };
            match std::str::from_utf8(slice) {
                Ok(s) => Ok(s.to_owned()),
                Err(e) => Err(Error::Native(format!("non-utf8 model output: {e}"))),
            }
        }
    } else {
        let debug = if content.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(content) }.to_string_lossy().into_owned()
        };
        Err(Error::from_status(status, debug))
    };
    let _ = tx.send(result);
}

unsafe extern "C" fn structured_trampoline(
    status: i32,
    content: sys::FMGeneratedContentRef,
    user_info: *mut c_void,
) {
    let tx = unsafe { Box::from_raw(user_info as *mut oneshot::Sender<Result<GeneratedContent>>) };
    let result = if status == crate::error::status::SUCCESS {
        match ManagedRef::<GeneratedContentTag>::from_owned(content) {
            Ok(handle) => Ok(GeneratedContent { handle }),
            Err(e) => Err(e),
        }
    } else {
        Err(Error::from_status(status, String::new()))
    };
    let _ = tx.send(result);
}
