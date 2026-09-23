use ext_php_rs::prelude::*;

#[php_function]
#[php(optional = desc)]
pub fn greet(name: String, description: Option<String>) -> String {
    format!("{name}{}", description.unwrap_or_default())
}

fn main() {}
