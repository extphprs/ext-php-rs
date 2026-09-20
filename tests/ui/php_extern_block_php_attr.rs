use ext_php_rs::prelude::*;

#[php_extern]
#[php(name = "strings")]
extern "C" {
    fn strlen(s: &str) -> i64;
}

fn main() {}
