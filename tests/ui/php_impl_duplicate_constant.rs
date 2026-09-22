use ext_php_rs::prelude::*;

#[php_class]
pub struct Limits;

#[php_impl]
impl Limits {
    pub const MAX_SIZE: i64 = 10;
    #[php(name = "MAX_SIZE")]
    pub const LARGEST: i64 = 20;
}

fn main() {}
