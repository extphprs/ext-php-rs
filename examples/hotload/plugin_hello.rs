//! Hello plugin

#[php_function]
fn hello(name: String) -> String {
    format!("Hello, {}!", name)
}

#[php_function]
fn rust_nanos() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as i64
}

#[php_function]
fn sum_of_squares(n: i64) -> i64 {
    (1..=n).map(|x| x * x).sum()
}
