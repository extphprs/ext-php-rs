use ext_php_rs::prelude::*;

#[php_enum]
pub enum Flag {
    #[php(value = true)]
    On,
}

fn main() {}
