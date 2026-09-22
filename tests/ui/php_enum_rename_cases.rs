use ext_php_rs::prelude::*;

#[php_enum]
#[php(rename_cases = "snake_case")]
pub enum Suit {
    Hearts,
}

fn main() {}
