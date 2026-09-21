use ext_php_rs::prelude::*;

#[php_class]
pub struct Foo;

#[php_impl]
impl Foo {
    #[php(getter, setter)]
    fn value(&self) -> i64 {
        1
    }
}

fn main() {}
