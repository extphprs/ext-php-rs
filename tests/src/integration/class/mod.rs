#![allow(clippy::unused_self)]
use ext_php_rs::{
    class::RegisteredClass,
    convert::IntoZval,
    prelude::*,
    types::{ZendClassObject, Zval},
    zend::ce,
};

#[cfg(php84)]
use ext_php_rs::types::ZendObject;

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

    /// Returns a new instance with a different string (tests returning Self)
    pub fn with_string(&self, string: String) -> Self {
        Self {
            string,
            number: self.number,
            boolean_prop: self.boolean_prop,
        }
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

/// Regression coverage for the `ZBox<ZendClassObject<T>>::set_zval` refcount
/// bug: throwing an exception built through `ZendClassObject::new(...)
/// .into_zval(...)` (rather than `T.into_zval(...)`) and carrying a
/// `#[php(prop)]` field leaks the underlying object. On a PHP debug build the
/// leak cascades into a `_zend_hash_str_add_or_update_i` assertion at module
/// shutdown.
#[php_class]
#[php(extends(ce = ce::exception, stub = "\\Exception"))]
#[derive(Default)]
pub struct TestClassExtendsWithProp {
    #[php(prop)]
    pub payload: String,
}

#[php_impl]
impl TestClassExtendsWithProp {
    pub fn __construct() -> Self {
        Self::default()
    }
}

#[php_function]
pub fn throw_class_object_exception_with_prop() -> PhpResult<i32> {
    let payload = TestClassExtendsWithProp {
        payload: "boom".to_string(),
    };
    let zval = ZendClassObject::new(payload).into_zval(false)?;
    Err(PhpException::from_class::<TestClassExtendsWithProp>("ignored".into()).with_object(zval))
}

/// Regression coverage for a refcount leak in the `#[php(prop)]` field getter
/// when the field is an owned refcounted type (here `String`) AND the property
/// is read via a C-level method that uses `zval_get_string` + `RETURN_STR`
/// (e.g. `Exception::getMessage`).
///
/// The generated getter writes a fresh `zend_string` with refcount=1 into the
/// `rv` slot. PHP's `getMessage` then calls `zval_get_string(prop)` which
/// addrefs (→2) and `RETURN_STR` transfers the pointer to `return_value`
/// without changing the refcount. When the method returns, the stack `rv`
/// goes out of scope without being dtor'd, orphaning one refcount per call.
/// Each `$e->getMessage()` therefore leaks a `zend_string`.
///
/// Surfaced first in production by biscuit-php's `DatalogException` subclasses,
/// which declare `#[php(prop, flags = Protected)] message: String` and shadow
/// the parent `\Exception::$message`.
#[php_class]
#[php(extends(ce = ce::exception, stub = "\\Exception"))]
#[derive(Default)]
pub struct TestExceptionMessageLeak {
    /// Public to keep the test focused on the refcount leak rather than the
    /// visibility-check path. The leak reproduces on any `#[php(prop)]` field
    /// whose type allocates a `zend_string` via `IntoZval`; biscuit-php's
    /// real-world trigger happens to use Protected, but the codegen bug is the
    /// same shape regardless of visibility.
    #[php(prop)]
    pub message: String,
}

#[php_impl]
impl TestExceptionMessageLeak {
    pub fn __construct() -> Self {
        Self::default()
    }
}

#[php_function]
pub fn throw_exception_with_message_prop() -> PhpResult<i32> {
    let payload = TestExceptionMessageLeak {
        message: "leak-bait message contents".to_string(),
    };
    let zval = ZendClassObject::new(payload).into_zval(false)?;
    Err(PhpException::from_class::<TestExceptionMessageLeak>("ignored".into()).with_object(zval))
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
    fn private_method() -> u32 {
        3
    }

    #[php(vis = "protected")]
    fn protected_method() -> u32 {
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

/// Test class with static properties (Issue #252)
#[php_class]
pub struct TestStaticProps {
    /// Instance property for comparison
    #[php(prop)]
    pub instance_value: i32,
    /// Static property - managed by PHP, not Rust handlers
    #[php(prop, static)]
    pub static_counter: i32,
    /// Private static property
    #[php(prop, static, flags = ext_php_rs::flags::PropertyFlags::Private)]
    pub private_static: String,
}

#[php_impl]
impl TestStaticProps {
    pub fn __construct(value: i32) -> Self {
        Self {
            instance_value: value,
            // Note: static fields have default values in PHP, not from Rust constructor
            static_counter: 0,
            private_static: String::new(),
        }
    }

    /// Static method to increment the static counter
    pub fn increment_counter() {
        let ce = Self::get_metadata().ce();
        let current: i64 = ce.get_static_property("staticCounter").unwrap_or(0);
        ce.set_static_property("staticCounter", current + 1)
            .expect("Failed to set static property");
    }

    /// Static method to get the current counter value
    pub fn get_counter() -> i64 {
        let ce = Self::get_metadata().ce();
        ce.get_static_property("staticCounter").unwrap_or(0)
    }

    /// Static method to set the counter to a specific value
    pub fn set_counter(value: i64) {
        let ce = Self::get_metadata().ce();
        ce.set_static_property("staticCounter", value)
            .expect("Failed to set static property");
    }
}

/// Test class for lazy object support (PHP 8.4+)
#[php_class]
pub struct TestLazyClass {
    #[php(prop)]
    pub data: String,
    #[php(prop)]
    pub initialized: bool,
}

#[php_impl]
impl TestLazyClass {
    pub fn __construct(data: String) -> Self {
        Self {
            data,
            initialized: true,
        }
    }
}

/// Check if a `ZendObject` is lazy
#[cfg(php84)]
#[php_function]
pub fn test_is_lazy(obj: &ZendObject) -> bool {
    obj.is_lazy()
}

/// Check if a `ZendObject` is a lazy ghost
#[cfg(php84)]
#[php_function]
pub fn test_is_lazy_ghost(obj: &ZendObject) -> bool {
    obj.is_lazy_ghost()
}

/// Check if a `ZendObject` is a lazy proxy
#[cfg(php84)]
#[php_function]
pub fn test_is_lazy_proxy(obj: &ZendObject) -> bool {
    obj.is_lazy_proxy()
}

/// Check if a lazy object has been initialized
#[cfg(php84)]
#[php_function]
pub fn test_is_lazy_initialized(obj: &ZendObject) -> bool {
    obj.is_lazy_initialized()
}

/// Test readonly class (PHP 8.2+)
/// All properties are implicitly readonly
#[cfg(php82)]
#[php_class]
#[php(readonly)]
pub struct TestReadonlyClass {
    name: String,
    value: i32,
}

#[cfg(php82)]
#[php_impl]
impl TestReadonlyClass {
    pub fn __construct(name: String, value: i32) -> Self {
        Self { name, value }
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_value(&self) -> i32 {
        self.value
    }
}

/// Test class for returning $this (Issue #502)
/// This demonstrates returning &mut Self from methods for fluent interfaces
#[php_class]
pub struct FluentBuilder {
    value: i32,
    name: String,
}

#[php_impl]
impl FluentBuilder {
    pub fn __construct() -> Self {
        Self {
            value: 0,
            name: String::new(),
        }
    }

    /// Set value and return $this for method chaining
    pub fn set_value(&mut self, value: i32) -> &mut Self {
        self.value = value;
        self
    }

    /// Set name and return $this for method chaining
    pub fn set_name(&mut self, name: String) -> &mut Self {
        self.name = name;
        self
    }

    /// Get the current value
    pub fn get_value(&self) -> i32 {
        self.value
    }

    /// Get the current name
    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    /// Test returning &Self (immutable reference to self)
    pub fn get_self(&self) -> &Self {
        self
    }
}

/// Test class for reserved keyword method names
#[php_class]
pub struct TestReservedKeywordMethods {
    value: String,
}

#[php_impl]
impl TestReservedKeywordMethods {
    pub fn __construct() -> Self {
        Self {
            value: String::from("initial"),
        }
    }

    #[allow(clippy::wrong_self_convention, clippy::new_ret_no_self)]
    pub fn r#new(&mut self, value: String) -> String {
        let result = format!("new called with: {value}");
        self.value = value;
        result
    }

    pub fn r#default(&self) -> String {
        String::from("default value")
    }

    pub fn r#class(&self) -> String {
        String::from("TestReservedKeywordMethods")
    }

    pub fn r#match(&self, pattern: String) -> bool {
        self.value.contains(&pattern)
    }

    pub fn r#return(&self) -> String {
        self.value.clone()
    }

    pub fn r#static(&self) -> String {
        String::from("not actually static")
    }
}

