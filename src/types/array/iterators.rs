use cfg_if::cfg_if;
use std::{
    convert::TryInto,
    iter::{DoubleEndedIterator, ExactSizeIterator, Iterator},
    ptr,
};

use super::{ArrayKey, ZendHashTable};
use crate::boxed::ZBox;
use crate::{
    ffi::{
        HashPosition, zend_hash_get_current_data_ex, zend_hash_get_current_key_type_ex,
        zend_hash_get_current_key_zval_ex, zend_hash_internal_pointer_end_ex,
        zend_hash_move_backwards_ex, zend_hash_move_forward_ex,
    },
    types::Zval,
};

#[cfg(php85)]
use crate::ffi::zend_hash_key_type_HASH_KEY_NON_EXISTENT;

/// Immutable iterator upon a reference to a hashtable.
pub struct Iter<'a> {
    ht: &'a ZendHashTable,
    current_num: i64,
    end_num: i64,
    pos: HashPosition,
    end_pos: HashPosition,
}

impl<'a> Iter<'a> {
    /// Creates a new iterator over a hashtable.
    ///
    /// # Parameters
    ///
    /// * `ht` - The hashtable to iterate.
    ///
    /// # Panics
    ///
    /// Panics if the hashtable length exceeds `i64::MAX`.
    #[must_use]
    #[expect(
        clippy::expect_used,
        reason = "a hash table cannot hold more than i64::MAX elements"
    )]
    pub fn new(ht: &'a ZendHashTable) -> Self {
        let end_num: i64 = ht
            .len()
            .try_into()
            .expect("Integer overflow in hashtable length");
        let mut end_pos = 0;
        unsafe {
            zend_hash_internal_pointer_end_ex(ptr::from_ref(ht).cast_mut(), &raw mut end_pos);
        }

        Self {
            ht,
            current_num: 0,
            end_num,
            pos: 0,
            end_pos,
        }
    }

    /// Advances the iterator and returns the next key-value pair as raw Zvals.
    ///
    /// Returns `None` when iteration is finished.
    pub fn next_zval(&mut self) -> Option<(Zval, &'a Zval)> {
        if self.current_num >= self.end_num {
            return None;
        }

        let key_type = unsafe {
            zend_hash_get_current_key_type_ex(ptr::from_ref(self.ht).cast_mut(), &raw mut self.pos)
        };

        // Key type `1` is string (HASH_KEY_IS_STRING)
        // Key type `2` is long (HASH_KEY_IS_LONG)
        // Key type `3` is null/non-existent (HASH_KEY_NON_EXISTENT)
        // Pre-PHP 8.5 returns int, PHP 8.5+ returns enum u32
        cfg_if! {
            if #[cfg(php85)] {
                if key_type == zend_hash_key_type_HASH_KEY_NON_EXISTENT {
                    return None;
                }
            } else {
                // Pre-PHP 8.5: function returns signed int (i32)
                // Check for -1 (defensive) and 3 (HASH_KEY_NON_EXISTENT)
                if key_type == -1 || key_type == 3 {
                    return None;
                }
            }
        }

        let mut key = Zval::new();

        unsafe {
            zend_hash_get_current_key_zval_ex(
                ptr::from_ref(self.ht).cast_mut(),
                (&raw const key).cast_mut(),
                &raw mut self.pos,
            );
        }
        let value = unsafe {
            let val_ptr =
                zend_hash_get_current_data_ex(ptr::from_ref(self.ht).cast_mut(), &raw mut self.pos);

            if val_ptr.is_null() {
                return None;
            }

            &*val_ptr
        };

        if !key.is_long() && !key.is_string() {
            key.set_long(self.current_num);
        }

        unsafe { zend_hash_move_forward_ex(ptr::from_ref(self.ht).cast_mut(), &raw mut self.pos) };
        self.current_num += 1;

        Some((key, value))
    }

    /// # Safety
    ///
    /// `key` must be filled by `zend_hash_get_current_key_zval_ex` from the
    /// iterated table, so a string key is the bucket's own `zend_string`,
    /// which the table keeps alive for `'a`.
    unsafe fn array_key(key: &Zval) -> Option<ArrayKey<'a>> {
        if let Some(index) = key.long() {
            return Some(ArrayKey::Long(index));
        }
        let key = key.zend_str()?;
        Some(match key.as_str() {
            Ok(key) => ArrayKey::String(key.to_owned()),
            // SAFETY: the caller guarantees the table owns this string for `'a`.
            Err(_) => ArrayKey::ZendString(unsafe { &*ptr::from_ref(key) }),
        })
    }
}

impl<'a> IntoIterator for &'a ZendHashTable {
    type Item = (ArrayKey<'a>, &'a Zval);
    type IntoIter = Iter<'a>;

