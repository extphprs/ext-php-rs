//! Represents an array in PHP. As all arrays in PHP are associative arrays,
//! they are represented by hash tables.

use std::{convert::TryFrom, ffi::CString, fmt::Debug, ptr};

use crate::{
    boxed::{ZBox, ZBoxable},
    convert::{FromZval, FromZvalMut, IntoZval},
    error::Result,
    ffi::zend_ulong,
    ffi::{
        _zend_new_array, GC_FLAGS_MASK, GC_FLAGS_SHIFT, HT_MIN_SIZE, zend_array_count,
        zend_array_destroy, zend_array_dup, zend_empty_array, zend_hash_clean, zend_hash_index_del,
        zend_hash_index_find, zend_hash_index_update, zend_hash_next_index_insert,
        zend_hash_str_del, zend_hash_str_find, zend_hash_str_update,
    },
    flags::{DataType, ZvalTypeFlags},
    types::Zval,
};

mod array_key;
mod conversions;
mod iterators;

pub use array_key::ArrayKey;
pub use iterators::{Iter, Values};

/// A PHP hashtable.
///
/// In PHP, arrays are represented as hashtables. This allows you to push values
/// onto the end of the array like a vector, while also allowing you to insert
/// at arbitrary string key indexes.
///
/// A PHP hashtable stores values as [`Zval`]s. This allows you to insert
/// different types into the same hashtable. Types must implement [`IntoZval`]
/// to be able to be inserted into the hashtable.
///
/// # Examples
///
/// ```no_run
/// use ext_php_rs::types::ZendHashTable;
///
/// let mut ht = ZendHashTable::new();
/// ht.push(1);
/// ht.push("Hello, world!");
/// ht.insert("Like", "Hashtable");
///
/// assert_eq!(ht.len(), 3);
/// assert_eq!(ht.get_index(0).and_then(|zv| zv.long()), Some(1));
/// ```
pub type ZendHashTable = crate::ffi::HashTable;

// Clippy complains about there being no `is_empty` function when implementing
// on the alias `ZendStr` :( <https://github.com/rust-lang/rust-clippy/issues/7702>
#[allow(clippy::len_without_is_empty)]
impl ZendHashTable {
    /// Creates a new, empty, PHP hashtable, returned inside a [`ZBox`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let ht = ZendHashTable::new();
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if memory for the hashtable could not be allocated.
    #[must_use]
    pub fn new() -> ZBox<Self> {
        Self::with_capacity(HT_MIN_SIZE)
    }

    /// Creates a new, empty, PHP hashtable with an initial size, returned
    /// inside a [`ZBox`].
    ///
    /// # Parameters
    ///
    /// * `size` - The size to initialize the array with.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let ht = ZendHashTable::with_capacity(10);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if memory for the hashtable could not be allocated.
    #[must_use]
    pub fn with_capacity(size: u32) -> ZBox<Self> {
        unsafe {
            // SAFETY: PHP allocator handles the creation of the array.
            #[allow(clippy::used_underscore_items)]
            let ptr = _zend_new_array(size);

            // SAFETY: `as_mut()` checks if the pointer is null, and panics if it is not.
            ZBox::from_raw(
                ptr.as_mut()
                    .expect("Failed to allocate memory for hashtable"),
            )
        }
    }

