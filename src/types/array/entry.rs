//! Entry API for [`ZendHashTable`], similar to Rust's `std::collections::hash_map::Entry`.
//!
//! This module provides an ergonomic API for working with entries in a PHP
//! hashtable, allowing conditional insertion or modification based on whether
//! a key already exists.
//!
//! # Examples
//!
//! ```no_run
//! use ext_php_rs::types::ZendHashTable;
//!
//! let mut ht = ZendHashTable::new();
//!
//! // Insert a default value if the key doesn't exist
//! ht.entry("counter").or_insert(0i64);
//!
//! // Modify the value if it exists
//! ht.entry("counter").and_modify(|v| {
//!     if let Some(n) = v.long() {
//!         v.set_long(n + 1);
//!     }
//! });
//!
//! // Or use or_insert_with for lazy initialization
//! ht.entry("computed").or_insert_with(|| "computed value");
//! ```

use super::{ArrayKey, ZendHashTable};
use crate::error::Error;
use crate::{
    convert::IntoZval,
    error::Result,
    ffi::{
        zend_hash_index_find, zend_hash_index_update, zend_hash_str_find, zend_hash_str_update,
        zend_ulong,
    },
    types::Zval,
};
use std::mem::ManuallyDrop;

/// A view into a single entry in a [`ZendHashTable`], which may either be vacant or
/// occupied.
///
/// This enum is constructed from the [`entry`] method on [`ZendHashTable`].
///
/// [`entry`]: ZendHashTable::entry
pub enum Entry<'a, 'k> {
    /// An occupied entry.
    Occupied(OccupiedEntry<'a, 'k>),
    /// A vacant entry.
    Vacant(VacantEntry<'a, 'k>),
}

/// A view into an occupied entry in a [`ZendHashTable`].
///
/// It is part of the [`Entry`] enum.
pub struct OccupiedEntry<'a, 'k> {
    ht: &'a mut ZendHashTable,
    key: ArrayKey<'k>,
}

/// A view into a vacant entry in a [`ZendHashTable`].
///
/// It is part of the [`Entry`] enum.
pub struct VacantEntry<'a, 'k> {
    ht: &'a mut ZendHashTable,
    key: ArrayKey<'k>,
}

