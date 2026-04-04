//! Math plugin

#[php_function]
fn add(a: i64, b: i64) -> i64 {
    a + b
}

#[php_function]
fn subtract(a: i64, b: i64) -> i64 {
    a - b
}

#[php_function]
fn multiply(a: i64, b: i64) -> i64 {
    a * b
}

#[php_function]
fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => {
            let mut a = 0u64;
            let mut b = 1u64;
            for _ in 2..=n {
                let c = a + b;
                a = b;
                b = c;
            }
            b
        }
    }
}