    /// Returns the current number of elements in the array.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// ht.push(1);
    /// ht.push("Hello, world");
    ///
    /// assert_eq!(ht.len(), 2);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        unsafe { zend_array_count(ptr::from_ref(self).cast_mut()) as usize }
    }

    /// Returns whether the hash table is empty.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// assert_eq!(ht.is_empty(), true);
    ///
    /// ht.push(1);
    /// ht.push("Hello, world");
    ///
    /// assert_eq!(ht.is_empty(), false);
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears the hash table, removing all values.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// ht.insert("test", "hello world");
    /// assert_eq!(ht.is_empty(), false);
    ///
    /// ht.clear();
    /// assert_eq!(ht.is_empty(), true);
    /// ```
    pub fn clear(&mut self) {
        unsafe { zend_hash_clean(self) }
    }

    /// Attempts to retrieve a value from the hash table with a string key.
    ///
    /// # Parameters
    ///
    /// * `key` - The key to search for in the hash table.
    ///
    /// # Returns
    ///
    /// * `Some(&Zval)` - A reference to the zval at the position in the hash
    ///   table.
    /// * `None` - No value at the given position was found.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// ht.insert("test", "hello world");
    /// assert_eq!(ht.get("test").and_then(|zv| zv.str()), Some("hello world"));
    /// ```
    #[must_use]
    pub fn get<'a, K>(&self, key: K) -> Option<&Zval>
    where
        K: Into<ArrayKey<'a>>,
    {
        match key.into() {
            ArrayKey::Long(index) => unsafe {
                #[allow(clippy::cast_sign_loss)]
                zend_hash_index_find(self, index as zend_ulong).as_ref()
            },
            ArrayKey::String(key) => unsafe {
                zend_hash_str_find(
                    self,
                    CString::new(key.as_str()).ok()?.as_ptr(),
                    key.len() as _,
                )
                .as_ref()
            },
            ArrayKey::Str(key) => unsafe {
                zend_hash_str_find(self, CString::new(key).ok()?.as_ptr(), key.len() as _).as_ref()
            },
        }
    }

    /// Attempts to retrieve a value from the hash table with a string key.
    ///
    /// # Parameters
    ///
    /// * `key` - The key to search for in the hash table.
    ///
    /// # Returns
    ///
    /// * `Some(&Zval)` - A reference to the zval at the position in the hash
    ///   table.
    /// * `None` - No value at the given position was found.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// ht.insert("test", "hello world");
    /// assert_eq!(ht.get("test").and_then(|zv| zv.str()), Some("hello world"));
    /// ```
    // TODO: Verify if this is safe to use, as it allows mutating the
    // hashtable while only having a reference to it. #461
    #[allow(clippy::mut_from_ref)]
    #[must_use]
    pub fn get_mut<'a, K>(&self, key: K) -> Option<&mut Zval>
    where
        K: Into<ArrayKey<'a>>,
    {
        match key.into() {
            ArrayKey::Long(index) => unsafe {
                #[allow(clippy::cast_sign_loss)]
                zend_hash_index_find(self, index as zend_ulong).as_mut()
            },
            ArrayKey::String(key) => unsafe {
                zend_hash_str_find(
                    self,
                    CString::new(key.as_str()).ok()?.as_ptr(),
                    key.len() as _,
                )
                .as_mut()
            },
            ArrayKey::Str(key) => unsafe {
                zend_hash_str_find(self, CString::new(key).ok()?.as_ptr(), key.len() as _).as_mut()
            },
        }
    }

    /// Attempts to retrieve a value from the hash table with an index.
    ///
    /// # Parameters
    ///
    /// * `key` - The key to search for in the hash table.
    ///
    /// # Returns
    ///
    /// * `Some(&Zval)` - A reference to the zval at the position in the hash
    ///   table.
    /// * `None` - No value at the given position was found.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// ht.push(100);
    /// assert_eq!(ht.get_index(0).and_then(|zv| zv.long()), Some(100));
    /// ```
    #[must_use]
    pub fn get_index(&self, key: i64) -> Option<&Zval> {
        #[allow(clippy::cast_sign_loss)]
        unsafe {
            zend_hash_index_find(self, key as zend_ulong).as_ref()
        }
    }

    /// Attempts to retrieve a value from the hash table with an index.
    ///
    /// # Parameters
    ///
    /// * `key` - The key to search for in the hash table.
    ///
    /// # Returns
    ///
    /// * `Some(&Zval)` - A reference to the zval at the position in the hash
    ///   table.
    /// * `None` - No value at the given position was found.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// ht.push(100);
    /// assert_eq!(ht.get_index(0).and_then(|zv| zv.long()), Some(100));
    /// ```
    // TODO: Verify if this is safe to use, as it allows mutating the
    // hashtable while only having a reference to it. #461
    #[allow(clippy::mut_from_ref)]
    #[must_use]
    pub fn get_index_mut(&self, key: i64) -> Option<&mut Zval> {
        unsafe {
            #[allow(clippy::cast_sign_loss)]
            zend_hash_index_find(self, key as zend_ulong).as_mut()
        }
    }

    /// Attempts to remove a value from the hash table with a string key.
    ///
    /// # Parameters
    ///
    /// * `key` - The key to remove from the hash table.
    ///
    /// # Returns
    ///
    /// * `Some(())` - Key was successfully removed.
    /// * `None` - No key was removed, did not exist.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// ht.insert("test", "hello world");
    /// assert_eq!(ht.len(), 1);
    ///
    /// ht.remove("test");
    /// assert_eq!(ht.len(), 0);
    /// ```
    pub fn remove<'a, K>(&mut self, key: K) -> Option<()>
    where
        K: Into<ArrayKey<'a>>,
    {
        let result = match key.into() {
            ArrayKey::Long(index) => unsafe {
                #[allow(clippy::cast_sign_loss)]
                zend_hash_index_del(self, index as zend_ulong)
            },
            ArrayKey::String(key) => unsafe {
                zend_hash_str_del(
                    self,
                    CString::new(key.as_str()).ok()?.as_ptr(),
                    key.len() as _,
                )
            },
            ArrayKey::Str(key) => unsafe {
                zend_hash_str_del(self, CString::new(key).ok()?.as_ptr(), key.len() as _)
            },
        };

        if result < 0 { None } else { Some(()) }
    }

    /// Attempts to remove a value from the hash table with a string key.
    ///
    /// # Parameters
    ///
    /// * `key` - The key to remove from the hash table.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Key was successfully removed.
    /// * `None` - No key was removed, did not exist.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// ht.push("hello");
    /// assert_eq!(ht.len(), 1);
    ///
    /// ht.remove_index(0);
    /// assert_eq!(ht.len(), 0);
    /// ```
    pub fn remove_index(&mut self, key: i64) -> Option<()> {
        let result = unsafe {
            #[allow(clippy::cast_sign_loss)]
            zend_hash_index_del(self, key as zend_ulong)
        };

        if result < 0 { None } else { Some(()) }
    }

    /// Attempts to insert an item into the hash table, or update if the key
    /// already exists. Returns nothing in a result if successful.
    ///
    /// # Parameters
    ///
    /// * `key` - The key to insert the value at in the hash table.
    /// * `value` - The value to insert into the hash table.
    ///
    /// # Returns
    ///
    /// Returns nothing in a result on success.
    ///
    /// # Errors
    ///
    /// Returns an error if the key could not be converted into a [`CString`],
    /// or converting the value into a [`Zval`] failed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// ht.insert("a", "A");
    /// ht.insert("b", "B");
    /// ht.insert("c", "C");
    /// assert_eq!(ht.len(), 3);
    /// ```
    pub fn insert<'a, K, V>(&mut self, key: K, val: V) -> Result<()>
    where
        K: Into<ArrayKey<'a>>,
        V: IntoZval,
    {
        let mut val = val.into_zval(false)?;
        match key.into() {
            ArrayKey::Long(index) => {
                unsafe {
                    #[allow(clippy::cast_sign_loss)]
                    zend_hash_index_update(self, index as zend_ulong, &raw mut val)
                };
            }
            ArrayKey::String(key) => {
                unsafe {
                    // Use raw bytes directly since zend_hash_str_update takes a length.
                    // This allows keys with embedded null bytes (e.g. PHP property mangling).
                    zend_hash_str_update(
                        self,
                        key.as_str().as_ptr().cast(),
                        key.len(),
                        &raw mut val,
                    )
                };
            }
            ArrayKey::Str(key) => {
                unsafe {
                    // Use raw bytes directly since zend_hash_str_update takes a length.
                    // This allows keys with embedded null bytes (e.g. PHP property mangling).
                    zend_hash_str_update(self, key.as_ptr().cast(), key.len(), &raw mut val)
                };
            }
        }
        val.release();
        Ok(())
    }

    /// Inserts an item into the hash table at a specified index, or updates if
    /// the key already exists. Returns nothing in a result if successful.
    ///
    /// # Parameters
    ///
    /// * `key` - The index at which the value should be inserted.
    /// * `val` - The value to insert into the hash table.
    ///
    /// # Returns
    ///
    /// Returns nothing in a result on success.
    ///
    /// # Errors
    ///
    /// Returns an error if converting the value into a [`Zval`] failed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// ht.insert_at_index(0, "A");
    /// ht.insert_at_index(5, "B");
    /// ht.insert_at_index(0, "C"); // notice overriding index 0
    /// assert_eq!(ht.len(), 2);
    /// ```
    pub fn insert_at_index<V>(&mut self, key: i64, val: V) -> Result<()>
    where
        V: IntoZval,
    {
        let mut val = val.into_zval(false)?;
        unsafe {
            #[allow(clippy::cast_sign_loss)]
            zend_hash_index_update(self, key as zend_ulong, &raw mut val)
        };
        val.release();
        Ok(())
    }

    /// Pushes an item onto the end of the hash table. Returns a result
    /// containing nothing if the element was successfully inserted.
    ///
    /// # Parameters
    ///
    /// * `val` - The value to insert into the hash table.
    ///
    /// # Returns
    ///
    /// Returns nothing in a result on success.
    ///
    /// # Errors
    ///
    /// Returns an error if converting the value into a [`Zval`] failed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// ht.push("a");
    /// ht.push("b");
    /// ht.push("c");
    /// assert_eq!(ht.len(), 3);
    /// ```
    pub fn push<V>(&mut self, val: V) -> Result<()>
    where
        V: IntoZval,
    {
        let mut val = val.into_zval(false)?;
        unsafe { zend_hash_next_index_insert(self, &raw mut val) };
        val.release();

        Ok(())
    }

    /// Checks if the hashtable only contains numerical keys.
    ///
    /// # Returns
    ///
    /// True if all keys on the hashtable are numerical.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// ht.push(0);
    /// ht.push(3);
    /// ht.push(9);
    /// assert!(ht.has_numerical_keys());
    ///
    /// ht.insert("obviously not numerical", 10);
    /// assert!(!ht.has_numerical_keys());
    /// ```
    #[must_use]
    pub fn has_numerical_keys(&self) -> bool {
        !self.into_iter().any(|(k, _)| !k.is_long())
    }

    /// Checks if the hashtable has numerical, sequential keys.
    ///
    /// # Returns
    ///
    /// True if all keys on the hashtable are numerical and are in sequential
    /// order (i.e. starting at 0 and not skipping any keys).
    ///
    /// # Panics
    ///
    /// Panics if the number of elements in the hashtable exceeds `i64::MAX`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// ht.push(0);
    /// ht.push(3);
    /// ht.push(9);
    /// assert!(ht.has_sequential_keys());
    ///
    /// ht.insert_at_index(90, 10);
    /// assert!(!ht.has_sequential_keys());
    /// ```
    #[must_use]
    pub fn has_sequential_keys(&self) -> bool {
        !self
            .into_iter()
            .enumerate()
            .any(|(i, (k, _))| ArrayKey::Long(i64::try_from(i).expect("Integer overflow")) != k)
    }

    /// Returns an iterator over the values contained inside the hashtable, as
    /// if it was a set or list.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// for val in ht.values() {
    ///     dbg!(val);
    /// }
    #[inline]
    #[must_use]
    pub fn values(&self) -> Values<'_> {
        Values::new(self)
    }

    /// Returns an iterator over the key(s) and value contained inside the
    /// hashtable.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::{ZendHashTable, ArrayKey};
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// for (key, val) in ht.iter() {
    ///     match &key {
    ///         ArrayKey::Long(index) => {
    ///         }
    ///         ArrayKey::String(key) => {
    ///         }
    ///         ArrayKey::Str(key) => {
    ///         }
    ///     }
    ///     dbg!(key, val);
    /// }
    #[inline]
    #[must_use]
    pub fn iter(&self) -> Iter<'_> {
        self.into_iter()
    }

    /// Determines whether this hashtable is immutable.
    ///
    /// Immutable hashtables are shared and cannot be modified. The primary
    /// example is the empty immutable shared array returned by
    /// [`ZendEmptyArray`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let ht = ZendHashTable::new();
    /// assert!(!ht.is_immutable());
    /// ```
    #[must_use]
    pub fn is_immutable(&self) -> bool {
        // SAFETY: Type info is initialized by Zend on array init.
        let gc_type_info = unsafe { self.gc.u.type_info };
        let gc_flags = (gc_type_info >> GC_FLAGS_SHIFT) & (GC_FLAGS_MASK >> GC_FLAGS_SHIFT);

        gc_flags & ZvalTypeFlags::Immutable.bits() != 0
    }
}

