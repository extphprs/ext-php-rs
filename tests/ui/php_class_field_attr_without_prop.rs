use ext_php_rs::prelude::*;

#[php_class]
pub struct Counter {
    #[php(name = "total")]
    count: i64,
}

fn main() {}
