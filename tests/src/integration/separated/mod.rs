use ext_php_rs::{
    php_function,
    prelude::ModuleBuilder,
    types::{PhpRef, Separated, Zval},
    wrap_function,
};

/// Accepts any value via Separated, returns its type name.
/// Should work with literals — no pass-by-reference required.
#[php_function]
pub fn test_separated_type(data: Separated) -> String {
    format!("{:?}", data.get_type())
}

/// Mutates an array via Separated (COW separation).
/// The caller's original array is NOT modified.
#[php_function]
pub fn test_separated_array_push(mut data: Separated) -> i64 {
    let Some(ht) = data.array_mut() else {
        return -1;
    };
    ht.push("added_by_rust").ok();
    i64::try_from(ht.len()).unwrap_or(i64::MAX)
}

/// Reads a value via `Separated` without mutation (`Deref` to `&Zval`).
#[php_function]
pub fn test_separated_array_len(data: Separated) -> i64 {
    data.array()
        .map_or(-1, |ht| i64::try_from(ht.len()).unwrap_or(i64::MAX))
}

/// Mutates the caller's variable in-place via `PhpRef`.
/// Requires `&$var` at the PHP call site.
#[php_function]
pub fn test_phpref_increment(mut val: PhpRef) {
    if let Some(n) = val.long() {
        val.set_long(n + 1);
    }
}

/// Pushes to the caller's array in-place via `PhpRef`.
#[php_function]
pub fn test_phpref_array_push(mut val: PhpRef) -> i64 {
    let Some(ht) = val.array_mut() else {
        return -1;
    };
    ht.push("pushed_by_rust").ok();
    i64::try_from(ht.len()).unwrap_or(i64::MAX)
}

/// Returns the long value read through `PhpRef` (`Deref` to `&Zval`).
#[php_function]
pub fn test_phpref_read(val: PhpRef) -> i64 {
    val.long().unwrap_or(-1)
}

/// Replaces the `Zval` content entirely through `PhpRef`.
#[php_function]
pub fn test_phpref_set_string(mut val: PhpRef) {
    val.set_string("replaced_by_rust", false).ok();
}

/// Uses Separated with a string value.
#[php_function]
pub fn test_separated_string(data: Separated) -> String {
    data.str().unwrap_or("not_a_string").to_string()
}

/// Proves that Separated with a literal long works.
#[php_function]
pub fn test_separated_long(data: Separated) -> i64 {
    data.long().unwrap_or(-1)
}

/// Takes a Separated and returns it as a new Zval (clone the content).
#[php_function]
pub fn test_separated_passthrough(data: Separated) -> Zval {
    data.shallow_clone()
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .function(wrap_function!(test_separated_type))
        .function(wrap_function!(test_separated_array_push))
        .function(wrap_function!(test_separated_array_len))
        .function(wrap_function!(test_phpref_increment))
        .function(wrap_function!(test_phpref_array_push))
        .function(wrap_function!(test_phpref_read))
        .function(wrap_function!(test_phpref_set_string))
        .function(wrap_function!(test_separated_string))
        .function(wrap_function!(test_separated_long))
        .function(wrap_function!(test_separated_passthrough))
}

#[cfg(test)]
mod tests {
    #[test]
    fn separated_works() {
        assert!(crate::integration::test::run_php("separated/separated.php"));
    }

    #[test]
    #[cfg(feature = "embed")]
    fn separated_works_embed() {
        crate::integration::test::run_php_embed("separated/separated.php");
    }
}
