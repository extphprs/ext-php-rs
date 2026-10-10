//! Response headers and status code, the way `header()`, `header_remove()` and
//! `http_response_code()` set them.

use std::{ffi::c_void, mem, ptr};

use crate::{
    error::{Error, Result},
    ffi::{
        ZEND_RESULT_CODE_SUCCESS, ext_php_rs_output_activated, sapi_header_line, sapi_header_op,
        sapi_header_op_enum, sapi_header_op_enum_SAPI_HEADER_ADD,
        sapi_header_op_enum_SAPI_HEADER_DELETE, sapi_header_op_enum_SAPI_HEADER_DELETE_ALL,
        sapi_header_op_enum_SAPI_HEADER_REPLACE, sapi_header_op_enum_SAPI_HEADER_SET_STATUS,
    },
    zend::globals::lock,
};

/// Sets a response header and removes the headers with the same name, like
/// `header($header)`.
///
/// `header` is a full line, for example `"Content-Type: text/plain"`. PHP
/// applies its usual rules: a `Location` header sets the status to `302` (or
/// `303` for a non `GET`/`HEAD` request) unless it is already `201` or `3xx`,
/// and an `HTTP/` line sets the status line. Call [`set_response_code`] after
/// a `Location` header to send another redirect code.
///
/// # Errors
///
/// [`Error::ResponseHeaderFailed`] if no request is active (for example during
/// `MINIT`), if a [`SapiGlobals`](crate::zend::SapiGlobals) guard is alive, if
/// `header` is empty, or if PHP refuses the header. PHP emits a warning when
/// the headers are already sent or when `header` contains a new line or a NUL
/// byte.
pub fn set_header(header: &str) -> Result<()> {
    line_op(sapi_header_op_enum_SAPI_HEADER_REPLACE, header)
}

/// Adds a response header and keeps the headers with the same name, like
/// `header($header, false)`.
///
/// # Errors
///
/// See [`set_header`].
pub fn add_header(header: &str) -> Result<()> {
    line_op(sapi_header_op_enum_SAPI_HEADER_ADD, header)
}

/// Removes the response headers called `name`, like `header_remove($name)`.
///
/// # Errors
///
/// See [`set_header`]. PHP emits a warning when `name` contains a colon.
pub fn remove_header(name: &str) -> Result<()> {
    line_op(sapi_header_op_enum_SAPI_HEADER_DELETE, name)
}

/// Removes all the response headers, like `header_remove()`.
///
/// # Errors
///
/// See [`set_header`].
pub fn remove_all_headers() -> Result<()> {
    header_op(sapi_header_op_enum_SAPI_HEADER_DELETE_ALL, ptr::null_mut())
}

/// Sets the response status code, like `http_response_code($code)`.
///
/// # Errors
///
/// See [`set_header`].
pub fn set_response_code(code: u16) -> Result<()> {
    header_op(
        sapi_header_op_enum_SAPI_HEADER_SET_STATUS,
        ptr::without_provenance_mut(usize::from(code)),
    )
}

fn line_op(op: sapi_header_op_enum, line: &str) -> Result<()> {
    // SAFETY: SAPI.h requires a zeroed `sapi_header_line`. All its fields are
    // integers or raw pointers, for which zero is valid.
    let mut header_line: sapi_header_line = unsafe { mem::zeroed() };
    header_line.line = line.as_ptr().cast();
    header_line.line_len = line.len();
    header_op(op, (&raw mut header_line).cast())
}

fn header_op(op: sapi_header_op_enum, arg: *mut c_void) -> Result<()> {
    // SAFETY: the output globals exist from module startup until the process
    // ends.
    if !unsafe { ext_php_rs_output_activated() } || sapi_globals_borrowed() {
        return Err(Error::ResponseHeaderFailed);
    }
    // SAFETY: output is active only between `sapi_activate` and
    // `sapi_deactivate_module`, so the header list is live. No guard is alive,
    // so no Rust reference sees the engine change the SAPI globals. `arg` is
    // what `op` reads: a `sapi_header_line` that the engine copies, a status
    // code or nothing. The engine frees its copy before it emits a warning, and
    // this frame owns nothing to drop, so a bailout from an error handler
    // leaks nothing.
    if unsafe { sapi_header_op(op, arg) } == ZEND_RESULT_CODE_SUCCESS {
        Ok(())
    } else {
        Err(Error::ResponseHeaderFailed)
    }
}

fn sapi_globals_borrowed() -> bool {
    cfg_if::cfg_if! {
        if #[cfg(php_zts)] {
            lock::SAPI_GLOBALS_LOCK.with(|lock| lock.is_locked())
        } else {
            lock::SAPI_GLOBALS_LOCK.is_locked()
        }
    }
}

#[cfg(all(test, feature = "embed"))]
mod tests {
    use super::*;
    use crate::{embed::Embed, zend::SapiGlobals};

    fn headers() -> String {
        Embed::eval("implode('|', preg_grep('/^X-Test/', headers_list()));")
            .ok()
            .and_then(|list| list.string())
            .unwrap_or_default()
    }

    #[test]
    fn set_header_replaces_same_name() {
        let headers = Embed::run(|| {
            set_header("X-Test: a").expect("set_header failed");
            set_header("X-Test: b").expect("set_header failed");
            headers()
        });

        assert_eq!(headers, "X-Test: b");
    }

    #[test]
    fn add_header_keeps_same_name() {
        let headers = Embed::run(|| {
            add_header("X-Test: a").expect("add_header failed");
            add_header("X-Test: b").expect("add_header failed");
            headers()
        });

        assert_eq!(headers, "X-Test: a|X-Test: b");
    }

    #[test]
    fn remove_header_drops_name() {
        let headers = Embed::run(|| {
            set_header("X-Test-Keep: a").expect("set_header failed");
            set_header("X-Test-Drop: b").expect("set_header failed");
            remove_header("X-Test-Drop").expect("remove_header failed");
            headers()
        });

        assert_eq!(headers, "X-Test-Keep: a");
    }

    #[test]
    fn remove_all_headers_empties_list() {
        let headers = Embed::run(|| {
            set_header("X-Test: a").expect("set_header failed");
            remove_all_headers().expect("remove_all_headers failed");
            headers()
        });

        assert_eq!(headers, "");
    }

    #[test]
    fn set_response_code_sets_status() {
        let code = Embed::run(|| {
            set_response_code(404).expect("set_response_code failed");
            Embed::eval("http_response_code();")
                .ok()
                .and_then(|code| code.long())
        });

        assert_eq!(code, Some(404));
    }

    #[test]
    fn header_with_new_line_is_refused() {
        let refused = Embed::run(|| {
            matches!(
                set_header("X-Test: a\r\nX-Injected: b"),
                Err(Error::ResponseHeaderFailed)
            )
        });

        assert!(refused);
    }

    #[test]
    fn empty_header_is_refused() {
        let refused = Embed::run(|| matches!(set_header(""), Err(Error::ResponseHeaderFailed)));

        assert!(refused);
    }

    #[test]
    fn alive_sapi_globals_guard_refuses_change() {
        let refused = Embed::run(|| {
            let _guard = SapiGlobals::get();
            matches!(set_header("X-Test: a"), Err(Error::ResponseHeaderFailed))
        });

        assert!(refused);
    }
}