impl<'a, 'k> Entry<'a, 'k> {
    /// Ensures a value is in the entry by inserting the default if empty, and
    /// returns a mutable reference to the value in the entry.
    ///
    /// # Parameters
    ///
    /// * `default` - The default value to insert if the entry is vacant.
    ///
    /// # Returns
    ///
    /// A result containing a mutable reference to the value, or an error if
    /// the insertion failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the value conversion to [`Zval`] fails or if
    /// the key contains a null byte (for string keys).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    /// ht.entry("key").or_insert("default value");
    /// assert_eq!(ht.get("key").and_then(|v| v.str()), Some("default value"));
    /// ```
    pub fn or_insert<V: IntoZval>(self, default: V) -> Result<&'a mut Zval> {
        match self {
            Entry::Occupied(entry) => entry.into_mut().ok_or(Error::InvalidPointer),
            Entry::Vacant(entry) => entry.insert(default),
        }
    }

    /// Ensures a value is in the entry by inserting the result of the default
    /// function if empty, and returns a mutable reference to the value in the
    /// entry.
    ///
    /// # Parameters
    ///
    /// * `default` - A function that returns the default value to insert.
    ///
    /// # Returns
    ///
    /// A result containing a mutable reference to the value, or an error if
    /// the insertion failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the value conversion to [`Zval`] fails or if
    /// the key contains a null byte (for string keys).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    /// ht.entry("key").or_insert_with(|| "computed value");
    /// ```
    pub fn or_insert_with<V: IntoZval, F: FnOnce() -> V>(self, default: F) -> Result<&'a mut Zval> {
        match self {
            Entry::Occupied(entry) => entry.into_mut().ok_or(Error::InvalidPointer),
            Entry::Vacant(entry) => entry.insert(default()),
        }
    }

    /// Ensures a value is in the entry by inserting the result of the default
    /// function if empty. The function receives a reference to the key.
    ///
    /// # Parameters
    ///
    /// * `default` - A function that takes the key and returns the default value.
    ///
    /// # Returns
    ///
    /// A result containing a mutable reference to the value, or an error if
    /// the insertion failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the value conversion to [`Zval`] fails or if
    /// the key contains a null byte (for string keys).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    /// ht.entry("key").or_insert_with_key(|k| format!("value for {}", k));
    /// ```
    pub fn or_insert_with_key<V: IntoZval, F: FnOnce(&ArrayKey<'k>) -> V>(
        self,
        default: F,
    ) -> Result<&'a mut Zval> {
        match self {
            Entry::Occupied(entry) => entry.into_mut().ok_or(Error::InvalidPointer),
            Entry::Vacant(entry) => {
                let value = default(entry.key());
                entry.insert(value)
            }
        }
    }

    /// Returns a reference to this entry's key.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::{ZendHashTable, ArrayKey};
    ///
    /// let mut ht = ZendHashTable::new();
    /// assert_eq!(ht.entry("key").key(), &ArrayKey::Str("key"));
    /// ```
    #[must_use]
    pub fn key(&self) -> &ArrayKey<'k> {
        match self {
            Entry::Occupied(entry) => entry.key(),
            Entry::Vacant(entry) => entry.key(),
        }
    }

    /// Provides in-place mutable access to an occupied entry before any
    /// potential inserts into the map.
    ///
    /// # Parameters
    ///
    /// * `f` - A function that modifies the value in place.
    ///
    /// # Returns
    ///
    /// The entry, allowing for method chaining.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    /// ht.insert("counter", 0i64);
    ///
    /// ht.entry("counter")
    ///     .and_modify(|v| {
    ///         if let Some(n) = v.long() {
    ///             v.set_long(n + 1);
    ///         }
    ///     })
    ///     .or_insert(0i64);
    /// ```
    #[must_use]
    pub fn and_modify<F: FnOnce(&mut Zval)>(self, f: F) -> Self {
        match self {
            Entry::Occupied(mut entry) => {
                if let Some(value) = entry.get_mut() {
                    f(value);
                }
                Entry::Occupied(entry)
            }
            Entry::Vacant(entry) => Entry::Vacant(entry),
        }
    }
}

impl<'a, 'k> Entry<'a, 'k>
where
    'k: 'a,
{
    /// Ensures a value is in the entry by inserting the default value if empty,
    /// and returns a mutable reference to the value in the entry.
    ///
    /// This is a convenience method that uses `Default::default()` as the
    /// default value.
    ///
    /// # Returns
    ///
    /// A result containing a mutable reference to the value, or an error if
    /// the insertion failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the key contains a null byte (for string keys).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    /// // Inserts a null Zval if the key doesn't exist
    /// ht.entry("key").or_default();
    /// ```
    pub fn or_default(self) -> Result<&'a mut Zval> {
        match self {
            Entry::Occupied(entry) => entry.into_mut().ok_or(Error::InvalidPointer),
            Entry::Vacant(entry) => entry.insert(Zval::new()),
        }
    }
}

impl<'a, 'k> OccupiedEntry<'a, 'k> {
    /// Creates a new occupied entry.
    pub(super) fn new(ht: &'a mut ZendHashTable, key: ArrayKey<'k>) -> Self {
        Self { ht, key }
    }

