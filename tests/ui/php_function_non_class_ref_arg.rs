use ext_php_rs::prelude::*;

pub struct NotAClass {}

#[php_function]
pub fn read(_value: &NotAClass) {}

fn main() {}
