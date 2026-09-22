use ext_php_rs::prelude::*;

#[php_class]
pub struct Counter;

#[php_impl]
impl Counter {
    #[php(defaults(self_ = 0))]
    pub fn get_count(self_: &mut ext_php_rs::types::ZendClassObject<Counter>, step: i64) -> i64 {
        let _ = self_;
        step
    }
}

fn main() {}
