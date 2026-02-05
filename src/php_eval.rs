//! Execute embedded PHP code within a running PHP extension.
//!
//! This module provides a way to compile and execute PHP code that has been
//! embedded into the extension binary at compile time using `include_bytes!`.
//!
//! Uses `zend_compile_string` + `zend_execute` (not `zend_eval_string`)
//! to avoid security scanner false positives and compatibility issues
//! with hardened PHP configurations.
//!
//! # Example
//!
//! ```rust,ignore
//! use ext_php_rs::php_eval;
//!
//! // Both include_bytes! and include_str! are supported:
//! const SETUP_BYTES: &[u8] = include_bytes!("../php/setup.php");
//! const SETUP_STR: &str = include_str!("../php/setup.php");
//!
//! php_eval::execute(SETUP_BYTES).expect("failed to execute embedded PHP");
//! php_eval::execute(SETUP_STR).expect("failed to execute embedded PHP");
//! ```

use crate::ffi;
use crate::types::ZendStr;
use crate::zend::try_catch;
use std::fmt;
use std::mem;
use std::panic::AssertUnwindSafe;

/// Errors that can occur when executing embedded PHP code.
#[derive(Debug)]
pub enum PhpEvalError {
    /// The code does not start with a `<?php` open tag.
    MissingOpenTag,
    /// PHP failed to compile the code (syntax error).
    CompilationFailed,
    /// The code executed but threw an unhandled exception.
    ExecutionFailed,
    /// A PHP fatal error (bailout) occurred during execution.
    Bailout,
}

impl fmt::Display for PhpEvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhpEvalError::MissingOpenTag => {
                write!(f, "PHP code must start with a <?php open tag")
            }
            PhpEvalError::CompilationFailed => write!(f, "PHP compilation failed (syntax error)"),
            PhpEvalError::ExecutionFailed => {
                write!(f, "PHP execution threw an unhandled exception")
            }
            PhpEvalError::Bailout => write!(f, "PHP fatal error (bailout) during execution"),
        }
    }
}

impl std::error::Error for PhpEvalError {}

/// Execute embedded PHP code within the running PHP engine.
///
/// The code **must** start with a `<?php` opening tag (case-insensitive),
/// optionally preceded by a UTF-8 BOM and/or whitespace. The tag is
/// stripped before compilation. The C wrapper uses
/// `ZEND_COMPILE_POSITION_AFTER_OPEN_TAG` (on PHP 8.2+) so the scanner
/// starts directly in PHP mode.
///
/// Error reporting is suppressed during execution and restored afterward,
/// matching the pattern used by production PHP extensions like Blackfire.
///
/// # Arguments
///
/// * `code` - Raw PHP source, typically from `include_bytes!` or
///   `include_str!`. Any type implementing `AsRef<[u8]>` is accepted.
///
/// # Errors
///
/// Returns [`PhpEvalError::MissingOpenTag`] if the code does not start
/// with `<?php`. Returns other [`PhpEvalError`] variants if compilation
/// fails, an exception is thrown, or a fatal error occurs.
pub fn execute(code: impl AsRef<[u8]>) -> Result<(), PhpEvalError> {
    let code = strip_bom(code.as_ref());
    let code = strip_php_open_tag(code).ok_or(PhpEvalError::MissingOpenTag)?;

    if code.is_empty() {
        return Ok(());
    }

    let source = ZendStr::new(code, false);

    // Suppress error reporting so compilation warnings from embedded
    // code don't bubble up to the application's error handler.
    // Saved outside `try_catch` so it is always restored, even on bailout.
    let eg = unsafe { ffi::ext_php_rs_executor_globals() };
    let prev_error_reporting = unsafe { mem::replace(&mut (*eg).error_reporting, 0) };

    let result = try_catch(AssertUnwindSafe(|| unsafe {
        let op_array = ffi::ext_php_rs_zend_compile_string(
            source.as_ptr().cast_mut(),
            c"embedded_php".as_ptr(),
        );

        if op_array.is_null() {
            return Err(PhpEvalError::CompilationFailed);
        }

        ffi::ext_php_rs_zend_execute(op_array);

        if !(*eg).exception.is_null() {
            return Err(PhpEvalError::ExecutionFailed);
        }

        Ok(())
    }));

    unsafe { (*eg).error_reporting = prev_error_reporting };

    match result {
        Err(_) => Err(PhpEvalError::Bailout),
        Ok(inner) => inner,
    }
}

fn strip_bom(code: &[u8]) -> &[u8] {
    if code.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &code[3..]
    } else {
        code
    }
}

fn strip_php_open_tag(code: &[u8]) -> Option<&[u8]> {
    let trimmed = match code.iter().position(|b| !b.is_ascii_whitespace()) {
        Some(pos) => &code[pos..],
        None => return None,
    };

    if trimmed.len() >= 5 && trimmed[..5].eq_ignore_ascii_case(b"<?php") {
        Some(trimmed[5..].trim_ascii_start())
    } else {
        None
    }
}