    /// Gets a reference to the key in the entry.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::{ZendHashTable, ArrayKey};
    ///
    /// let mut ht = ZendHashTable::new();
    /// ht.insert("key", "value");
    ///
    /// if let ext_php_rs::types::array::Entry::Occupied(entry) = ht.entry("key") {
    ///     assert_eq!(entry.key(), &ArrayKey::Str("key"));
    /// }
    /// ```
    #[must_use]
    pub fn key(&self) -> &ArrayKey<'k> {
        &self.key
    }

    /// Gets a reference to the value in the entry.
    ///
    /// # Returns
    ///
    /// A result containing a reference to the value, or an error if
    /// the key conversion failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the key contains a null byte (for string keys).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::{ZendHashTable, Zval};
    ///
    /// let mut ht = ZendHashTable::new();
    /// ht.insert("key", "value");
    ///
    /// if let ext_php_rs::types::array::Entry::Occupied(entry) = ht.entry("key") {
    ///     assert_eq!(entry.get().and_then(Zval::str), Some("value"));
    /// }
    /// ```
    #[must_use]
    pub fn get(&self) -> Option<&Zval> {
        match &self.key {
            ArrayKey::Long(index) => unsafe {
                #[allow(clippy::cast_sign_loss)]
                zend_hash_index_find(self.ht, *index as zend_ulong).as_ref()
            },
            ArrayKey::String(key) => unsafe {
                zend_hash_str_find(self.ht, key.as_ptr().cast(), key.len()).as_ref()
            },
            ArrayKey::Str(key) => unsafe {
                zend_hash_str_find(self.ht, key.as_ptr().cast(), key.len()).as_ref()
            },
        }
    }

    /// Gets a mutable reference to the value in the entry.
    ///
    /// # Returns
    ///
    /// A result containing a mutable reference to the value, or an error if
    /// the key conversion failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the key contains a null byte (for string keys).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    /// ht.insert("counter", 0i64);
    ///
    /// if let ext_php_rs::types::array::Entry::Occupied(mut entry) = ht.entry("counter") {
    ///     if let Some(v) = entry.get_mut() {
    ///         if let Some(n) = v.long() {
    ///             v.set_long(n + 1);
    ///         }
    ///     }
    /// }
    /// ```
    pub fn get_mut(&mut self) -> Option<&mut Zval> {
        get_value_mut(&self.key, self.ht)
    }

    /// Converts the entry into a mutable reference to the value.
    ///
    /// If you need multiple references to the `OccupiedEntry`, see [`get_mut`].
    ///
    /// [`get_mut`]: OccupiedEntry::get_mut
    ///
    /// # Returns
    ///
    /// A result containing a mutable reference to the value with the entry's
    /// lifetime, or an error if the key conversion failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the key contains a null byte (for string keys).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    /// ht.insert("key", "value");
    ///
    /// if let ext_php_rs::types::array::Entry::Occupied(entry) = ht.entry("key") {
    ///     let value = entry.into_mut().unwrap();
    ///     // value has the lifetime of the hashtable borrow
    /// }
    /// ```
    #[must_use]
    pub fn into_mut(self) -> Option<&'a mut Zval> {
        get_value_mut(&self.key, self.ht)
    }

    /// Sets the value of the entry, returning the old value.
    ///
    /// # Parameters
    ///
    /// * `value` - The new value to set.
    ///
    /// # Returns
    ///
    /// A result containing the old value (shallow cloned), or an error if the
    /// insertion failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the value conversion to [`Zval`] fails or if
    /// the key contains a null byte (for string keys).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::{ZendHashTable, Zval};
    ///
    /// let mut ht = ZendHashTable::new();
    /// ht.insert("key", "old");
    ///
    /// if let ext_php_rs::types::array::Entry::Occupied(mut entry) = ht.entry("key") {
    ///     let old = entry.insert("new").unwrap();
    ///     assert_eq!(old.as_ref().and_then(Zval::str), Some("old"));
    /// }
    /// ```
    pub fn insert<V: IntoZval>(&mut self, value: V) -> Result<Option<Zval>> {
        let old = self.get().map(Zval::shallow_clone);
        insert_value(&self.key, self.ht, value)?;
        Ok(old)
    }

    /// Takes ownership of the key and value from the entry, removing it from
    /// the hashtable.
    ///
    /// # Returns
    ///
    /// A result containing a tuple of the key and the removed value (shallow
    /// cloned), or an error if the removal failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the key contains a null byte (for string keys).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    /// ht.insert("key", "value");
    ///
    /// if let ext_php_rs::types::array::Entry::Occupied(entry) = ht.entry("key") {
    ///     let (key, value) = entry.remove_entry();
    ///     assert!(ht.get("key").is_none());
    /// }
    /// ```
    pub fn remove_entry(self) -> (ArrayKey<'k>, Option<Zval>) {
        let value = self.get().map(Zval::shallow_clone);
        self.ht.remove(self.key.clone());
        (self.key, value)
    }

    /// Removes the value from the entry, returning it.
    ///
    /// # Returns
    ///
    /// A result containing the removed value (shallow cloned), or an error if
    /// the removal failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the key contains a null byte (for string keys).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    /// ht.insert("key", "value");
    ///
    /// if let ext_php_rs::types::array::Entry::Occupied(entry) = ht.entry("key") {
    ///     let value = entry.remove().unwrap();
    ///     assert_eq!(value.str(), Some("value"));
    /// }
    /// ```
    pub fn remove(self) -> Option<Zval> {
        let value = self.get().map(Zval::shallow_clone);
        self.ht.remove(self.key);
        value
    }
}

