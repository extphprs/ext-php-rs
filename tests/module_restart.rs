//! A SAPI can shut the module down and start it again in the same process:
//! `FrankenPHP` does on `POST /frankenphp/workers/restart` and on
//! `opcache_reset()`. Every MINIT must register the module again.
#![cfg_attr(windows, feature(abi_vectorcall))]
#![cfg(all(feature = "embed", feature = "closure"))]
#![allow(
    missing_docs,
    clippy::needless_pass_by_value,
    clippy::must_use_candidate
)]
extern crate ext_php_rs;

use ext_php_rs::builders::SapiBuilder;
use ext_php_rs::embed::{
    Embed, cleanup_sapi_allocations, ext_php_rs_sapi_shutdown, ext_php_rs_sapi_startup,
};
use ext_php_rs::ffi::{
    ZEND_RESULT_CODE_SUCCESS, php_module_shutdown, php_module_startup, php_request_shutdown,
    php_request_startup, sapi_shutdown, sapi_startup,
};
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;
use ext_php_rs::zend::try_catch_first;

#[php_class]
#[php(name = "Restart\\Greeter")]
pub struct Greeter {
    name: String,
}

#[php_impl]
impl Greeter {
    pub fn __construct(name: String) -> Self {
        Self { name }
    }

    pub fn greet(&self, greeting: String) -> String {
        format!("{greeting}, {}!", self.name)
    }
}

#[php_interface]
#[php(name = "Restart\\Named")]
pub trait Named {
    fn name(&self) -> String;
}

#[php_enum]
#[php(name = "Restart\\Suit")]
pub enum Suit {
    #[php(value = "H")]
    Hearts,
    #[php(value = "S")]
    Spades,
}

#[php_function]
pub fn restart_shout() -> Closure {
    Closure::wrap(Box::new(|s: String| s.to_uppercase()) as Box<dyn Fn(String) -> String>)
}

#[php_module]
pub fn module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .constant(("RESTART_ANSWER", 42, &[]))
        .class::<Greeter>()
        .interface::<PhpInterfaceNamed>()
        .enumeration::<Suit>()
        .function(wrap_function!(restart_shout))
}

fn eval(code: &str) -> Zval {
    Embed::eval(code).unwrap_or_else(|e| panic!("`{code}` failed: {e:?}"))
}

fn run_module_once() {
    let mut sapi = SapiBuilder::new("restart", "Restart").build().unwrap();
    let sapi = &raw mut sapi;

    unsafe {
        ext_php_rs_sapi_startup();
        sapi_startup(sapi);
        assert_eq!(
            php_module_startup(sapi, get_module()),
            ZEND_RESULT_CODE_SUCCESS
        );
    }

    assert_eq!(unsafe { php_request_startup() }, ZEND_RESULT_CODE_SUCCESS);

    try_catch_first(|| {
        let greeting = eval("(new Restart\\Greeter('world'))->greet('Hello');");
        assert_eq!(greeting.string().as_deref(), Some("Hello, world!"));

        assert_eq!(
            eval("interface_exists(Restart\\Named::class);").bool(),
            Some(true)
        );

        let suit = eval("Restart\\Suit::from('S')->name;");
        assert_eq!(suit.string().as_deref(), Some("Spades"));

        assert_eq!(eval("RESTART_ANSWER;").long(), Some(42));

        let shout = eval("restart_shout()('hi');");
        assert_eq!(shout.string().as_deref(), Some("HI"));
    })
    .expect("PHP bailed out");

    unsafe {
        php_request_shutdown(std::ptr::null_mut());
        php_module_shutdown();
        sapi_shutdown();
        ext_php_rs_sapi_shutdown();
        cleanup_sapi_allocations(sapi);
    }
}

#[test]
fn test_module_is_registered_again_when_restarted() {
    // One thread per lifecycle, as FrankenPHP starts a new main thread: TSRM
    // is never started again on a thread that shut it down.
    for _ in 0..3 {
        std::thread::spawn(run_module_once)
            .join()
            .expect("the module should start, serve a request and stop");
    }
}
