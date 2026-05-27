use ext_php_rs::{
    prelude::*,
    types::{ArrayKey, ZendHashTable, Zval},
};

#[php_function]
pub fn iter_next(ht: &ZendHashTable) -> Vec<Zval> {
    ht.iter()
        .flat_map(|(k, v)| [key_to_zval(k), v.shallow_clone()])
        .collect()
}

#[php_function]
pub fn iter_back(ht: &ZendHashTable) -> Vec<Zval> {
    ht.iter()
        .rev()
        .flat_map(|(k, v)| [key_to_zval(k), v.shallow_clone()])
        .collect()
}

#[php_function]
pub fn iter_next_back(ht: &ZendHashTable, modulus: usize) -> Vec<Option<Zval>> {
    let mut result = Vec::with_capacity(ht.len());
    let mut iter = ht.iter();

    for i in 0..ht.len() + modulus {
        let entry = if i % modulus == 0 {
            iter.next_back()
        } else {
            iter.next()
        };

        if let Some((k, v)) = entry {
            result.push(Some(key_to_zval(k)));
            result.push(Some(v.shallow_clone()));
        } else {
            result.push(None);
        }
    }

    result
}

fn key_to_zval(key: ArrayKey) -> Zval {
    match key {
        ArrayKey::String(s) => {
            let mut zval = Zval::new();
            let _ = zval.set_string(s.as_str(), false);
            zval
        }
        ArrayKey::Str(s) => {
            let mut zval = Zval::new();
            let _ = zval.set_string(s, false);
            zval
        }
        ArrayKey::Long(l) => {
            let mut zval = Zval::new();
            zval.set_long(l);
            zval
        },
        ArrayKey::ZendString(_) => todo!()
    }
}
pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    builder
        .function(wrap_function!(iter_next))
        .function(wrap_function!(iter_back))
        .function(wrap_function!(iter_next_back))
}

#[cfg(test)]
mod tests {
    #[test]
    fn iterator_works() {
        assert!(crate::integration::test::run_php("iterator/iterator.php"));
    }
}
