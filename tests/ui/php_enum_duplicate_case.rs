use ext_php_rs::prelude::*;

#[php_enum]
pub enum Suit {
    Hearts,
    #[php(name = "Hearts")]
    Spades,
}

fn main() {}