unsafe impl ZBoxable for ZendHashTable {
    fn free(&mut self) {
        // Do not attempt to free the immutable shared empty array.
        if self.is_immutable() {
            return;
        }
        // SAFETY: ZBox has immutable access to `self`.
        unsafe { zend_array_destroy(self) }
    }
}

impl Debug for ZendHashTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(self.into_iter().map(|(k, v)| (k.to_string(), v)))
            .finish()
    }
}

impl ToOwned for ZendHashTable {
    type Owned = ZBox<ZendHashTable>;

    fn to_owned(&self) -> Self::Owned {
        unsafe {
            // SAFETY: FFI call does not modify `self`, returns a new hashtable.
            let ptr = zend_array_dup(ptr::from_ref(self).cast_mut());

            // SAFETY: `as_mut()` checks if the pointer is null, and panics if it is not.
            ZBox::from_raw(
                ptr.as_mut()
                    .expect("Failed to allocate memory for hashtable"),
            )
        }
    }
}

impl Default for ZBox<ZendHashTable> {
    fn default() -> Self {
        ZendHashTable::new()
    }
}

impl Clone for ZBox<ZendHashTable> {
    fn clone(&self) -> Self {
        (**self).to_owned()
    }
}

impl IntoZval for ZBox<ZendHashTable> {
    const TYPE: DataType = DataType::Array;
    const NULLABLE: bool = false;

