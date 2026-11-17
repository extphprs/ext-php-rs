#[macro_use]
extern crate ext_php_rs_derive;

use ext_php_rs::types::Zval;

#[php_class]
pub struct Emitter {
    listeners: Vec<Zval>,
    count: i64,
}

#[php_class]
pub struct Pair(Zval, String);
