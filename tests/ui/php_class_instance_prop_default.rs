use ext_php_rs::prelude::*;

#[php_class]
pub struct Counter {
    #[php(prop, default = 0)]
    count: i64,
}

fn main() {}
