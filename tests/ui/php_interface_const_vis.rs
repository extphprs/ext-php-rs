use ext_php_rs::prelude::*;

#[php_interface]
pub trait HasLimit {
    #[php(vis = "protected")]
    const LIMIT: i64 = 10;
}

fn main() {}
