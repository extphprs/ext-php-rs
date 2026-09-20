//! A Rust panic reached from PHP must never cross the `extern "C"` boundary.
//! Every fixture here panics on purpose; the PHP side asserts that it sees a
//! PHP `Error` whose message starts with `Rust panic: ` and that the engine
//! keeps running afterwards.
#![allow(clippy::unused_self)]
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use ext_php_rs::{prelude::*, types::ZendCallable, zend::BailoutGuard, zend::ce};

#[php_function]
pub fn panic_test_function() {
    panic!("function panicked");
}

#[php_function]
pub fn panic_test_healthy() -> i32 {
    42
}

#[php_function]
pub fn panic_test_payload() {
    std::panic::panic_any(42_u8);
}

#[php_function]
pub fn panic_test_after_throw() {
    PhpException::new("thrown first".into(), 0, ce::type_error()).throw();
    panic!("then panicked");
}

static NESTED_RESULT: Mutex<String> = Mutex::new(String::new());

#[php_function]
pub fn panic_test_nested(callback: ZendCallable) {
    let outcome = match callback.try_call(vec![]) {
        Ok(_) => "no exception".to_owned(),
        Err(ext_php_rs::error::Error::ExceptionPending { class }) => format!("pending: {class}"),
        Err(other) => format!("other: {other}"),
    };
    *NESTED_RESULT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = outcome;
}

#[php_function]
pub fn panic_test_nested_result() -> String {
    NESTED_RESULT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

static GUARD_DROPS: AtomicU32 = AtomicU32::new(0);

struct DropCounter;

impl Drop for DropCounter {
    fn drop(&mut self) {
        GUARD_DROPS.fetch_add(1, Ordering::SeqCst);
    }
}

#[php_function]
pub fn panic_test_guard() {
    let _guarded = BailoutGuard::new(DropCounter);
    let _plain = DropCounter;
    panic!("guarded panic");
}

#[php_function]
pub fn panic_test_guard_drops() -> u32 {
    GUARD_DROPS.load(Ordering::SeqCst)
}

#[php_extern]
extern "C" {
    fn strpos(haystack: &str, needle: &str) -> i64;
}

#[php_function]
pub fn panic_test_extern() -> i64 {
    unsafe { strpos("abc", "z") }
}

#[php_interface]
#[php(name = "PanicIface")]
#[allow(dead_code)]
pub trait PanicIface {
    fn iface_method(&self) -> i32;
}

#[php_class]
#[php(name = "PanicClass")]
pub struct PanicClass {
    #[php(prop)]
    pub plain: i32,
}

#[php_impl]
impl PanicClass {
    pub fn __construct(explode: bool) -> Self {
        assert!(!explode, "constructor panicked");
        Self { plain: 7 }
    }

    pub fn method(&self) {
        panic!("method panicked");
    }

    pub fn static_method() {
        panic!("static method panicked");
    }

    #[php(getter)]
    pub fn get_boom(&self) -> i32 {
        panic!("getter panicked");
    }

    #[php(setter)]
    pub fn set_boom(&mut self, _value: i32) {
        panic!("setter panicked");
    }

    pub fn healthy(&self) -> i32 {
        self.plain
    }
}

#[php_impl_interface]
impl PanicIface for PanicClass {
    fn iface_method(&self) -> i32 {
        panic!("interface method panicked");
    }
}

struct ExplodingOnClone;

impl Clone for ExplodingOnClone {
    fn clone(&self) -> Self {
        panic!("clone panicked");
    }
}

#[php_class]
#[php(name = "PanicClone")]
#[derive(Clone)]
pub struct PanicClone {
    _inner: ExplodingOnClone,
}

#[php_impl]
impl PanicClone {
    pub fn __construct() -> Self {
        Self {
            _inner: ExplodingOnClone,
        }
    }
}

#[php_class]
#[php(name = "PanicDrop")]
pub struct PanicDrop;

impl Drop for PanicDrop {
    fn drop(&mut self) {
        panic!("drop panicked");
    }
}

#[php_impl]
impl PanicDrop {
    pub fn __construct() -> Self {
        Self
    }
}

#[cfg(feature = "closure")]
#[php_function]
pub fn panic_test_closure() -> Closure {
    Closure::wrap(Box::new(|| -> i32 { panic!("closure panicked") }) as Box<dyn Fn() -> i32>)
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    let builder = builder
        .function(wrap_function!(panic_test_function))
        .function(wrap_function!(panic_test_healthy))
        .function(wrap_function!(panic_test_payload))
        .function(wrap_function!(panic_test_after_throw))
        .function(wrap_function!(panic_test_nested))
        .function(wrap_function!(panic_test_nested_result))
        .function(wrap_function!(panic_test_guard))
        .function(wrap_function!(panic_test_guard_drops))
        .function(wrap_function!(panic_test_extern))
        .interface::<PhpInterfacePanicIface>()
        .class::<PanicClass>()
        .class::<PanicClone>()
        .class::<PanicDrop>();
    #[cfg(feature = "closure")]
    let builder = builder.function(wrap_function!(panic_test_closure));
    builder
}

#[cfg(test)]
mod tests {
    use crate::integration::test::run_php_capturing_stderr;

    fn panic_script_survives(file: &str) {
        let stderr = run_php_capturing_stderr(&format!("panic/{file}"));
        assert!(
            stderr.contains("panicked at"),
            "the panic hook never fired for {file}:\n{stderr}"
        );
    }

    #[test]
    fn function_panic_becomes_error() {
        panic_script_survives("panic_function.php");
    }

    #[test]
    fn method_panics_become_error() {
        panic_script_survives("panic_method.php");
    }

    #[test]
    fn constructor_panic_becomes_error() {
        panic_script_survives("panic_constructor.php");
    }

    #[test]
    fn property_handler_panics_become_error() {
        panic_script_survives("panic_property.php");
    }

    #[test]
    fn clone_panic_becomes_error() {
        panic_script_survives("panic_clone.php");
    }

    #[test]
    fn drop_panic_becomes_warning() {
        panic_script_survives("panic_drop.php");
    }

    #[cfg(feature = "closure")]
    #[test]
    fn closure_panic_becomes_error() {
        panic_script_survives("panic_closure.php");
    }

    #[test]
    fn pending_exception_wins_over_panic() {
        panic_script_survives("panic_after_throw.php");
    }

    #[test]
    fn non_string_payload_has_fallback_text() {
        panic_script_survives("panic_payload.php");
    }

    #[test]
    fn extern_failure_becomes_error() {
        panic_script_survives("panic_extern.php");
    }

    #[test]
    fn nested_call_chain_survives() {
        panic_script_survives("panic_nested.php");
    }

    #[test]
    fn bailout_guard_drops_once_on_panic() {
        panic_script_survives("panic_guard.php");
    }

    fn module_refused_at_startup(crate_name: &str, logged: &str) {
        let (status, output) = crate::integration::test::load_broken_module(crate_name);

        assert!(!status.success(), "{output}");
        assert!(
            status.code().is_some(),
            "php was killed by a signal: {status}\n{output}"
        );
        assert!(output.contains(logged), "{output}");
        assert!(
            output.contains(&format!("Unable to start {crate_name} module")),
            "{output}"
        );
        assert!(!output.contains("panicked"), "{output}");
        assert!(!output.contains("alive"), "{output}");
    }

    #[test]
    fn unbuildable_module_fails_minit_instead_of_aborting() {
        module_refused_at_startup("broken-module", "module could not be built");
    }

    #[test]
    fn failing_registration_fails_minit_instead_of_aborting() {
        module_refused_at_startup("broken-minit", "module startup failed");
    }
}
