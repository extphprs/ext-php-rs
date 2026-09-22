use ext_php_rs::prelude::*;

#[php_function]
#[php(name = "renamed", change_case = "UPPER_CASE")]
pub fn helper() -> i64 {
    42
}

fn main() {}