impl<'a, 'k> VacantEntry<'a, 'k> {
    /// Creates a new vacant entry.
    pub(super) fn new(ht: &'a mut ZendHashTable, key: ArrayKey<'k>) -> Self {
        Self { ht, key }
    }

    /// Gets a reference to the key that would be used when inserting a value
    /// through the `VacantEntry`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::{ZendHashTable, ArrayKey};
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// if let ext_php_rs::types::array::Entry::Vacant(entry) = ht.entry("key") {
    ///     assert_eq!(entry.key(), &ArrayKey::Str("key"));
    /// }
    /// ```
    #[must_use]
    pub fn key(&self) -> &ArrayKey<'k> {
        &self.key
    }

    /// Take ownership of the key.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::{ZendHashTable, ArrayKey};
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// if let ext_php_rs::types::array::Entry::Vacant(entry) = ht.entry("key") {
    ///     let key = entry.into_key();
    ///     assert_eq!(key, ArrayKey::Str("key"));
    /// }
    /// ```
    #[must_use]
    pub fn into_key(self) -> ArrayKey<'k> {
        self.key
    }

    /// Sets the value of the entry with the `VacantEntry`'s key, and returns
    /// a mutable reference to it.
    ///
    /// # Parameters
    ///
    /// * `value` - The value to insert.
    ///
    /// # Returns
    ///
    /// A result containing a mutable reference to the inserted value, or an
    /// error if the insertion failed.
    ///
    /// # Errors
    ///
    /// Returns an error if the value conversion to [`Zval`] fails or if
    /// the key contains a null byte (for string keys).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// if let ext_php_rs::types::array::Entry::Vacant(entry) = ht.entry("key") {
    ///     entry.insert("value");
    /// }
    /// assert_eq!(ht.get("key").and_then(|v| v.str()), Some("value"));
    /// ```
    pub fn insert<V: IntoZval>(self, value: V) -> Result<&'a mut Zval> {
        insert_value(&self.key, self.ht, value)
    }
}

/// Helper function to get a mutable value from the hashtable by key.
#[inline]
fn get_value_mut<'a>(key: &ArrayKey<'_>, ht: &'a mut ZendHashTable) -> Option<&'a mut Zval> {
    match key {
        ArrayKey::Long(index) => unsafe {
            #[allow(clippy::cast_sign_loss)]
            zend_hash_index_find(ht, *index as zend_ulong).as_mut()
        },
        ArrayKey::String(key) => unsafe {
            zend_hash_str_find(ht, key.as_ptr().cast(), key.len()).as_mut()
        },
        ArrayKey::Str(key) => unsafe {
            zend_hash_str_find(ht, key.as_ptr().cast(), key.len()).as_mut()
        },
    }
}