    fn set_zval(self, zv: &mut Zval, _: bool) -> Result<()> {
        zv.set_hashtable(self);
        Ok(())
    }
}

impl<'a> FromZval<'a> for &'a ZendHashTable {
    const TYPE: DataType = DataType::Array;

    fn from_zval(zval: &'a Zval) -> Option<Self> {
        zval.array()
    }
}

impl<'a> FromZvalMut<'a> for &'a mut ZendHashTable {
    const TYPE: DataType = DataType::Array;

    fn from_zval_mut(zval: &'a mut Zval) -> Option<Self> {
        zval.array_mut()
    }
}

/// Represents an empty, immutable, shared PHP array.
///
/// Since PHP 7.3, it's possible for extensions to return a zval backed by
/// an immutable shared hashtable. This helps avoid redundant hashtable
/// allocations when returning empty arrays to userland PHP code.
///
/// This struct provides a safe way to return an empty array without allocating
/// a new hashtable. It implements [`IntoZval`] so it can be used as a return
/// type for PHP functions.
///
/// # Safety
///
/// Unlike [`ZendHashTable`], this type does not allow any mutation of the
/// underlying array, as it points to a shared static empty array in PHP's
/// memory.
///
/// # Example
///
/// ```rust,ignore
/// use ext_php_rs::prelude::*;
/// use ext_php_rs::types::ZendEmptyArray;
///
/// #[php_function]
/// pub fn get_empty_array() -> ZendEmptyArray {
///     ZendEmptyArray
/// }
/// ```
///
/// This is more efficient than returning `Vec::<i32>::new()` or creating
/// a new `ZendHashTable` when you know the result will be empty.
#[derive(Debug, Clone, Copy, Default)]
pub struct ZendEmptyArray;

impl ZendEmptyArray {
    /// Returns a reference to the underlying immutable empty hashtable.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendEmptyArray;
    ///
    /// let empty = ZendEmptyArray;
    /// let ht = empty.as_hashtable();
    /// assert!(ht.is_empty());
    /// assert!(ht.is_immutable());
    /// ```
    #[must_use]
    pub fn as_hashtable(&self) -> &ZendHashTable {
        // SAFETY: zend_empty_array is a static global initialized by PHP.
        unsafe { &zend_empty_array }
    }
}

impl IntoZval for ZendEmptyArray {
    const TYPE: DataType = DataType::Array;
    const NULLABLE: bool = false;

    fn set_zval(self, zv: &mut Zval, _persistent: bool) -> Result<()> {
        // Set the zval to point to the immutable shared empty array.
        // This mirrors the ZVAL_EMPTY_ARRAY macro in PHP.
        zv.u1.type_info = ZvalTypeFlags::Array.bits();
        zv.value.arr = ptr::from_ref(self.as_hashtable()).cast_mut();
        Ok(())
    }
}
