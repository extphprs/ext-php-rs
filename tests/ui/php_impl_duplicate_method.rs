use ext_php_rs::prelude::*;

#[php_class]
pub struct Counter;

#[php_impl]
impl Counter {
    pub fn get_count(&self) -> i64 {
        0
    }

    #[php(name = "GETCOUNT")]
    pub fn count_again(&self) -> i64 {
        0
    }
}

fn main() {}
