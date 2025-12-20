#![allow(clippy::unused_self)]
use ext_php_rs::{
    convert::IntoZval,
    prelude::*,
    types::{ZendClassObject, Zval},
    zend::ce,
};

/// Doc comment
/// Goes here
#[php_class]
pub struct TestClass {
    string: String,
    number: i32,
    #[php(prop)]
    boolean_prop: bool,
}

#[php_impl]
impl TestClass {
    #[php(getter)]
    pub fn get_string(&self) -> String {
        self.string.clone()
    }

    #[php(setter)]
    pub fn set_string(&mut self, string: String) {
        self.string = string;
    }

    #[php(getter)]
    pub fn get_number(&self) -> i32 {
        self.number
    }

    #[php(setter)]
    pub fn set_number(&mut self, number: i32) {
        self.number = number;
    }

    pub fn static_call(name: String) -> String {
        format!("Hello {name}")
    }

    pub fn self_ref(
        self_: &mut ZendClassObject<TestClass>,
        val: String,
    ) -> &mut ZendClassObject<TestClass> {
        self_.string = format!("Changed to {val}");
        self_
    }

    pub fn self_multi_ref<'a>(
        self_: &'a mut ZendClassObject<TestClass>,
        val: &str,
    ) -> &'a mut ZendClassObject<TestClass> {
        self_.string = format!("Changed to {val}");
        self_
    }
}

#[php_function]
pub fn test_class(string: String, number: i32) -> TestClass {
    TestClass {
        string,
        number,
        boolean_prop: true,
    }
}

#[php_class]
#[php(implements(ce = ce::arrayaccess, stub = "ArrayAccess"))]
pub struct TestClassArrayAccess {}

#[php_impl]
impl TestClassArrayAccess {
    /// Constructor
    /// doc
    /// comment
    pub fn __construct() -> Self {
        Self {}
    }

    // We need to use `Zval` because ArrayAccess needs $offset to be a `mixed`
    pub fn offset_exists(&self, offset: &'_ Zval) -> bool {
        offset.is_long()
    }
    pub fn offset_get(&self, offset: &'_ Zval) -> PhpResult<bool> {
        let integer_offset = offset.long().ok_or("Expected integer offset")?;
        Ok(integer_offset % 2 == 0)
    }
    pub fn offset_set(&mut self, _offset: &'_ Zval, _value: &'_ Zval) -> PhpResult {
        Err("Setting values is not supported".into())
    }
    pub fn offset_unset(&mut self, _offset: &'_ Zval) -> PhpResult {
        Err("Setting values is not supported".into())
    }
}

#[php_class]
#[php(extends(ce = ce::exception, stub = "\\Exception"))]
#[derive(Default)]
pub struct TestClassExtends;

#[php_impl]
impl TestClassExtends {
    pub fn __construct() -> Self {
        Self {}
    }
}

#[php_function]
pub fn throw_exception() -> PhpResult<i32> {
    Err(
        PhpException::from_class::<TestClassExtends>("Not good!".into())
            .with_object(TestClassExtends.into_zval(false)?),
    )
}

#[php_class]
#[php(implements(ce = ce::arrayaccess, stub = "ArrayAccess"))]
#[php(extends(ce = ce::exception, stub = "\\Exception"))]
pub struct TestClassExtendsImpl;

#[php_impl]
impl TestClassExtendsImpl {
    pub fn __construct() -> Self {
        Self {}
    }

    // We need to use `Zval` because ArrayAccess needs $offset to be a `mixed`
    pub fn offset_exists(&self, offset: &'_ Zval) -> bool {
        offset.is_long()
    }
    pub fn offset_get(&self, offset: &'_ Zval) -> PhpResult<bool> {
        let integer_offset = offset.long().ok_or("Expected integer offset")?;
        Ok(integer_offset % 2 == 0)
    }
    pub fn offset_set(&mut self, _offset: &'_ Zval, _value: &'_ Zval) -> PhpResult {
        Err("Setting values is not supported".into())
    }
    pub fn offset_unset(&mut self, _offset: &'_ Zval) -> PhpResult {
        Err("Setting values is not supported".into())
    }
}

#[php_class]
struct TestClassMethodVisibility;

#[php_impl]
impl TestClassMethodVisibility {
    #[php(vis = "private")]
    fn __construct() -> Self {
        Self
    }

    #[php(vis = "private")]
    fn private() -> u32 {
        3
    }

    #[php(vis = "protected")]
    fn protected() -> u32 {
        3
    }
}
#[php_class]
struct TestClassProtectedConstruct;

#[php_impl]
impl TestClassProtectedConstruct {
    #[php(vis = "protected")]
    fn __construct() -> Self {
        Self
    }
}

/// Test class for issue #325 - returning &'static str from getter
#[php_class]
pub struct TestClassStaticStrGetter;

#[php_impl]
impl TestClassStaticStrGetter {
    pub fn __construct() -> Self {
        Self
    }

    /// This getter returns a &'static str which previously failed to compile
    /// due to "implementation of `FromZval` is not general enough" error.
    #[php(getter)]
    pub fn get_static_value(&self) -> &'static str {
        "Hello from static str"
    }
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .class::<TestClass>()
        .class::<TestClassArrayAccess>()
        .class::<TestClassExtends>()
        .class::<TestClassExtendsImpl>()
        .class::<TestClassMethodVisibility>()
        .class::<TestClassProtectedConstruct>()
        .class::<TestClassStaticStrGetter>()
        .function(wrap_function!(test_class))
        .function(wrap_function!(throw_exception))
}

#[cfg(test)]
mod tests {
    #[test]
    fn class_works() {
        assert!(crate::integration::test::run_php("class/class.php"));
    }
}
