use ext_php_rs::prelude::*;

#[php_class]
pub struct Counter;

#[php_impl]
impl Counter {
    #[php(optional = self_)]
    pub fn get_count(
        self_: &mut ext_php_rs::types::ZendClassObject<Counter>,
        step: Option<i64>,
    ) -> i64 {
        let _ = self_;
        step.unwrap_or(1)
    }
}

fn main() {}
