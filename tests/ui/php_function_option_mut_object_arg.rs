use ext_php_rs::prelude::*;

#[php_class]
pub struct Counter {}

#[php_function]
pub fn bump(_counter: Option<&mut Counter>) {}

fn main() {}
