use ext_php_rs::prelude::*;

#[php_class]
pub struct Person {
    age: i64,
}

#[php_impl]
impl Person {
    #[php(getter, optional = age)]
    pub fn get_age(&self) -> i64 {
        self.age
    }
}

fn main() {}
