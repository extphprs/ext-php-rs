use ext_php_rs::prelude::*;

#[php_function]
pub fn greet(#[php(name = "who")] name: String) -> String {
    name
}

fn main() {}
