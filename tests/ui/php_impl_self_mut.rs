use ext_php_rs::prelude::*;

#[php_class]
pub struct Counter;

#[php_impl]
impl Counter {
    pub fn get_count(self_: &mut ext_php_rs::types::ZendClassObject<Counter>) -> i64 {
        let _ = self_;
        0
    }
}

fn main() {}