/// Test class with final methods
#[php_class]
pub struct TestFinalMethods;

#[php_impl]
impl TestFinalMethods {
    pub fn __construct() -> Self {
        Self
    }

    /// A final method that cannot be overridden
    #[php(final)]
    pub fn final_method(&self) -> &'static str {
        "final method result"
    }

    /// A final static method
    #[php(final)]
    pub fn final_static_method() -> &'static str {
        "final static method result"
    }

    /// A normal method that can be overridden
    pub fn normal_method(&self) -> &'static str {
        "normal method result"
    }
}

/// Test abstract class with abstract methods
#[php_class]
#[php(flags = ext_php_rs::flags::ClassFlags::Abstract)]
pub struct TestAbstractClass;

#[php_impl]
impl TestAbstractClass {
    /// Protected constructor for subclasses
    #[php(vis = "protected")]
    pub fn __construct() -> Self {
        Self
    }

    /// An abstract method that must be implemented by subclasses.
    /// The body is never called - it exists only for Rust syntax requirements.
    #[php(abstract)]
    pub fn abstract_method(&self) -> String {
        unimplemented!()
    }

    /// A concrete method in the abstract class
    pub fn concrete_method(&self) -> &'static str {
        "concrete method in abstract class"
    }
}

/// Test class for property visibility
#[php_class]
pub struct TestPropertyVisibility {
    /// Public property - accessible from anywhere
    #[php(prop)]
    pub public_num: i32,
    /// Private property - only accessible from within the class
    #[php(prop, flags = ext_php_rs::flags::PropertyFlags::Private)]
    pub private_str: String,
    /// Protected property - accessible from class and subclasses
    #[php(prop, flags = ext_php_rs::flags::PropertyFlags::Protected)]
    pub protected_str: String,
}

