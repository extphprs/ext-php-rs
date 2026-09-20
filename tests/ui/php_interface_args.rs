use ext_php_rs::prelude::*;

#[php_interface(name = "Named")]
pub trait Named {
    fn name(&self) -> String;
}

fn main() {}
