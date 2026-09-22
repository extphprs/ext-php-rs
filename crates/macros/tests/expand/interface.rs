#[macro_use]
extern crate ext_php_rs_derive;

/// Doc comments for MyInterface.
/// This is a basic interface example.
#[php_interface]
trait MyInterface {
    /// Doc comments for MY_CONST.
    const MY_CONST: i32 = 42;
    /// Doc comments for my_method.
    /// This method does something.
    fn my_method(&self, arg: i32) -> String;
}

#[php_interface]
#[php(change_method_case = "UPPER_CASE")]
#[php(change_constant_case = "snake_case")]
trait MyInterface2 {
    const MY_CONST: i32 = 42;
    #[php(change_case = "PascalCase")]
    const ANOTHER_CONST: &'static str = "Hello";
    fn my_method(&self, arg: i32) -> String;
    #[php(change_case = "PascalCase")]
    fn anotherMethod(&self) -> i32;
    fn static_method() -> i32;
}

#[php_class]
struct MyImpl {}

#[php_impl_interface]
impl MyInterface for MyImpl {
    fn my_method(&self, arg: i32) -> String {
        String::new()
    }
}
