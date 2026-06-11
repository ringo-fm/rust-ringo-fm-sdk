//! Tool registration — slot-based dispatch mirroring go-ringo-fm-sdk/bridge.c.
//!
//! Each registered tool occupies a numbered slot (0–31). `bridge_rust.c`
//! generates 32 distinct C function pointers; each calls
//! `rust_fm_tool_callback_slot(slot, …)` so the Swift runtime routes calls to
//! the correct Rust handler even when multiple tools share the same session.

use std::ffi::{c_char, c_void, CString};
use std::sync::{Arc, Mutex};

use ringo_fm_sys as sys;
use async_trait::async_trait;

use crate::error::{Error, Result};
use crate::generated::{GeneratedContent, GeneratedContentTag};
use crate::handle::{check_error, ManagedRef};
use crate::schema::GenerationSchema;

// Maximum number of concurrently registered tools. Must match FM_TOOL_SLOTS in bridge_rust.c.
const FM_TOOL_SLOTS: usize = 32;

/// Implement for each tool you want to register.
#[async_trait]
pub trait Tool: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Result<GenerationSchema>;
    async fn call(&self, input: GeneratedContent) -> Result<String>;
}

/// Opaque handle to a registered tool. Pass into [`crate::LanguageModelSession::new`].
pub struct ToolHandle {
    pub(crate) handle: ManagedRef<ToolTag>,
    slot: usize,
}

pub(crate) struct ToolTag;

// ---- Slot registry -------------------------------------------------------

#[async_trait]
trait Dispatcher: Send + Sync {
    async fn dispatch(&self, input: GeneratedContent) -> Result<String>;
}

struct ToolDispatcher<T: Tool> {
    tool: T,
}

#[async_trait]
impl<T: Tool> Dispatcher for ToolDispatcher<T> {
    async fn dispatch(&self, input: GeneratedContent) -> Result<String> {
        self.tool.call(input).await
    }
}

struct SlotEntry {
    dispatcher: Arc<dyn Dispatcher>,
    // Stored as usize to satisfy Send/Sync; cast back to *const c_void when needed.
    tool_ptr: usize,
}

unsafe impl Send for SlotEntry {}
unsafe impl Sync for SlotEntry {}

struct SlotRegistry {
    slots: Mutex<[Option<SlotEntry>; FM_TOOL_SLOTS]>,
}

impl SlotRegistry {
    const fn new() -> Self {
        // Option<SlotEntry> is not Copy so we can't use array repeat syntax;
        // build via an unsafe transmute of zeroed memory instead.
        Self { slots: Mutex::new(unsafe { std::mem::zeroed() }) }
    }

    fn acquire(&self) -> Option<usize> {
        let mut g = self.slots.lock().unwrap();
        for (i, slot) in g.iter_mut().enumerate() {
            if slot.is_none() {
                // Mark occupied with a placeholder so a concurrent acquire skips it.
                *slot = Some(SlotEntry {
                    dispatcher: Arc::new(NoopDispatcher),
                    tool_ptr: 0,
                });
                return Some(i);
            }
        }
        None
    }

    fn commit(&self, slot: usize, entry: SlotEntry) {
        let mut g = self.slots.lock().unwrap();
        g[slot] = Some(entry);
    }

    fn release(&self, slot: usize) {
        let mut g = self.slots.lock().unwrap();
        g[slot] = None;
    }

    fn get(&self, slot: usize) -> Option<(Arc<dyn Dispatcher>, usize)> {
        let g = self.slots.lock().unwrap();
        g[slot].as_ref().map(|e| (e.dispatcher.clone(), e.tool_ptr))
    }
}

struct NoopDispatcher;

#[async_trait]
impl Dispatcher for NoopDispatcher {
    async fn dispatch(&self, _input: GeneratedContent) -> Result<String> {
        Err(Error::Native("tool slot not yet committed".into()))
    }
}

static REGISTRY: SlotRegistry = SlotRegistry::new();

// ---- C-callable entry point ----------------------------------------------

/// Called by each slot trampoline in bridge_rust.c.
#[no_mangle]
pub extern "C" fn rust_fm_tool_callback_slot(
    slot: i32,
    content: sys::FMGeneratedContentRef,
    call_id: u32,
) {
    let slot = slot as usize;
    let Some((dispatcher, tool_ptr_usize)) = REGISTRY.get(slot) else { return };

    let handle = match ManagedRef::<GeneratedContentTag>::from_owned(content) {
        Ok(h) => h,
        Err(_) => return,
    };
    let input = GeneratedContent { handle };

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(r) => r,
            Err(_) => return,
        };
        let result = rt.block_on(dispatcher.dispatch(input));
        let output = match result {
            Ok(s) => s,
            Err(e) => format!("{{\"error\": \"{}\"}}", e.to_string().replace('"', "\\\"")),
        };
        let Ok(out_c) = CString::new(output) else { return };
        if tool_ptr_usize != 0 {
            let tool_ptr = tool_ptr_usize as *const c_void;
            unsafe { sys::FMBridgedToolFinishCall(tool_ptr, call_id, out_c.as_ptr()) };
        }
    });
}

// ---- Public API ----------------------------------------------------------

// Declared in bridge_rust.c; routes through the per-slot trampoline.
extern "C" {
    fn fm_rust_tool_create_at_slot(
        slot: i32,
        name: *const c_char,
        description: *const c_char,
        parameters: *const c_void,
        out_error_code: *mut i32,
        out_error_description: *mut *mut c_char,
    ) -> *const c_void;
}

impl ToolHandle {
    /// Register a tool and obtain a handle to pass to [`crate::LanguageModelSession::new`].
    pub fn register<T: Tool>(tool: T) -> Result<Self> {
        let slot = REGISTRY.acquire().ok_or_else(|| {
            Error::Native(format!("tool: no free slot (limit {FM_TOOL_SLOTS} concurrent tools)"))
        })?;

        let name_c = CString::new(tool.name()).map_err(|e| Error::Native(e.to_string()))?;
        let desc_c = CString::new(tool.description()).map_err(|e| Error::Native(e.to_string()))?;
        let schema = tool.parameters()?;

        let mut code: i32 = 0;
        let mut desc: *mut c_char = std::ptr::null_mut();
        let ptr = unsafe {
            fm_rust_tool_create_at_slot(
                slot as i32,
                name_c.as_ptr(),
                desc_c.as_ptr(),
                schema.as_ptr(),
                &mut code,
                &mut desc,
            )
        };
        if let Err(e) = check_error(code, desc) {
            REGISTRY.release(slot);
            return Err(e);
        }
        if ptr.is_null() {
            REGISTRY.release(slot);
            return Err(Error::Native(format!("tool: failed to create {:?}", tool.name())));
        }

        let handle = match ManagedRef::<ToolTag>::from_owned(ptr) {
            Ok(h) => h,
            Err(e) => { REGISTRY.release(slot); return Err(e); }
        };

        REGISTRY.commit(slot, SlotEntry {
            dispatcher: Arc::new(ToolDispatcher { tool }),
            tool_ptr: handle.as_ptr() as usize,
        });

        Ok(Self { handle, slot })
    }

    pub(crate) fn as_ptr(&self) -> *const c_void {
        self.handle.as_ptr()
    }
}

impl Drop for ToolHandle {
    fn drop(&mut self) {
        REGISTRY.release(self.slot);
    }
}
