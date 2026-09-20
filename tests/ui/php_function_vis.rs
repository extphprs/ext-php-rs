use ext_php_rs::prelude::*;

#[php_function]
#[php(vis = "private")]
pub fn helper() -> i64 {
    42
}

fn main() {}
