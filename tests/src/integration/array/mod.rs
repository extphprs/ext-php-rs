use std::collections::{BTreeMap, HashMap};

use ext_php_rs::{
    convert::IntoZval,
    ffi::HashTable,
    php_function,
    prelude::ModuleBuilder,
    types::{ArrayKey, ZendHashTable, Zval},
    wrap_function,
};

#[php_function]
pub fn test_array(a: Vec<String>) -> Vec<String> {
    a
}

#[php_function]
pub fn test_array_assoc(a: HashMap<String, String>) -> HashMap<String, String> {
    a
}

#[php_function]
pub fn test_array_assoc_array_keys(a: Vec<(ArrayKey, String)>) -> Vec<(ArrayKey, String)> {
    a
}

#[php_function]
pub fn test_btree_map(a: BTreeMap<ArrayKey, String>) -> BTreeMap<ArrayKey, String> {
    a
}

#[php_function]
pub fn test_array_keys() -> Zval {
    let mut ht = HashTable::new();
    ht.insert(-42, "foo").unwrap();
    ht.insert(0, "bar").unwrap();
    ht.insert(5, "baz").unwrap();
    ht.insert("10", "qux").unwrap();
    ht.insert("quux", "quuux").unwrap();

    ht.into_zval(false).unwrap()
}

/// Test that `Option<&ZendHashTable>` can accept literal arrays (issue #515)
#[php_function]
pub fn test_optional_array_ref(arr: Option<&ZendHashTable>) -> i64 {
    arr.map_or(-1, |ht| i64::try_from(ht.len()).unwrap_or(i64::MAX))
}

/// Test that `Option<&mut ZendHashTable>` works correctly (anti-regression for issue #515)
#[php_function]
pub fn test_optional_array_mut_ref(arr: Option<&mut ZendHashTable>) -> i64 {
    match arr {
        Some(ht) => {
            // Add an element to verify mutation works
            ht.insert("added_by_rust", "value").ok();
            i64::try_from(ht.len()).unwrap_or(i64::MAX)
        }
        None => -1,
    }
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .function(wrap_function!(test_array))
        .function(wrap_function!(test_array_assoc))
        .function(wrap_function!(test_array_assoc_array_keys))
        .function(wrap_function!(test_btree_map))
        .function(wrap_function!(test_array_keys))
        .function(wrap_function!(test_optional_array_ref))
        .function(wrap_function!(test_optional_array_mut_ref))
}

#[cfg(test)]
mod tests {
    #[test]
    fn array_works() {
        assert!(crate::integration::test::run_php("array/array.php"));
    }
}
