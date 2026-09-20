use ext_php_rs::prelude::*;

#[php_interface]
pub trait Named {
    fn name(&self) -> String;
}

#[php_class]
pub struct Foo;

#[php_impl_interface]
#[php(change_method_case = "snake_case")]
impl Named for Foo {
    fn name(&self) -> String {
        String::new()
    }
}

fn main() {}
