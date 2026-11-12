use ext_php_rs::prelude::*;

#[php_enum]
pub enum Suit {
    Hearts,
}

#[php_function]
pub fn pick(_suit: &Suit) {}

fn main() {}
