//! Output handlers that filter what PHP writes, the way `ob_start()` does.

use std::{
    alloc::Layout,
    borrow::Cow,
    ffi::{c_int, c_void},
    mem,
    panic::AssertUnwindSafe,
    ptr, slice,
};

use bitflags::bitflags;

use crate::{
    alloc::{efree, emalloc},
    error::{Error, Result},
    ffi::{
        PHP_OUTPUT_HANDLER_CLEAN, PHP_OUTPUT_HANDLER_CLEANABLE, PHP_OUTPUT_HANDLER_DEFAULT_SIZE,
        PHP_OUTPUT_HANDLER_FINAL, PHP_OUTPUT_HANDLER_FLUSH, PHP_OUTPUT_HANDLER_FLUSHABLE,
        PHP_OUTPUT_HANDLER_REMOVABLE, PHP_OUTPUT_HANDLER_START, ZEND_RESULT_CODE_FAILURE,
        ZEND_RESULT_CODE_SUCCESS, ext_php_rs_output_activated, php_output_buffer,
        php_output_context, php_output_handler, php_output_handler_create_internal,
        php_output_handler_free, php_output_handler_set_context, php_output_handler_start,
    },
    zend::{CatchError, bailout, try_catch},
};

bitflags! {
    /// What userland may do with a handler started by [`start_output_handler`].
    #[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
    pub struct OutputHandlerFlags: u32 {
        /// `ob_clean()` may discard the buffered output.
        const Cleanable = PHP_OUTPUT_HANDLER_CLEANABLE;
        /// `ob_flush()` may send the buffered output.
        const Flushable = PHP_OUTPUT_HANDLER_FLUSHABLE;
        /// `ob_end_flush()`, `ob_end_clean()` and `ob_get_clean()` may remove
        /// the handler.
        const Removable = PHP_OUTPUT_HANDLER_REMOVABLE;
        /// The flags `ob_start()` uses by default.
        const Std = Self::Cleanable.bits() | Self::Flushable.bits() | Self::Removable.bits();
    }
}

bitflags! {
    /// Why PHP runs the output handler. An empty set is a plain write.
    #[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
    pub struct OutputOp: u32 {
        /// First run of the handler.
        const Start = PHP_OUTPUT_HANDLER_START;
        /// The buffer is cleaned. PHP discards what the handler returns.
        const Clean = PHP_OUTPUT_HANDLER_CLEAN;
        /// The buffer is flushed.
        const Flush = PHP_OUTPUT_HANDLER_FLUSH;
        /// Last run of the handler. PHP removes it after this run.
        const Final = PHP_OUTPUT_HANDLER_FINAL;
    }
}

enum Slot<F> {
    Idle(F),
    Running,
    Freed,
    Poisoned,
}

/// Pushes `handler` on top of PHP's output buffer stack, like `ob_start()`
/// with a callback.
///
/// PHP runs `handler` with the buffered bytes and an [`OutputOp`] when the
/// buffer reaches `chunk_size` bytes (`0` means no limit), and when it is
/// flushed, cleaned or removed. PHP sends on the bytes that `handler` returns.
/// Return `Cow::Borrowed(input)` to pass the input through unchanged, or an
/// empty slice to drop it. At the end of the request, PHP runs `handler` one
/// last time with [`OutputOp::Final`] and then drops it.
///
/// The input is not copied. A borrowed start of the input (`input` or
/// `&input[..n]`) is sent without a copy. Any other output is copied once to
/// the request heap.
///
/// PHP discards the output that `handler` writes while it runs. If `handler`
/// panics, PHP sends the buffered bytes unchanged and disables the handler. If
/// `handler` causes a fatal error, the closure leaks.
///
/// # Errors
///
/// [`Error::OutputHandlerStartFailed`] if no request is active (for example
/// during `MINIT`), or if a conflict check registered for `name` rejects the
/// handler.
///
/// # Example
///
/// ```ignore
/// use std::borrow::Cow;
/// use ext_php_rs::zend::{OutputHandlerFlags, start_output_handler};
///
/// start_output_handler("upper", 0, OutputHandlerFlags::Std, |input, _| {
///     Cow::Owned(input.to_ascii_uppercase())
/// })?;
/// ```
pub fn start_output_handler<F>(
    name: &str,
    chunk_size: usize,
    flags: OutputHandlerFlags,
    handler: F,
) -> Result<()>
where
    F: for<'a> FnMut(&'a [u8], OutputOp) -> Cow<'a, [u8]> + 'static,
{
    // SAFETY: the output globals exist from module startup until the process
    // ends.
    if !unsafe { ext_php_rs_output_activated() } {
        return Err(Error::OutputHandlerStartFailed);
    }

    let state = Box::into_raw(Box::new(Slot::Idle(handler)));
    let name_ptr = if name.is_empty() {
        c"".as_ptr()
    } else {
        name.as_ptr().cast()
    };

    // SAFETY: output is active, so the handler goes on the request heap and on
    // the live handler stack. The engine copies `name`. From `set_context`
    // on, the handler owns `state` and frees it through `drop_state::<F>`.
    unsafe {
        let mut php_handler = php_output_handler_create_internal(
            name_ptr,
            name.len(),
            Some(call_handler::<F>),
            chunk_size,
            flags.bits().cast_signed(),
        );
        php_output_handler_set_context(php_handler, state.cast(), Some(drop_state::<F>));
        if php_output_handler_start(php_handler) == ZEND_RESULT_CODE_SUCCESS {
            return Ok(());
        }
        php_output_handler_free(&raw mut php_handler);
    }
    Err(Error::OutputHandlerStartFailed)
}

