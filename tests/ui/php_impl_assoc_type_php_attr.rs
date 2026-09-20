use ext_php_rs::prelude::*;

#[php_class]
pub struct Foo;

#[php_impl]
impl Foo {
    #[php(name = "Item")]
    type Item = i64;
}

fn main() {}