#[cfg(feature = "embed")]
#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::embed::Embed;

    #[test]
    fn test_execute_with_php_open_tag() {
        Embed::run(|| {
            let result = execute(b"<?php $x = 42;");
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_execute_with_php_open_tag_and_newline() {
        Embed::run(|| {
            let result = execute(b"<?php\n$x = 42;");
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_execute_tag_only() {
        Embed::run(|| {
            let result = execute(b"<?php");
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_execute_exception() {
        Embed::run(|| {
            let result = execute(b"<?php throw new \\RuntimeException('test');");
            assert!(matches!(result, Err(PhpEvalError::ExecutionFailed)));
        });
    }

    #[test]
    fn test_execute_missing_open_tag() {
        Embed::run(|| {
            let result = execute(b"$x = 1 + 2;");
            assert!(matches!(result, Err(PhpEvalError::MissingOpenTag)));
        });
    }

    #[test]
    fn test_execute_compilation_error() {
        Embed::run(|| {
            let result = execute(b"<?php this is not valid php {{{");
            assert!(matches!(result, Err(PhpEvalError::CompilationFailed)));
        });
    }

    #[test]
    fn test_execute_with_bom() {
        Embed::run(|| {
            let mut code = vec![0xEF, 0xBB, 0xBF];
            code.extend_from_slice(b"<?php $x = 'bom_test';");
            let result = execute(&code);
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_execute_defines_variable() {
        Embed::run(|| {
            let result = execute(b"<?php $embed_test = 'hello from embedded php';");
            assert!(result.is_ok());

            let val = Embed::eval("$embed_test;");
            assert!(val.is_ok());
            assert_eq!(val.unwrap().string().unwrap(), "hello from embedded php");
        });
    }

    #[test]
    fn test_execute_empty_code() {
        Embed::run(|| {
            let result = execute(b"");
            assert!(matches!(result, Err(PhpEvalError::MissingOpenTag)));
        });
    }

    #[test]
    fn test_execute_include_bytes_pattern() {
        Embed::run(|| {
            let code: &[u8] = b"<?php\n\
                $embedded_value = 42;\n\
                define('EMBEDDED_CONST', true);\n";
            let result = execute(code);
            assert!(result.is_ok());
        });
    }

    #[test]
    fn test_execute_with_str() {
        Embed::run(|| {
            let code: &str = "<?php $str_test = 'from_str';";
            let result = execute(code);
            assert!(result.is_ok());

            let val = Embed::eval("$str_test;");
            assert!(val.is_ok());
            assert_eq!(val.unwrap().string().unwrap(), "from_str");
        });
    }

    #[test]
    fn test_execute_with_string() {
        Embed::run(|| {
            let code = String::from("<?php $string_test = 'from_string';");
            let result = execute(code);
            assert!(result.is_ok());

            let val = Embed::eval("$string_test;");
            assert!(val.is_ok());
            assert_eq!(val.unwrap().string().unwrap(), "from_string");
        });
    }

    #[test]
    fn test_execute_with_vec() {
        Embed::run(|| {
            let code: Vec<u8> = b"<?php $vec_test = 'from_vec';".to_vec();
            let result = execute(code);
            assert!(result.is_ok());

            let val = Embed::eval("$vec_test;");
            assert!(val.is_ok());
            assert_eq!(val.unwrap().string().unwrap(), "from_vec");
        });
    }

    #[test]
    fn test_strip_bom() {
        let cases: &[(&[u8], &[u8])] = &[
            (&[0xEF, 0xBB, 0xBF, b'h', b'i'], b"hi"),
            (b"hello", b"hello"),
            (b"", b""),
        ];
        for (input, expected) in cases {
            assert_eq!(
                super::strip_bom(input),
                *expected,
                "input: {:?}",
                String::from_utf8_lossy(input)
            );
        }
    }

    #[test]
    fn test_strip_php_open_tag() {
        let cases: &[(&[u8], Option<&[u8]>)] = &[
            (b"<?php $x;", Some(b"$x;")),
            (b"<?php\n$x;", Some(b"$x;")),
            (b"<?php\r\n$x;", Some(b"$x;")),
            (b"<?php\t\n  $x;", Some(b"$x;")),
            (b"<?php", Some(b"")),
            (b"  <?php $x;", Some(b"$x;")),
            (b"<?PHP $x;", Some(b"$x;")),
            (b"<?Php\n$x;", Some(b"$x;")),
            (b"", None),
            (b"   ", None),
            (b"$x = 1;", None),
            (b"hello", None),
        ];
        for (input, expected) in cases {
            assert_eq!(
                super::strip_php_open_tag(input),
                *expected,
                "input: {:?}",
                String::from_utf8_lossy(input)
            );
        }
    }
}
