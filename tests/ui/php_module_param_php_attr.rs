use ext_php_rs::prelude::*;

#[php_module]
pub fn get_module(#[php(name = "m")] module: ModuleBuilder) -> ModuleBuilder {
    module
}

fn main() {}
