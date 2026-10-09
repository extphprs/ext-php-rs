//! Output handlers that filter what PHP writes, the way `ob_start()` does.

use std::{
    borrow::Cow,
    ffi::{c_int, c_void},
    mem,
    panic::AssertUnwindSafe,
    ptr, slice,
};

use bitflags::bitflags;

use crate::{
    alloc::efree,
    error::{Error, Result},
    ffi::{
        PHP_OUTPUT_HANDLER_CLEAN, PHP_OUTPUT_HANDLER_CLEANABLE, PHP_OUTPUT_HANDLER_FINAL,
        PHP_OUTPUT_HANDLER_FLUSH, PHP_OUTPUT_HANDLER_FLUSHABLE, PHP_OUTPUT_HANDLER_REMOVABLE,
        PHP_OUTPUT_HANDLER_START, ZEND_RESULT_CODE_FAILURE, ZEND_RESULT_CODE_SUCCESS,
        ext_php_rs_output_activated, php_output_context, php_output_handler,
        php_output_handler_create_internal, php_output_handler_free,
        php_output_handler_set_context, php_output_handler_start,
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

struct HandlerState<F> {
    func: F,
    output: Vec<u8>,
}

enum Slot<F> {
    Idle(HandlerState<F>),
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

    let state = Box::into_raw(Box::new(Slot::Idle(HandlerState {
        func: handler,
        output: Vec::new(),
    })));

    // SAFETY: output is active, so the handler goes on the request heap and on
    // the live handler stack. The engine copies `name`. From `set_context`
    // on, the handler owns `state` and frees it through `drop_state::<F>`.
    unsafe {
        let mut php_handler = php_output_handler_create_internal(
            name.as_ptr().cast(),
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
    let mut state = match unsafe { mem::replace(&mut *slot, Slot::Running) } {
        Slot::Idle(state) => state,
        other => {
            unsafe { *slot = other };
            return ZEND_RESULT_CODE_FAILURE;
        }
    };

    // SAFETY: `handler_context` points at the `opaq` field of a live handler,
    // and the engine fed `in_` with that handler's buffer. Detaching the
    // buffer sends output written by `func` to a fresh one, so `input`
    // stays valid.
    let (handler, buffer, input, op) = unsafe {
        let handler = handler_context
            .byte_sub(mem::offset_of!(php_output_handler, opaq))
            .cast::<php_output_handler>();
        let buffer = ptr::replace(&raw mut (*handler).buffer, mem::zeroed());
        let context = &*output_context;
        let input = if context.in_.used == 0 {
            &[][..]
        } else {
            slice::from_raw_parts(context.in_.data.cast::<u8>(), context.in_.used)
        };
        (
            handler,
            buffer,
            input,
            OutputOp::from_bits_truncate(context.op.cast_unsigned()),
        )
    };

    let HandlerState { func, output } = &mut state;
    let result = try_catch(AssertUnwindSafe(|| match func(input, op) {
        Cow::Borrowed(bytes) => (bytes.as_ptr(), bytes.len()),
        Cow::Owned(bytes) => {
            *output = bytes;
            (output.as_ptr(), output.len())
        }
    }));

    // SAFETY: the slot is live, see above.
    if matches!(unsafe { &*slot }, Slot::Freed) {
        // SAFETY: the engine freed the handler on a fatal error that `func`
        // swallowed with `try_catch`, so the slot and the detached buffer are
        // only ours. The fatal error is raised again, as the engine
        // expects.
        unsafe {
            drop(Box::from_raw(slot));
            if !buffer.data.is_null() {
                efree(buffer.data.cast());
            }
        }
        if matches!(result, Err(CatchError::Bailout)) {
            mem::forget(state);
        } else {
            drop(state);
        }
        // SAFETY: re-raises the fatal error to the enclosing `zend_try`.
        unsafe { bailout() }
    }

    // SAFETY: the handler is live as the slot is not `Freed`. The engine drops
    // output written by a running handler, so the fresh buffer is freed.
    unsafe {
        let written = ptr::replace(&raw mut (*handler).buffer, buffer);
        if !written.data.is_null() {
            efree(written.data.cast());
        }
    }

    if let Err(CatchError::Bailout) = result {
        mem::forget(state);
        // SAFETY: the slot is live, see above. Re-raises the bailout to the
        // enclosing `zend_try`.
        unsafe {
            *slot = Slot::Poisoned;
            bailout()
        }
    }

    // SAFETY: the slot is live, see above. Moving `state` keeps `state.output`
    // where it is.
    unsafe { *slot = Slot::Idle(state) };
    let Ok((data, len)) = result else {
        return ZEND_RESULT_CODE_FAILURE;
    };
    if len > 0 {
        // SAFETY: `data` points into the handler buffer, `'static` memory or
        // `state.output`. The engine is done with `out` before it runs or frees
        // the handler again, and none of them change before that. `free` is
        // cleared so the engine does not free it.
        unsafe {
            let out = &mut (*output_context).out;
            out.data = data.cast_mut().cast();
            out.used = len;
            out.size = len;
            out.set_free(0);
        }
    }
    ZEND_RESULT_CODE_SUCCESS
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
    use crate::{embed::Embed, zend::output_write};

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
