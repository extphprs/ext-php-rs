use ext_php_rs::prelude::*;

#[php_class]
pub struct Foo;

#[php_impl]
impl Foo {
    #[php(abstract, vis = "private")]
    fn hidden(&self) {}
}

fn main() {}