#[php_impl]
impl TestPropertyVisibility {
    pub fn __construct(public_num: i32, private_str: String, protected_str: String) -> Self {
        Self {
            public_num,
            private_str,
            protected_str,
        }
    }

    /// Method to access private property from within the class
    pub fn get_private(&self) -> String {
        self.private_str.clone()
    }

    /// Method to access protected property from within the class
    pub fn get_protected(&self) -> String {
        self.protected_str.clone()
    }

    /// Method to set private property from within the class
    pub fn set_private(&mut self, value: String) {
        self.private_str = value;
    }

    /// Method to set protected property from within the class
    pub fn set_protected(&mut self, value: String) {
        self.protected_str = value;
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

// Test for simple type syntax in extends (Issue #173)
//
// When both parent and child are Rust-defined classes, inherited methods don't work
// automatically because each Rust type has its own object handlers. The workaround
// is to use a Rust trait for shared behavior.

/// Trait for shared behavior between base and child classes
trait BaseClassBehavior {
    fn get_base_info(&self) -> &'static str {
        "I am the base class"
    }
}

#[php_class]
#[derive(Default)]
pub struct TestBaseClass;

impl BaseClassBehavior for TestBaseClass {}

#[php_impl]
impl TestBaseClass {
    pub fn __construct() -> Self {
        Self
    }

    /// Method exposed to PHP - delegates to trait
    pub fn get_base_info(&self) -> &'static str {
        BaseClassBehavior::get_base_info(self)
    }
}

// Child class using the new simple type syntax for extends
#[php_class]
#[php(extends(TestBaseClass))]
#[derive(Default)]
pub struct TestChildClass;

