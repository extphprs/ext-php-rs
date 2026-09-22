use ext_php_rs::prelude::*;

#[php_interface]
#[php(change_method_case = "snake_case")]
pub trait Greeting {
    fn say_hello(&self) -> String;
}

#[php_class]
pub struct Greeter;

#[php_impl_interface(change_method_case = "snake_case")]
impl Greeting for Greeter {
    fn say_hello(&self) -> String {
        String::new()
    }
}

fn main() {}
