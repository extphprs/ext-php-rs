use ext_php_rs::prelude::*;

#[php_interface]
pub trait HasName {
    #[php(setter)]
    fn set_name(&mut self, name: String);
}

fn main() {}
