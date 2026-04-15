use criterion::{Criterion, criterion_group, criterion_main};
use ext_php_rs::embed::Embed;
use ext_php_rs::types::{ZendHashTable, ZendStr, Zval};

fn bench_eval_simple(c: &mut Criterion) {
    Embed::run(|| {
        c.bench_function("eval_simple_expression", |b| {
            b.iter(|| {
                let result = Embed::eval("1 + 1;");
                assert!(result.is_ok());
            });
        });
    });
}

fn bench_eval_string_concat(c: &mut Criterion) {
    Embed::run(|| {
        c.bench_function("eval_string_concat", |b| {
            b.iter(|| {
                let result = Embed::eval("'hello' . ' ' . 'world';");
                assert!(result.is_ok());
            });
        });
    });
}

fn bench_eval_array_creation(c: &mut Criterion) {
    Embed::run(|| {
        c.bench_function("eval_array_creation", |b| {
            b.iter(|| {
                let result = Embed::eval("[1, 2, 3, 4, 5];");
                assert!(result.is_ok());
            });
        });
    });
}

fn bench_hashtable_insert_sequential(c: &mut Criterion) {
    Embed::run(|| {
        c.bench_function("hashtable_push_100", |b| {
            b.iter(|| {
                let mut ht = ZendHashTable::new();
                for i in 0..100 {
                    ht.push(i as i64).unwrap();
                }
            });
        });
    });
}

fn bench_hashtable_insert_string_keys(c: &mut Criterion) {
    Embed::run(|| {
        let keys: Vec<String> = (0..100).map(|i| format!("key_{i}")).collect();

        c.bench_function("hashtable_insert_string_keys_100", |b| {
            b.iter(|| {
                let mut ht = ZendHashTable::new();
                for (i, key) in keys.iter().enumerate() {
                    ht.insert(key.as_str(), i as i64).unwrap();
                }
            });
        });
    });
}

fn bench_hashtable_get(c: &mut Criterion) {
    Embed::run(|| {
        let mut ht = ZendHashTable::new();
        for i in 0..100 {
            ht.insert(format!("key_{i}").as_str(), i as i64).unwrap();
        }

        c.bench_function("hashtable_get_by_string_key", |b| {
            b.iter(|| {
                let _ = ht.get("key_50");
            });
        });
    });
}

fn bench_hashtable_get_index(c: &mut Criterion) {
    Embed::run(|| {
        let mut ht = ZendHashTable::new();
        for i in 0..100 {
            ht.push(i as i64).unwrap();
        }

        c.bench_function("hashtable_get_by_index", |b| {
            b.iter(|| {
                let _ = ht.get_index(50);
            });
        });
    });
}

fn bench_zend_string_creation(c: &mut Criterion) {
    Embed::run(|| {
        c.bench_function("zend_string_create_short", |b| {
            b.iter(|| {
                let _s = ZendStr::new("hello world", false);
            });
        });
    });
}

fn bench_zend_string_creation_long(c: &mut Criterion) {
    Embed::run(|| {
        let long_string = "a".repeat(1000);

        c.bench_function("zend_string_create_long", |b| {
            b.iter(|| {
                let _s = ZendStr::new(&long_string, false);
            });
        });
    });
}

fn bench_zval_type_conversions(c: &mut Criterion) {
    Embed::run(|| {
        c.bench_function("zval_set_and_read_long", |b| {
            b.iter(|| {
                let mut zv = Zval::new();
                zv.set_long(42);
                let _ = zv.long();
            });
        });
    });
}

fn bench_zval_string_roundtrip(c: &mut Criterion) {
    Embed::run(|| {
        c.bench_function("zval_string_roundtrip", |b| {
            b.iter(|| {
                let mut zv = Zval::new();
                let _ = zv.set_string("hello world", false);
                let _ = zv.str();
            });
        });
    });
}

fn bench_hashtable_iteration(c: &mut Criterion) {
    Embed::run(|| {
        let mut ht = ZendHashTable::new();
        for i in 0..100 {
            ht.insert(format!("key_{i}").as_str(), i as i64).unwrap();
        }

        c.bench_function("hashtable_iterate_100_entries", |b| {
            b.iter(|| {
                let mut count = 0;
                for (_key, _val) in ht.iter() {
                    count += 1;
                }
                assert_eq!(count, 100);
            });
        });
    });
}

criterion_group!(
    benches,
    bench_eval_simple,
    bench_eval_string_concat,
    bench_eval_array_creation,
    bench_hashtable_insert_sequential,
    bench_hashtable_insert_string_keys,
    bench_hashtable_get,
    bench_hashtable_get_index,
    bench_zend_string_creation,
    bench_zend_string_creation_long,
    bench_zval_type_conversions,
    bench_zval_string_roundtrip,
    bench_hashtable_iteration,
);

criterion_main!(benches);
