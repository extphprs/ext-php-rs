use ext_php_rs::prelude::*;

#[php_interface]
pub trait Named {
    fn name(&self) -> String;
}

#[php_class]
pub struct Foo;

#[php_impl_interface]
impl Named for Foo {
    #[php(name = "getName")]
    fn name(&self) -> String {
        String::new()
    }
}

fn main() {}
