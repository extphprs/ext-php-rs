#[macro_use]
extern crate ext_php_rs_derive;

/// Doc comments for greet.
#[php_function]
#[php(defaults(times = 1))]
pub fn greet(name: String, age: Option<i64>, times: i64) -> String {
    String::new()
}

#[php_function]
pub fn sum(first: i64, rest: &[i64]) -> i64 {
    first
}

#[php_function]
pub fn bump(target: ext_php_rs::types::PhpRef<'_>) {}
