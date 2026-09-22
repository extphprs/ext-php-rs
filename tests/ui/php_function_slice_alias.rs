use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;

type Rest<'a> = &'a [&'a Zval];

#[php_function]
pub fn count_all(rest: Rest<'_>) -> usize {
    rest.len()
}

fn main() {}
