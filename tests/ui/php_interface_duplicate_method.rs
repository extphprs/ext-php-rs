use ext_php_rs::prelude::*;

#[php_interface]
pub trait Shape {
    fn area(&self) -> f64;
    #[php(name = "Area")]
    fn surface(&self) -> f64;
}

fn main() {}