    /// Returns an iterator over the key(s) and value contained inside the
    /// hashtable.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendHashTable;
    ///
    /// let mut ht = ZendHashTable::new();
    ///
    /// for (key, val) in ht.iter() {
    /// //   ^ Index if inserted at an index.
    /// //        ^ Optional string key, if inserted like a hashtable.
    /// //             ^ Inserted value.
    ///
    ///     dbg!(key, val);
    /// }
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Iter::new(self)
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (ArrayKey<'a>, &'a Zval);

    #[expect(
        clippy::expect_used,
        reason = "next_zval only yields long or string keys"
    )]
    fn next(&mut self) -> Option<Self::Item> {
        let (key, value) = self.next_zval()?;
        // SAFETY: `next_zval` reads `key` from `self.ht`.
        let key = unsafe { Self::array_key(&key) }.expect("Invalid array key!");
        Some((key, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = usize::try_from(self.end_num - self.current_num).unwrap_or(0);
        (remaining, Some(remaining))
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.len()
    }
}

impl ExactSizeIterator for Iter<'_> {}

impl DoubleEndedIterator for Iter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.end_num <= self.current_num {
            return None;
        }

        let key_type = unsafe {
            zend_hash_get_current_key_type_ex(
                ptr::from_ref(self.ht).cast_mut(),
                &raw mut self.end_pos,
            )
        };

        cfg_if! {
            if #[cfg(php85)] {
                if key_type == zend_hash_key_type_HASH_KEY_NON_EXISTENT {
                    return None;
                }
            } else {
                // Pre-PHP 8.5: function returns signed int (i32)
                // Check for -1 (defensive) and 3 (HASH_KEY_NON_EXISTENT)
                if key_type == -1 || key_type == 3 {
                    return None;
                }
            }
        }

        let key = Zval::new();

        unsafe {
            zend_hash_get_current_key_zval_ex(
                ptr::from_ref(self.ht).cast_mut(),
                (&raw const key).cast_mut(),
                &raw mut self.end_pos,
            );
        }
        let value = unsafe {
            zend_hash_get_current_data_ex(ptr::from_ref(self.ht).cast_mut(), &raw mut self.end_pos)
                .as_ref()
        }?;

        // SAFETY: `key` was read from `self.ht` at `end_pos`.
        let key = match unsafe { Self::array_key(&key) } {
            Some(key) => key,
            None => ArrayKey::Long(self.end_num),
        };

        unsafe {
            zend_hash_move_backwards_ex(ptr::from_ref(self.ht).cast_mut(), &raw mut self.end_pos)
        };
        self.end_num -= 1;

        Some((key, value))
    }
}

/// Immutable iterator which iterates over the values of the hashtable, as it
/// was a set or list.
pub struct Values<'a>(Iter<'a>);

impl<'a> Values<'a> {
    /// Creates a new iterator over a hashtables values.
    ///
    /// # Parameters
    ///
    /// * `ht` - The hashtable to iterate.
    #[must_use]
    pub fn new(ht: &'a ZendHashTable) -> Self {
        Self(Iter::new(ht))
    }
}

impl<'a> Iterator for Values<'a> {
    type Item = &'a Zval;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(_, zval)| zval)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    fn count(self) -> usize
    where
        Self: Sized,
    {
        self.0.count()
    }
}

impl ExactSizeIterator for Values<'_> {}

impl DoubleEndedIterator for Values<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(|(_, zval)| zval)
    }
}

impl FromIterator<Zval> for ZBox<ZendHashTable> {
    fn from_iter<T: IntoIterator<Item = Zval>>(iter: T) -> Self {
        let mut ht = ZendHashTable::new();
        for item in iter {
            // Inserting a zval cannot fail, as `push` only returns `Err` if converting
            // `val` to a zval fails.
            let _ = ht.push(item);
        }
        ht
    }
}

impl FromIterator<(i64, Zval)> for ZBox<ZendHashTable> {
    fn from_iter<T: IntoIterator<Item = (i64, Zval)>>(iter: T) -> Self {
        let mut ht = ZendHashTable::new();
        for (key, val) in iter {
            // Inserting a zval cannot fail, as `push` only returns `Err` if converting
            // `val` to a zval fails.
            let _ = ht.insert_at_index(key, val);
        }
        ht
    }
}

impl<'a> FromIterator<(&'a str, Zval)> for ZBox<ZendHashTable> {
    fn from_iter<T: IntoIterator<Item = (&'a str, Zval)>>(iter: T) -> Self {
        let mut ht = ZendHashTable::new();
        for (key, val) in iter {
            // Inserting a zval cannot fail, as `push` only returns `Err` if converting
            // `val` to a zval fails.
            let _ = ht.insert(key, val);
        }
        ht
    }
}
