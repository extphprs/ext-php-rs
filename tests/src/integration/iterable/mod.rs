use ext_php_rs::{prelude::*, types::Iterable};

#[php_function]
pub fn iterable_count(mut iterable: Iterable) -> usize {
    let Some(iter) = iterable.iter() else {
        return 0;
    };

    iter.count()
}

#[php_function]
pub fn iterable_keys_to_string(mut iterable: Iterable) -> String {
    let Some(iter) = iterable.iter() else {
        return String::new();
    };

    iter.map(|(key, _)| match key.str() {
        Some(s) => s.to_string(),
        None => key.long().map_or_else(String::new, |l| l.to_string()),
    })
    .collect::<Vec<_>>()
    .join(",")
}

#[php_function]
pub fn iterable_values_to_string(mut iterable: Iterable) -> String {
    let Some(iter) = iterable.iter() else {
        return String::new();
    };

    iter.map(|(_, value)| match value.str() {
        Some(s) => s.to_string(),
        None => value.long().map_or_else(String::new, |l| l.to_string()),
    })
    .collect::<Vec<_>>()
    .join(",")
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .function(wrap_function!(iterable_count))
        .function(wrap_function!(iterable_keys_to_string))
        .function(wrap_function!(iterable_values_to_string))
}

#[cfg(test)]
mod tests {
    #[test]
    fn iterable_works() {
        assert!(crate::integration::test::run_php("iterable/iterable.php"));
    }
}
