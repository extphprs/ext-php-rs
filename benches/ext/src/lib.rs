#![cfg_attr(windows, feature(abi_vectorcall))]
#![allow(
    clippy::must_use_candidate,
    clippy::missing_panics_doc,
    clippy::needless_pass_by_value,
    clippy::implicit_hasher
)]

use ext_php_rs::{
    boxed::ZBox,
    prelude::*,
    types::{PhpRef, ZendStr},
};

#[php_function]
pub fn bench_function(n: u64) -> u64 {
    n
}

#[php_function]
pub fn bench_callback_function(callback: ZendCallable, n: usize) {
    for i in 0..n {
        callback
            .try_call(vec![&i])
            .expect("Failed to call function");
    }
}

#[php_class]
pub struct BenchClass;

#[php_impl]
impl BenchClass {
    pub fn __construct() -> Self {
        Self
    }

    pub fn method(&self, n: u64) -> u64 {
        n
    }

    pub fn static_method(n: u64) -> u64 {
        n
    }
}

#[php_class]
pub struct BenchProps {
    #[php(prop)]
    pub field_a: i64,
    #[php(prop)]
    pub field_b: String,
    #[php(prop)]
    pub field_c: bool,
    inner_value: i64,
}

#[php_impl]
impl BenchProps {
    pub fn __construct(a: i64, b: String) -> Self {
        Self {
            field_a: a,
            field_b: b,
            field_c: true,
            inner_value: a * 2,
        }
    }

    #[php(getter)]
    pub fn get_computed(&self) -> i64 {
        self.inner_value
    }

    #[php(setter)]
    pub fn set_computed(&mut self, val: i64) {
        self.inner_value = val;
    }
}

#[php_function]
pub fn bench_array_with_str_ref_keys(mut array: PhpRef, n: u64) {
    let Some(array) = array.array_mut() else {
        return;
    };

    for i in 0..n {
        let _ = array.insert("key0", i);
        let _ = array.insert("key1", i);
        let _ = array.insert("key2", i);
        let _ = array.insert("key3", i);
        let _ = array.insert("key4", i);
    }
}

#[php_function]
pub fn bench_array_with_interned_keys(mut array: PhpRef, n: u64) {
    let Some(array) = array.array_mut() else {
        return;
    };

    for i in 0..n {
        let _ = array.insert(INTERNED_KEYS.get().key0.as_ref().unwrap(), i);
        let _ = array.insert(INTERNED_KEYS.get().key1.as_ref().unwrap(), i);
        let _ = array.insert(INTERNED_KEYS.get().key2.as_ref().unwrap(), i);
        let _ = array.insert(INTERNED_KEYS.get().key3.as_ref().unwrap(), i);
        let _ = array.insert(INTERNED_KEYS.get().key4.as_ref().unwrap(), i);
    }
}

#[derive(Default)]
struct InternedKeys {
    key0: Option<ZBox<ZendStr>>,
    key1: Option<ZBox<ZendStr>>,
    key2: Option<ZBox<ZendStr>>,
    key3: Option<ZBox<ZendStr>>,
    key4: Option<ZBox<ZendStr>>,
}

impl ModuleGlobal for InternedKeys {
    fn ginit(&mut self) {
        self.key0 = Some(ZendStr::new_interned("key0", true));
        self.key1 = Some(ZendStr::new_interned("key1", true));
        self.key2 = Some(ZendStr::new_interned("key2", true));
        self.key3 = Some(ZendStr::new_interned("key3", true));
        self.key4 = Some(ZendStr::new_interned("key4", true));
    }
}

static INTERNED_KEYS: ModuleGlobals<InternedKeys> = ModuleGlobals::new();

#[php_module]
pub fn build_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .function(wrap_function!(bench_function))
        .function(wrap_function!(bench_callback_function))
        .function(wrap_function!(bench_array_with_str_ref_keys))
        .function(wrap_function!(bench_array_with_interned_keys))
        .class::<BenchClass>()
        .class::<BenchProps>()
        .globals(&INTERNED_KEYS)
}
