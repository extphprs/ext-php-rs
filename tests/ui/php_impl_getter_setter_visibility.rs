use ext_php_rs::prelude::*;

#[php_class]
pub struct Foo;

#[php_impl]
impl Foo {
    #[php(getter)]
    fn get_value(&self) -> i64 {
        1
    }

    #[php(setter, vis = "private")]
    fn set_value(&mut self, _value: i64) {}
}

fn main() {}
