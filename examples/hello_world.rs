#![allow(missing_docs, clippy::must_use_candidate)]
#![cfg_attr(windows, feature(abi_vectorcall))]
use ext_php_rs::{constant::IntoConst, prelude::*, types::ZendClassObject};

#[derive(Debug)]
#[php_class]
pub struct TestClass {
    #[php(prop)]
    a: i32,
    #[php(prop)]
    b: i32,
    #[php(prop)]
    name: String,
    /// An optional nickname.
    #[php(prop)]
    optional: Option<String>,
    #[php(prop, static, default = 100)]
    #[allow(dead_code)]
    max_limit: i32,
}

#[php_impl]
impl TestClass {
    #[php(name = "NEW_CONSTANT_NAME")]
    pub const SOME_CONSTANT: i32 = 5;
    pub const SOME_OTHER_STR: &'static str = "Hello, world!";

    pub fn __construct(a: i32, b: i32, name: String) -> Self {
        Self {
            a: a + 10,
            b: b + 10,
            name,
            optional: None,
            max_limit: 100,
        }
    }

    #[php(getter)]
    pub fn get_tags(&self) -> Vec<bool> {
        vec![true, false]
    }

    #[php(defaults(a = 5, test = 100))]
    pub fn test_camel_case(&self, a: i32, test: i32) {
        println!("a: {a} test: {test}");
    }

    fn x() -> i32 {
        5
    }

    pub fn builder_pattern(
        self_: &mut ZendClassObject<TestClass>,
    ) -> &mut ZendClassObject<TestClass> {
        dbg!(self_)
    }
}

#[php_function]
pub fn new_class() -> TestClass {
    TestClass {
        a: 1,
        b: 2,
        name: "default".into(),
        optional: None,
        max_limit: 100,
    }
}

#[php_function]
pub fn hello_world() -> &'static str {
    "Hello, world!"
}

#[php_const]
pub const HELLO_WORLD: i32 = 100;

#[php_extern]
extern "C" {
    fn phpinfo() -> bool;
}

#[derive(Debug, ZvalConvert)]
pub struct TestZvalConvert<'a> {
    a: i32,
    b: i32,
    c: &'a str,
}

#[php_function]
pub fn get_zval_convert(z: TestZvalConvert) -> i32 {
    dbg!(z);
    5
}

fn startup(_ty: i32, mod_num: i32) -> i32 {
    5.register_constant("SOME_CONST", mod_num).unwrap();
    0
}

#[php_module]
#[php(startup = startup)]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .class::<TestClass>()
        .function(wrap_function!(hello_world))
        .function(wrap_function!(new_class))
        .function(wrap_function!(get_zval_convert))
        .constant(wrap_constant!(HELLO_WORLD))
        .constant(("CONST_NAME", HELLO_WORLD, &[]))
}
