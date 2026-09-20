use ext_php_rs::prelude::*;

#[derive(ZvalConvert)]
pub enum Value {
    Int(#[php(name = "n")] i64),
}

fn main() {}
