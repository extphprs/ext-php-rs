#![allow(missing_docs, clippy::must_use_candidate)]
#![cfg_attr(windows, feature(abi_vectorcall))]
use ext_php_rs::{
    constant::IntoConst,
    prelude::*,
    types::{ZendClassObject, Zval},
};

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

/// Demonstrates compound PHP type hints. The argument accepts `int|string`
/// and the return type registers as `int|string|null`. Both strings are
/// parsed at macro-expansion time, so a typo such as `?Foo&Bar` would
/// fail at `cargo build` rather than at extension load.
#[php_function]
#[php(returns = "int|string|null")]
pub fn flexible_id(#[php(types = "int|string")] _value: &Zval) -> Option<i64> {
    None
}

/// Companion to `flexible_id` showing that the same compile-time parsing
/// works for class-side type strings. The literal `\TestClass|\OtherTestClass`
/// is parsed at macro-expansion time and resolves the class names against
/// PHP's global namespace at extension load. Use a leading `\` for the
/// fully qualified name; bare `TestClass` works too because the engine
/// places `#[php_class]`-defined structs in the global namespace.
#[php_function]
pub fn accept_class_value(#[php(types = "\\TestClass|\\OtherTestClass")] _value: &Zval) {}

/// Demonstrates `#[php(returns = "...")]` widening the inferred return
/// metadata. The Rust signature returns a concrete `TestClass`, so the
/// macro would otherwise register the return type as just `\TestClass`.
/// The override widens it to `\TestClass|\OtherTestClass`, which is
/// useful when a function returns one specific subtype today but the
/// PHP-side contract should leave room for a wider set of legal
/// values. Reflection on this function reports the wider union.
#[php_function]
#[php(returns = "\\TestClass|\\OtherTestClass")]
pub fn produce_test_class_or_other() -> TestClass {
    TestClass {
        a: 0,
        b: 0,
        name: "from union".into(),
        optional: None,
        max_limit: 100,
    }
}

/// Demonstrates `#[derive(PhpUnion)]` for primitive-typed variants. The
/// derive synthesises `PhpType::Union` from `<T as IntoZval>::TYPE` of
/// each variant, so the registered metadata is `int|float` here. Use
/// this when your union is fully captured by Rust enum dispatch and
/// every variant is a primitive that already implements `IntoZval` and
/// `FromZval` on its owned form. Class-typed variants are not yet
/// supported by the derive (tracked as a slice 7 follow-up); for
/// class unions today, prefer the `#[php(returns = "\Foo|\Bar")]`
/// override shown in `produce_test_class_or_other` above.
#[derive(PhpUnion)]
pub enum IntOrFloat {
    Int(i64),
    Float(f64),
}

#[php_function]
pub fn pick_number(use_float: bool) -> IntOrFloat {
    if use_float {
        IntOrFloat::Float(2.5)
    } else {
        IntOrFloat::Int(42)
    }
}

#[php_class]
pub struct OtherTestClass;

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
        .class::<OtherTestClass>()
        .function(wrap_function!(hello_world))
        .function(wrap_function!(flexible_id))
        .function(wrap_function!(accept_class_value))
        .function(wrap_function!(produce_test_class_or_other))
        .function(wrap_function!(pick_number))
        .function(wrap_function!(new_class))
        .function(wrap_function!(get_zval_convert))
        .constant(wrap_constant!(HELLO_WORLD))
        .constant(("CONST_NAME", HELLO_WORLD, &[]))
}
