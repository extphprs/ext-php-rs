use ext_php_rs::prelude::*;

#[php_interface]
pub trait HasName {
    #[php(getter)]
    fn name(&self) -> String;
}

fn main() {}
