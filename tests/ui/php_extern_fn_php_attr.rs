use ext_php_rs::prelude::*;

#[php_extern]
extern "C" {
    #[php(name = "strlen")]
    fn str_len(s: &str) -> i64;
}

fn main() {}