enum Output {
    Empty,
    Input(usize),
    Copied(*mut u8, usize),
}

unsafe extern "C" fn call_handler<F>(
    handler_context: *mut *mut c_void,
    output_context: *mut php_output_context,
) -> c_int
where
    F: for<'a> FnMut(&'a [u8], OutputOp) -> Cow<'a, [u8]> + 'static,
{
    // SAFETY: the engine passes `&handler->opaq`, which `start_output_handler`
    // set to a live `Slot<F>`.
    let slot = unsafe { *handler_context }.cast::<Slot<F>>();
    // SAFETY: while the slot is `Running`, `drop_state` marks it `Freed`
    // instead of freeing it, so it outlives this call.
    let mut func = match unsafe { mem::replace(&mut *slot, Slot::Running) } {
        Slot::Idle(func) => func,
        other => {
            // SAFETY: see above.
            unsafe { *slot = other };
            return ZEND_RESULT_CODE_FAILURE;
        }
    };

    // SAFETY: `handler_context` points at the `opaq` field of a live handler,
    // and the engine fed `in_` with that handler's buffer. Detaching the
    // buffer sends output written by `func` to a fresh one, so `input` stays
    // valid and unchanged.
    let (handler, buffer, op) = unsafe {
        let handler = handler_context
            .byte_sub(mem::offset_of!(php_output_handler, opaq))
            .cast::<php_output_handler>();
        let buffer = ptr::replace(&raw mut (*handler).buffer, mem::zeroed());
        let op = OutputOp::from_bits_truncate((*output_context).op.cast_unsigned());
        (handler, buffer, op)
    };
    let input = if buffer.used == 0 {
        &[][..]
    } else {
        // SAFETY: the detached buffer holds `used` bytes and only this call
        // can reach it.
        unsafe { slice::from_raw_parts(buffer.data.cast::<u8>(), buffer.used) }
    };

    let result = try_catch(AssertUnwindSafe(|| {
        let output = func(input, op);
        if output.is_empty() {
            Output::Empty
        } else if matches!(output, Cow::Borrowed(bytes) if bytes.as_ptr() == input.as_ptr()) {
            Output::Input(output.len())
        } else {
            Output::Copied(emalloc_copy(&output), output.len())
        }
    }));

    // SAFETY: the slot is live, see above.
    if matches!(unsafe { &*slot }, Slot::Freed) {
        // SAFETY: the engine freed the handler on a fatal error that `func`
        // swallowed with `try_catch`, so the slot and the buffers are only
        // ours. The fatal error is raised again, as the engine expects.
        unsafe {
            drop(Box::from_raw(slot));
            free_buffer(buffer);
            if let Ok(Output::Copied(data, _)) = result {
                efree(data);
            }
        }
        if matches!(result, Err(CatchError::Bailout)) {
            mem::forget(func);
        } else {
            drop(func);
        }
        // SAFETY: re-raises the fatal error to the enclosing `zend_try`.
        unsafe { bailout() }
    }

    // SAFETY: the handler is live as the slot is not `Freed`. It gets its
    // buffer back, and the engine drops what `func` wrote meanwhile.
    let written = unsafe { ptr::replace(&raw mut (*handler).buffer, buffer) };
    let output = match result {
        Ok(output) => output,
        Err(error) => {
            // SAFETY: `written` came from the engine and nothing else points
            // at it.
            unsafe { free_buffer(written) };
            if let CatchError::Bailout = error {
                mem::forget(func);
                // SAFETY: the slot is live. Re-raises the bailout to the
                // enclosing `zend_try`.
                unsafe {
                    *slot = Slot::Poisoned;
                    bailout()
                }
            }
            // SAFETY: the slot is live.
            unsafe { *slot = Slot::Idle(func) };
            return ZEND_RESULT_CODE_FAILURE;
        }
    };
    // SAFETY: the slot is live.
    unsafe { *slot = Slot::Idle(func) };

    // SAFETY: the engine owns `out` with `free` set, so no later write to the
    // handler, even from a header callback that runs the handler again, can
    // change or free what it points at.
    unsafe {
        let context = &mut *output_context;
        match output {
            Output::Empty => free_buffer(written),
            Output::Copied(data, len) => {
                free_buffer(written);
                give_to_engine(&mut context.out, data, len, len);
            }
            Output::Input(len) => {
                (*handler).buffer = if written.data.is_null() {
                    new_buffer()
                } else {
                    php_output_buffer { used: 0, ..written }
                };
                context.in_ = mem::zeroed();
                give_to_engine(&mut context.out, buffer.data.cast(), len, buffer.size);
            }
        }
    }
    ZEND_RESULT_CODE_SUCCESS
}