impl BaseClassBehavior for TestChildClass {}

#[php_impl]
impl TestChildClass {
    pub fn __construct() -> Self {
        Self
    }

    /// Re-export the inherited method - this is required because PHP inheritance
    /// doesn't automatically work for methods when both classes are Rust-defined
    pub fn get_base_info(&self) -> &'static str {
        BaseClassBehavior::get_base_info(self)
    }

    pub fn get_child_info(&self) -> &'static str {
        "I am the child class"
    }
}

#[php_class]
#[derive(Clone)]
pub struct TestCloneableClass {
    #[php(prop)]
    pub value: i32,
    #[php(prop)]
    pub name: String,
}

#[php_impl]
impl TestCloneableClass {
    pub fn __construct(value: i32, name: String) -> Self {
        Self { value, name }
    }

    pub fn accept_cloneable(obj: &TestCloneableClass) -> String {
        format!("accepted: {} {}", obj.value, obj.name)
    }
}

#[php_class]
pub struct TestUncloneableClass {
    #[php(prop)]
    pub data: String,
}

#[php_impl]
impl TestUncloneableClass {
    pub fn __construct(data: String) -> Self {
        Self { data }
    }
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    let builder = builder
        .class::<TestClass>()
        .class::<TestClassArrayAccess>()
        .class::<TestClassExtends>()
        .class::<TestClassExtendsWithProp>()
        .class::<TestClassExtendsImpl>()
        .class::<TestClassMethodVisibility>()
        .class::<TestClassProtectedConstruct>()
        .class::<TestStaticProps>()
        .class::<FluentBuilder>()
        .class::<TestPropertyVisibility>()
        .class::<TestReservedKeywordMethods>()
        .class::<TestLazyClass>()
        .class::<TestFinalMethods>()
        .class::<TestAbstractClass>()
        .class::<TestClassStaticStrGetter>()
        .class::<TestBaseClass>()
        .class::<TestChildClass>()
        .class::<TestCloneableClass>()
        .class::<TestUncloneableClass>()
        .class::<TestExceptionMessageLeak>()
        .function(wrap_function!(test_class))
        .function(wrap_function!(throw_exception))
        .function(wrap_function!(throw_class_object_exception_with_prop))
        .function(wrap_function!(throw_exception_with_message_prop));

    #[cfg(php84)]
    let builder = builder
        .function(wrap_function!(test_is_lazy))
        .function(wrap_function!(test_is_lazy_ghost))
        .function(wrap_function!(test_is_lazy_proxy))
        .function(wrap_function!(test_is_lazy_initialized));

    #[cfg(php82)]
    let builder = builder.class::<TestReadonlyClass>();

    builder
}

#[cfg(test)]
mod tests {
    /// Documents an outstanding bug in the `#[php(prop)]` field-property
    /// getter codegen: when an owned refcounted field (e.g. `String`) is read
    /// via the `Exception::getMessage` pattern (`zval_get_string` +
    /// `RETURN_STR`), the getter orphans one refcount on the `rv` stack zval
    /// per call. See the `#[php(prop)]` docs in `crates/macros/src/lib.rs`
    /// for the full mechanic and the recommended `#[php_method]` workaround.
    ///
    /// Ignored because a proper fix requires either an upstream PHP patch
    /// (`zval_ptr_dtor(&rv)` in `Exception::getMessage` when `retval == &rv`)
    /// or mirroring `#[php(prop)]` shadow fields to the parent's real
    /// property slot via `zend_update_property_stringl`. Run with
    /// `cargo test -- --ignored` to reproduce.
    #[test]
    #[ignore = "documents the #[php(prop)] String getter leak on the Exception::getMessage path; see crate-level docs"]
    fn prop_string_field_does_not_leak_on_repeated_get_message() {
        assert!(crate::integration::test::run_php(
            "class/prop_string_leak.php"
        ));
    }

    #[test]
    fn class_works() {
        assert!(crate::integration::test::run_php("class/class.php"));
    }
}