/// Helper function to insert a value into the hashtable by key.
fn insert_value<'a, V: IntoZval>(
    key: &ArrayKey<'_>,
    ht: &'a mut ZendHashTable,
    value: V,
) -> Result<&'a mut Zval> {
    // Wrap in ManuallyDrop to prevent drop from freeing the underlying data -
    // ownership is transferred to the hash table via shallow copy
    let mut val = ManuallyDrop::new(value.into_zval(false)?);
    match key {
        ArrayKey::Long(index) => unsafe {
            #[allow(clippy::cast_sign_loss)]
            zend_hash_index_update(ht, *index as zend_ulong, &raw mut *val)
                .as_mut()
                .ok_or(Error::InvalidPointer)
        },
        ArrayKey::String(key) => unsafe {
            zend_hash_str_update(ht, key.as_ptr().cast(), key.len(), &raw mut *val)
                .as_mut()
                .ok_or(Error::InvalidPointer)
        },
        ArrayKey::Str(key) => unsafe {
            zend_hash_str_update(ht, key.as_ptr().cast(), key.len(), &raw mut *val)
                .as_mut()
                .ok_or(Error::InvalidPointer)
        },
    }
}

#[cfg(test)]
#[cfg(feature = "embed")]
mod tests {
    use super::*;
    use crate::embed::Embed;

    #[test]
    fn test_entry_or_insert() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();

            // Insert into vacant entry
            let result = ht.entry("key").or_insert("value");
            assert!(result.is_ok());
            assert_eq!(ht.get("key").and_then(|v| v.str()), Some("value"));

            // Entry already exists, should return existing value
            let result = ht.entry("key").or_insert("other");
            assert!(result.is_ok());
            assert_eq!(ht.get("key").and_then(|v| v.str()), Some("value"));
        });
    }

    #[test]
    fn test_entry_or_insert_with() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();

            let result = ht.entry("key").or_insert_with(|| "computed");
            assert!(result.is_ok());
            assert_eq!(ht.get("key").and_then(|v| v.str()), Some("computed"));
        });
    }

    #[test]
    fn test_entry_and_modify() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();
            let _ = ht.insert("counter", 5i64);

            let result = ht
                .entry("counter")
                .and_modify(|v| {
                    if let Some(n) = v.long() {
                        v.set_long(n + 1);
                    }
                })
                .or_insert(0i64);

            assert!(result.is_ok());
            assert_eq!(ht.get("counter").and_then(Zval::long), Some(6));
        });
    }

    #[test]
    fn test_entry_and_modify_vacant() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();

            // and_modify on vacant entry should be a no-op
            let result = ht
                .entry("key")
                .and_modify(|v| {
                    v.set_long(100);
                })
                .or_insert(42i64);

            assert!(result.is_ok());
            assert_eq!(ht.get("key").and_then(Zval::long), Some(42));
        });
    }

    #[test]
    fn test_occupied_entry_insert() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();
            let _ = ht.insert("key", "old");

            if let Entry::Occupied(mut entry) = ht.entry("key") {
                let old = entry.insert("new").expect("insert should succeed");
                assert_eq!(old.as_ref().and_then(Zval::str), Some("old"));
            }
            assert_eq!(ht.get("key").and_then(|v| v.str()), Some("new"));
        });
    }

    #[test]
    fn test_occupied_entry_remove() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();
            let _ = ht.insert("key", "value");

            if let Entry::Occupied(entry) = ht.entry("key") {
                let value = entry.remove().expect("remove should succeed");
                assert_eq!(value.str(), Some("value"));
            }
            assert!(ht.get("key").is_none());
        });
    }

    #[test]
    fn test_entry_with_numeric_key() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();

            let result = ht.entry(42i64).or_insert("value");
            assert!(result.is_ok());
            assert_eq!(ht.get_index(42).and_then(|v| v.str()), Some("value"));
        });
    }

    #[test]
    fn test_vacant_entry_into_key() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();

            if let Entry::Vacant(entry) = ht.entry("my_key") {
                let key = entry.into_key();
                assert_eq!(key, ArrayKey::Str("my_key"));
            }
        });
    }

    #[test]
    fn test_entry_with_binary_key() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();
            let key = "\0MyClass\0myProp";
            let result = ht.entry(key).or_insert("value");
            assert!(result.is_ok());
        });
    }
}
