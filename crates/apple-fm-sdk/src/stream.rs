//! Streaming response — `Stream<Item = Result<String>>`.
//!
//! Each item is the *cumulative* snapshot of the response so far (matching the
//! Swift `streamResponse` semantics). The stream terminates when the Swift side
//! invokes the callback with `content == NULL` and status success.

use std::ffi::{c_char, c_void};
use std::pin::Pin;
use std::task::{Context, Poll};

use apple_fm_sdk_sys as sys;
use futures_core::Stream;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::error::{Error, Result};
use crate::handle::ManagedRef;

pub(crate) struct StreamTag;

/// Async stream of response chunks.
pub struct ResponseStream {
    _handle: ManagedRef<StreamTag>,
    rx: UnboundedReceiver<Result<String>>,
    // Boxed sender; the trampoline owns the only raw alias.
    _sender_guard: SenderGuard,
}

struct SenderGuard {
    ptr: *mut UnboundedSender<Result<String>>,
    consumed: bool,
}

impl SenderGuard {
    fn new(sender: UnboundedSender<Result<String>>) -> Self {
        let ptr = Box::into_raw(Box::new(sender));
        Self { ptr, consumed: false }
    }
    fn as_raw(&self) -> *mut c_void {
        self.ptr as *mut c_void
    }
}

impl Drop for SenderGuard {
    fn drop(&mut self) {
        if !self.consumed {
            // We never received an EOF callback; reclaim the box to avoid leaking.
            // Worst case: pending callback fires after free — but the stream handle
            // is also released here, so the Swift Task is cancelled before drop completes.
            unsafe { drop(Box::from_raw(self.ptr)) };
        }
    }
}

impl ResponseStream {
    pub(crate) fn start(stream_ptr: *const c_void) -> Result<Self> {
        let handle = ManagedRef::<StreamTag>::from_owned(stream_ptr)?;
        let (tx, rx) = unbounded_channel::<Result<String>>();
        let guard = SenderGuard::new(tx);
        unsafe {
            sys::FMLanguageModelSessionResponseStreamIterate(
                handle.as_ptr(),
                guard.as_raw(),
                Some(stream_trampoline),
            );
        }
        Ok(Self {
            _handle: handle,
            rx,
            _sender_guard: guard,
        })
    }
}

impl Stream for ResponseStream {
    type Item = Result<String>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

unsafe extern "C" fn stream_trampoline(
    status: i32,
    content: *const c_char,
    length: usize,
    user_info: *mut c_void,
) {
    // The sender is owned by the SenderGuard; we re-borrow via raw ptr here.
    if user_info.is_null() {
        return;
    }
    let tx_ref = unsafe { &*(user_info as *const UnboundedSender<Result<String>>) };

    if status == crate::error::status::SUCCESS {
        if content.is_null() {
            // EOF — just close the channel by not sending; the sender is freed when
            // the stream is dropped.
            return;
        }
        let slice = unsafe { std::slice::from_raw_parts(content as *const u8, length) };
        match std::str::from_utf8(slice) {
            Ok(s) => {
                let _ = tx_ref.send(Ok(s.to_owned()));
            }
            Err(e) => {
                let _ = tx_ref.send(Err(Error::Native(format!("non-utf8 chunk: {e}"))));
            }
        }
    } else {
        let debug = if content.is_null() {
            String::new()
        } else {
            let slice = unsafe { std::slice::from_raw_parts(content as *const u8, length) };
            String::from_utf8_lossy(slice).into_owned()
        };
        let _ = tx_ref.send(Err(Error::from_status(status, debug)));
    }
}
