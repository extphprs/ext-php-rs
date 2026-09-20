use ext_php_rs::prelude::*;

#[php_extern(name = "strlen")]
extern "C" {
    fn strlen(s: &str) -> i64;
}

fn main() {}
