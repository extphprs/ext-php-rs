use ext_php_rs::prelude::*;

#[php_class]
pub struct Person {
    age: i64,
}

#[php_impl]
impl Person {
    #[php(setter, defaults(age = 0))]
    pub fn set_age(&mut self, age: i64) {
        self.age = age;
    }
}

fn main() {}