fn emalloc_copy(bytes: &[u8]) -> *mut u8 {
    let data = emalloc(Layout::for_value(bytes));
    // SAFETY: `data` is a fresh request allocation of `bytes.len()` bytes.
    unsafe { ptr::copy_nonoverlapping(bytes.as_ptr(), data, bytes.len()) };
    data
}

fn new_buffer() -> php_output_buffer {
    let layout = Layout::new::<[u8; PHP_OUTPUT_HANDLER_DEFAULT_SIZE as usize]>();
    // SAFETY: an all-zero `php_output_buffer` is an empty buffer.
    let mut buffer: php_output_buffer = unsafe { mem::zeroed() };
    buffer.data = emalloc(layout).cast();
    buffer.size = layout.size();
    buffer
}

/// # Safety
///
/// `buffer.data` must be null or a request allocation that nothing else uses.
unsafe fn free_buffer(buffer: php_output_buffer) {
    if !buffer.data.is_null() {
        // SAFETY: see the function contract.
        unsafe { efree(buffer.data.cast()) };
    }
}

/// # Safety
///
/// `out` must be the empty output buffer of a live context, and `data` a
/// request allocation of `size` bytes that nothing else uses.
unsafe fn give_to_engine(out: &mut php_output_buffer, data: *mut u8, used: usize, size: usize) {
    out.data = data.cast();
    out.used = used;
    out.size = size;
    out.set_free(1);
}

unsafe extern "C" fn drop_state<F>(state: *mut c_void) {
    let slot = state.cast::<Slot<F>>();
    // SAFETY: `state` is the box leaked by `start_output_handler`. The engine
    // calls this once, when it frees the handler. A running `call_handler`
    // owns the box until it returns.
    unsafe {
        if matches!(*slot, Slot::Running) {
            *slot = Slot::Freed;
        } else {
            drop(Box::from_raw(slot));
        }
    }
}

#[cfg(all(test, feature = "embed"))]
mod tests {
    use std::{cell::RefCell, panic::AssertUnwindSafe, rc::Rc};

    use super::*;
    use crate::{
        embed::Embed,
        zend::{SapiGlobals, output_write},
    };

    fn filter<F>(input: &'static [u8], handler: F) -> String
    where
        F: for<'a> FnMut(&'a [u8], OutputOp) -> Cow<'a, [u8]> + 'static,
    {
        Embed::run(AssertUnwindSafe(move || {
            Embed::eval("ob_start();").expect("ob_start failed");
            start_output_handler("test", 0, OutputHandlerFlags::Std, handler)
                .expect("start_output_handler failed");
            let _ = output_write(input);
            Embed::eval("ob_end_flush();").expect("ob_end_flush failed");
            Embed::eval("ob_get_clean();")
                .ok()
                .and_then(|output| output.string())
                .unwrap_or_default()
        }))
    }

