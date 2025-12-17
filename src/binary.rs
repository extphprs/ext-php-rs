//! Provides implementations for converting to and from Zend binary strings,
//! commonly returned from functions such as [`pack`] and [`unpack`].
//!
//! [`pack`]: https://www.php.net/manual/en/function.pack.php
//! [`unpack`]: https://www.php.net/manual/en/function.unpack.php

use crate::ffi::{ext_php_rs_zend_string_init, zend_string};

use std::{
    convert::TryFrom,
    iter::FromIterator,
    ops::{Deref, DerefMut},
};

use crate::{
    convert::{FromZval, IntoZval},
    error::{Error, Result},
    flags::DataType,
    types::Zval,
};

/// Acts as a wrapper around [`Vec<T>`] where `T` implements [`Pack`]. Primarily
/// used for passing binary data into Rust functions. Can be treated as a
/// [`Vec`] in most situations, or can be 'unwrapped' into a [`Vec`] through the
/// [`From`] implementation on [`Vec`].
#[derive(Debug)]
pub struct Binary<T: Pack>(Vec<T>);

impl<T: Pack> Binary<T> {
    /// Creates a new binary wrapper from a set of data which can be converted
    /// into a vector.
    ///
    /// # Parameters
    ///
    /// * `data` - Data to store inside the binary wrapper.
    pub fn new(data: impl Into<Vec<T>>) -> Self {
        Self(data.into())
    }
}

impl<T: Pack> Deref for Binary<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Pack> DerefMut for Binary<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Pack> FromZval<'_> for Binary<T> {
    const TYPE: DataType = DataType::String;

    fn from_zval(zval: &Zval) -> Option<Self> {
        zval.binary().map(Binary)
    }
}

impl<T: Pack> TryFrom<Zval> for Binary<T> {
    type Error = Error;

    fn try_from(value: Zval) -> Result<Self> {
        Self::from_zval(&value).ok_or_else(|| Error::ZvalConversion(value.get_type()))
    }
}

impl<T: Pack> IntoZval for Binary<T> {
    const TYPE: DataType = DataType::String;
    const NULLABLE: bool = false;

    fn set_zval(self, zv: &mut Zval, _: bool) -> Result<()> {
        zv.set_binary(self.0);
        Ok(())
    }
}

impl<T: Pack> From<Binary<T>> for Vec<T> {
    fn from(value: Binary<T>) -> Self {
        value.0
    }
}

impl<T: Pack> From<Vec<T>> for Binary<T> {
    fn from(value: Vec<T>) -> Self {
        Self::new(value)
    }
}

impl<T: Pack> FromIterator<T> for Binary<T> {
    fn from_iter<U: IntoIterator<Item = T>>(iter: U) -> Self {
        Self(iter.into_iter().collect::<Vec<_>>())
    }
}

/// Used to convert between Zend binary strings and vectors. Useful in
/// conjunction with the [`pack`] and [`unpack`] functions built-in to PHP.
///
/// # Safety
///
/// The types cannot be ensured between PHP and Rust, as the data is represented
/// as a string when crossing the language boundary. Exercise caution when using
/// these functions.
///
/// [`pack`]: https://www.php.net/manual/en/function.pack.php
/// [`unpack`]: https://www.php.net/manual/en/function.unpack.php
pub unsafe trait Pack: Clone {
    /// Packs a given vector into a Zend binary string. Can be passed to PHP and
    /// then unpacked using the [`unpack`] function.
    ///
    /// # Parameters
    ///
    /// * `vec` - The vector to pack into a binary string.
    ///
    /// [`unpack`]: https://www.php.net/manual/en/function.unpack.php
    fn pack_into(vec: Vec<Self>) -> *mut zend_string;

    /// Unpacks a given Zend binary string into a Rust vector. Can be used to
    /// pass data from `pack` in PHP to Rust without encoding into another
    /// format. Note that the data *must* be all one type, as this
    /// implementation only unpacks one type.
    ///
    /// # Safety
    ///
    /// There is no way to tell if the data stored in the string is actually of
    /// the given type. The results of this function can also differ from
    /// platform-to-platform due to the different representation of some
    /// types on different platforms. Consult the [`pack`] function
    /// documentation for more details.
    ///
    /// # Parameters
    ///
    /// * `s` - The Zend string containing the binary data.
    ///
    /// [`pack`]: https://www.php.net/manual/en/function.pack.php
    fn unpack_into(s: &zend_string) -> Vec<Self>;
}

/// Implements the [`Pack`] trait for a given type.
macro_rules! pack_impl {
    ($t: ty) => {
        pack_impl!($t, <$t>::BITS);
    };

    ($t: ty, $d: expr) => {
        unsafe impl Pack for $t {
            fn pack_into(vec: Vec<Self>) -> *mut zend_string {
                let len = vec.len() * ($d as usize / 8);
                let ptr = vec.as_ptr();
                unsafe { ext_php_rs_zend_string_init(ptr.cast(), len as _, false) }
            }

            fn unpack_into(s: &zend_string) -> Vec<Self> {
                #[allow(clippy::cast_lossless)]
                let bytes = ($d / 8) as u64;
                let len = (s.len as u64) / bytes;
                let mut result =
                    Vec::with_capacity(len.try_into().expect("Capacity integer overflow"));
                // TODO: Check alignment
                #[allow(clippy::cast_ptr_alignment)]
                let ptr = s.val.as_ptr().cast::<$t>();

                // SAFETY: We calculate the length of memory that we can legally read based on
                // the side of the type, therefore we never read outside the memory we
                // should.
                for i in 0..len {
                    result.push(unsafe {
                        *ptr.offset(i.try_into().expect("Offset integer overflow"))
                    });
                }

                result
            }
        }
    };
}

pack_impl!(u8);
pack_impl!(i8);

pack_impl!(u16);
pack_impl!(i16);

pack_impl!(u32);
pack_impl!(i32);

pack_impl!(u64);
pack_impl!(i64);

pack_impl!(isize);
pack_impl!(usize);

pack_impl!(f32, 32);
pack_impl!(f64, 64);
