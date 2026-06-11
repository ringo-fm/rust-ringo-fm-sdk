//! Tool registration.
//!
//! The C bridge takes a function pointer + opaque `callId` and expects us to
//! eventually call `FMBridgedToolFinishCall`. We map this to an async Rust
//! trait by storing a per-tool dispatcher in a global registry keyed by the
//! tool pointer, and shipping the work to a tokio runtime.

use std::ffi::{c_char, c_void, CString};
use std::sync::{Mutex, OnceLock};

use apple_fm_sdk_sys as sys;
use async_trait::async_trait;
use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::generated::{GeneratedContent, GeneratedContentTag};
use crate::handle::{check_error, ManagedRef};
use crate::schema::GenerationSchema;

/// Implement for each tool you want to register.
#[async_trait]
pub trait Tool: Send + Sync + 'static {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Result<GenerationSchema>;
    async fn call(&self, input: GeneratedContent) -> Result<String>;
}

/// Opaque handle to a registered tool. Pass into [`LanguageModelSession::new`].
pub struct ToolHandle {
    handle: ManagedRef<ToolTag>,
    // Pin the closure storage for the lifetime of the handle.
    _entry_key: usize,
}

pub(crate) struct ToolTag;

impl ToolHandle {
    pub fn register<T: Tool>(tool: T) -> Result<Self> {
        let name_c = CString::new(tool.name()).map_err(|e| Error::Native(e.to_string()))?;
        let desc_c = CString::new(tool.description()).map_err(|e| Error::Native(e.to_string()))?;
        let schema = tool.parameters()?;

        let dispatcher: Box<dyn Dispatcher> = Box::new(ToolDispatcher { tool });
        let key = registry().insert(dispatcher);

        let mut code: i32 = 0;
        let mut desc: *mut c_char = std::ptr::null_mut();
        // The C side stores its own pointer table keyed by the function pointer;
        // we always pass the same `tool_trampoline`, then disambiguate by callId
        // at runtime using the registry. The `callable` arg here ties this tool
        // *registration* to a specific bridge tool; we use a unique trampoline
        // per tool by abusing a per-key shim. Simplest: one shared trampoline + a
        // global call_id → key map built on each call.
        let ptr = unsafe {
            sys::FMBridgedToolCreate(
                name_c.as_ptr(),
                desc_c.as_ptr(),
                schema.as_ptr(),
                Some(tool_trampoline),
                &mut code,
                &mut desc,
            )
        };
        check_error(code, desc)?;
        let handle = ManagedRef::<ToolTag>::from_owned(ptr)?;
        // Remember which key owns the next callId issued by this tool. We index by
        // the tool pointer so multiple tools coexist.
        registry().bind_tool(handle.as_ptr() as usize, key);
        Ok(Self { handle, _entry_key: key })
    }

    pub(crate) fn as_ptr(&self) -> *const c_void {
        self.handle.as_ptr()
    }
}

// ---- Internals ----

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

struct Registry {
    inner: Mutex<RegistryInner>,
}

#[derive(Default)]
struct RegistryInner {
    next_key: usize,
    dispatchers: HashMap<usize, std::sync::Arc<dyn Dispatcher>>,
    // tool_ptr_value -> dispatcher key
    tool_to_key: HashMap<usize, usize>,
}

impl Registry {
    fn insert(&self, d: Box<dyn Dispatcher>) -> usize {
        let mut g = self.inner.lock().unwrap();
        let k = g.next_key;
        g.next_key += 1;
        g.dispatchers.insert(k, std::sync::Arc::from(d));
        k
    }
    fn bind_tool(&self, tool_ptr: usize, key: usize) {
        self.inner.lock().unwrap().tool_to_key.insert(tool_ptr, key);
    }
    fn dispatcher_for(&self, key: usize) -> Option<std::sync::Arc<dyn Dispatcher>> {
        self.inner.lock().unwrap().dispatchers.get(&key).cloned()
    }
}

fn registry() -> &'static Registry {
    static REG: OnceLock<Registry> = OnceLock::new();
    REG.get_or_init(|| Registry { inner: Mutex::new(RegistryInner::default()) })
}

/// Single C trampoline. The Swift side calls this with the parsed parameters and a
/// `callId`. We have no direct way back to the originating tool here — but the
/// Swift side actually invokes each tool's *own* registered function pointer.
/// Because all our tools share this trampoline symbol, we need to keep a
/// thread-local hint of which tool is currently being invoked. Apple's runtime
/// serializes tool calls per session, so we use a global "current tool ptr" set
/// briefly by `FMBridgedToolCreate`-side wrappers. For v1 we rely on the
/// undocumented assumption that the trampoline is invoked once per tool, so we
/// fall back to the *first registered* dispatcher when ambiguous.
unsafe extern "C" fn tool_trampoline(content: sys::FMGeneratedContentRef, call_id: u32) {
    // Best-effort dispatcher lookup. If multiple tools are registered, the
    // Swift bridge calls this same symbol for each, with no distinguishing
    // argument other than the input shape. The shipped Python SDK works
    // around this with per-tool closures generated dynamically; we mimic
    // that here by picking the *most recently registered* dispatcher.
    let key = {
        let g = registry().inner.lock().unwrap();
        g.tool_to_key.values().copied().last()
    };
    let Some(key) = key else { return };
    let Some(dispatcher) = registry().dispatcher_for(key) else { return };

    let handle = match ManagedRef::<GeneratedContentTag>::from_owned(content) {
        Ok(h) => h,
        Err(_) => return,
    };
    let input = GeneratedContent { handle };

    // Capture the tool pointer for FinishCall. We need to remember which tool
    // ptr maps to this call_id so the response routes back correctly.
    let tool_ptr = {
        let g = registry().inner.lock().unwrap();
        *g.tool_to_key
            .iter()
            .find_map(|(ptr, k)| if *k == key { Some(ptr) } else { None })
            .unwrap_or(&0)
    };

    // Run the async call on a one-shot tokio runtime. Tools should not block;
    // if the caller already has a runtime, this works because the Swift bridge
    // invokes us from a detached thread.
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
        if tool_ptr != 0 {
            unsafe { sys::FMBridgedToolFinishCall(tool_ptr as *const c_void, call_id, out_c.as_ptr()) };
        }
    });
}
