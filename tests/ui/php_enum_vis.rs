use ext_php_rs::prelude::*;

#[php_enum]
#[php(vis = "protected")]
pub enum Status {
    Active,
}

fn main() {}
