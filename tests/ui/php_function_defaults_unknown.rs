use ext_php_rs::prelude::*;

#[php_function]
#[php(defaults(tims = 1, times = 2, count = 3))]
pub fn repeat(text: String, times: i64) -> String {
    text.repeat(usize::try_from(times).unwrap_or(0))
}

fn main() {}
