use ext_php_rs::prelude::*;

#[php_function]
#[php(optional = age)]
pub fn describe(name: String, age: Option<i64>, city: String) -> String {
    format!("{name} {age:?} {city}")
}

fn main() {}
