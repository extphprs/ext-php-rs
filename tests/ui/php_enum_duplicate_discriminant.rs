use ext_php_rs::prelude::*;

#[php_enum]
pub enum Level {
    #[php(value = 1)]
    Low,
    #[php(value = 1)]
    High,
}

fn main() {}
