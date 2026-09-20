use ext_php_rs::prelude::*;

#[derive(ZvalConvert)]
pub struct Point {
    #[php(name = "X")]
    x: i64,
}

fn main() {}
