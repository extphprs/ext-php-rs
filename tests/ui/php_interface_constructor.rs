use ext_php_rs::prelude::*;

#[php_interface]
pub trait Buildable {
    #[php(constructor)]
    fn new(id: i64) -> Self;
}

fn main() {}
