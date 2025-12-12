//! Collection type conversions for `ZendHashTable`.
//!
//! This module provides conversions between Rust collection types and PHP arrays
//! (represented as `ZendHashTable`). Each collection type has its own module for
//! better organization and maintainability.
//!
//! ## Supported Collections
//!
//! - `BTreeMap<K, V>` ↔ `ZendHashTable` (via `btree_map` module)
//! - `BTreeSet<V>` ↔ `ZendHashTable` (via `btree_set` module)
//! - `HashMap<K, V>` ↔ `ZendHashTable` (via `hash_map` module)
//! - `HashSet<V>` ↔ `ZendHashTable` (via `hash_set` module)
//! - `Vec<T>` and `Vec<(K, V)>` ↔ `ZendHashTable` (via `vec` module)

mod btree_map;
mod btree_set;
mod hash_map;
mod hash_set;
mod vec;