    #[test]
    fn borrowed_input_passes_through() {
        assert_eq!(filter(b"hello", |input, _| Cow::Borrowed(input)), "hello");
    }

    #[test]
    fn borrowed_prefix_is_sent() {
        assert_eq!(
            filter(b"hello", |input, _| Cow::Borrowed(&input[..2])),
            "he"
        );
    }

    #[test]
    fn borrowed_middle_is_sent() {
        assert_eq!(
            filter(b"hello", |input, _| Cow::Borrowed(&input[1..4])),
            "ell"
        );
    }

    #[test]
    fn owned_output_replaces_input() {
        let output = filter(b"hello", |input, _| Cow::Owned(input.to_ascii_uppercase()));

        assert_eq!(output, "HELLO");
    }

    #[test]
    fn static_output_replaces_input() {
        assert_eq!(filter(b"hello", |_, _| Cow::Borrowed(b"bye")), "bye");
    }

    #[test]
    fn empty_output_drops_input() {
        assert_eq!(filter(b"hello", |_, _| Cow::Borrowed(&[])), "");
    }

    #[test]
    fn output_written_by_the_handler_leaves_input_intact() {
        let output = filter(b"hello", |input, _| {
            for _ in 0..64 {
                let _ = output_write(&[b'x'; 1024]);
            }
            Cow::Borrowed(input)
        });

        assert_eq!(output, "hello");
    }

    #[test]
    fn panicking_handler_passes_input_through() {
        assert_eq!(filter(b"hello", |_, _| panic!("handler panic")), "hello");
    }

    #[test]
    fn handler_sees_each_op() {
        let ops = Embed::run(|| {
            let ops = Rc::new(RefCell::new(Vec::new()));
            let seen = Rc::clone(&ops);
            Embed::eval("ob_start();").expect("ob_start failed");
            start_output_handler("test", 0, OutputHandlerFlags::Std, move |input, op| {
                seen.borrow_mut().push(op);
                Cow::Borrowed(input)
            })
            .expect("start_output_handler failed");
            let _ = output_write(b"a");
            Embed::eval("ob_flush();").expect("ob_flush failed");
            let _ = output_write(b"b");
            Embed::eval("ob_end_flush();").expect("ob_end_flush failed");
            Embed::eval("ob_end_clean();").expect("ob_end_clean failed");
            ops.take()
        });

        assert_eq!(ops, [OutputOp::Start | OutputOp::Flush, OutputOp::Final]);
    }

    fn send_headers_through<F>(handler: F) -> usize
    where
        F: for<'a> FnMut(&'a [u8], OutputOp) -> Cow<'a, [u8]> + 'static,
    {
        Embed::run(AssertUnwindSafe(move || {
            {
                let mut sapi = SapiGlobals::get_mut();
                sapi.headers_sent = 0;
                sapi.request_info.no_headers = false;
            }
            Embed::eval("header_register_callback(fn () => print(str_repeat('b', 65536)));")
                .expect("header_register_callback failed");
            let calls = Rc::new(RefCell::new(0));
            let seen = Rc::clone(&calls);
            let mut handler = handler;
            start_output_handler("test", 1, OutputHandlerFlags::Std, move |input, op| {
                *seen.borrow_mut() += 1;
                handler(input, op)
            })
            .expect("start_output_handler failed");
            let _ = output_write(&[b'a'; 4096]);
            calls.take()
        }))
    }

    #[test]
    fn header_callback_output_leaves_owned_output_intact() {
        let calls = send_headers_through(|input, _| Cow::Owned(input.to_ascii_uppercase()));

        assert!(calls > 1);
    }

    #[test]
    fn header_callback_output_leaves_borrowed_input_intact() {
        assert!(send_headers_through(|input, _| Cow::Borrowed(input)) > 1);
    }

    #[test]
    fn fatal_error_swallowed_by_the_handler_is_raised_again() {
        let bailed_out = Embed::run(|| {
            start_output_handler("test", 0, OutputHandlerFlags::Std, |input, _| {
                let _ = Embed::eval("ob_end_clean();");
                Cow::Borrowed(input)
            })
            .expect("start_output_handler failed");
            let _ = output_write(b"hello");
            Embed::eval("ob_end_flush();").is_err_and(|err| err.is_bailout())
        });

        assert!(bailed_out);
    }
}
