#![allow(missing_docs, clippy::must_use_candidate)]
#![cfg_attr(windows, feature(abi_vectorcall))]
use ext_php_rs::{constant::IntoConst, prelude::*, types::ZendClassObject};

/// A simple test class demonstrating ext-php-rs features.
///
/// This class showcases property definitions, constants, and various
/// method types including constructors and static methods.
#[derive(Debug)]
#[php_class]
pub struct TestClass {
    #[php(prop)]
    a: i32,
    #[php(prop)]
    b: i32,
}

#[php_impl]
impl TestClass {
    #[php(name = "NEW_CONSTANT_NAME")]
    pub const SOME_CONSTANT: i32 = 5;
    pub const SOME_OTHER_STR: &'static str = "Hello, world!";

    /// Creates a new `TestClass` instance.
    ///
    /// Both values are incremented by 10 before being stored.
    ///
    /// # Arguments
    ///
    /// * `a` - First value to store
    /// * `b` - Second value to store
    pub fn __construct(a: i32, b: i32) -> Self {
        Self {
            a: a + 10,
            b: b + 10,
        }
    }

    /// Tests camelCase conversion and default parameter values.
    ///
    /// # Arguments
    ///
    /// * `a` - First parameter with default value 5
    /// * `test` - Second parameter with default value 100
    #[php(defaults(a = 5, test = 100))]
    pub fn test_camel_case(&self, a: i32, test: i32) {
        println!("a: {a} test: {test}");
    }

    /// Returns a static value.
    ///
    /// # Returns
    ///
    /// Always returns 5.
    fn x() -> i32 {
        5
    }

    /// Demonstrates the builder pattern by returning self.
    ///
    /// # Returns
    ///
    /// Returns the same instance for method chaining.
    pub fn builder_pattern(
        self_: &mut ZendClassObject<TestClass>,
    ) -> &mut ZendClassObject<TestClass> {
        dbg!(self_)
    }
}

/// Creates a new `TestClass` instance with default values.
///
/// # Returns
///
/// A `TestClass` with a=1 and b=2.
#[php_function]
pub fn new_class() -> TestClass {
    TestClass { a: 1, b: 2 }
}

/// Returns a friendly greeting.
///
/// # Returns
///
/// The string "Hello, world!".
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

/// Demonstrates `ZvalConvert` derive macro usage.
///
/// # Arguments
///
/// * `z` - An object that will be converted from a PHP value
///
/// # Returns
///
/// Always returns 5.
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
