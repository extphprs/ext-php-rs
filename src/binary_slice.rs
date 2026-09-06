//! Provides implementations for converting from Zend binary strings as slices,
//! commonly returned from functions such as [`pack`] and [`unpack`].
//!
//! [`pack`]: https://www.php.net/manual/en/function.pack.php
//! [`unpack`]: https://www.php.net/manual/en/function.unpack.php

use crate::ffi::zend_string;

use std::{ops::Deref, slice::from_raw_parts};

use crate::{convert::FromZval, flags::DataType, types::Zval};

/// Acts as a wrapper around `&[T]` where `T` implements [`PackSlice`].
/// Primarily used for passing read-only binary data into Rust functions.
#[derive(Debug)]
pub struct BinarySlice<'a, T>(&'a [T])
where
    T: PackSlice;

impl<'a, T> BinarySlice<'a, T>
where
    T: PackSlice,
{
    /// Creates a new binary slice wrapper from a slice of data.
    ///
    /// # Parameters
    ///
    /// * `data` - Slice to store inside the binary wrapper.
    pub fn new(data: &'a [T]) -> Self {
        Self(data)
    }
}

impl<'a, T> Deref for BinarySlice<'a, T>
where
    T: PackSlice,
{
    type Target = &'a [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, T> FromZval<'a> for BinarySlice<'a, T>
where
    T: PackSlice,
{
    const TYPE: DataType = DataType::String;

    fn from_zval(zval: &'a Zval) -> Option<Self> {
        zval.binary_slice().map(BinarySlice)
    }
}

impl<'a, T> From<BinarySlice<'a, T>> for &'a [T]
where
    T: PackSlice,
{
    fn from(value: BinarySlice<'a, T>) -> Self {
        value.0
    }
}

impl<'a, T> From<&'a [T]> for BinarySlice<'a, T>
where
    T: PackSlice,
{
    fn from(value: &'a [T]) -> Self {
        Self::new(value)
    }
}

/// Used to expose a Zend binary string as a slice. Useful in conjunction with
/// the [`pack`] and [`unpack`] functions built-in to PHP.
///
/// # Safety
///
/// The types cannot be ensured between PHP and Rust, as the data is represented
/// as a string when crossing the language boundary. Exercise caution when using
/// these functions.
///
/// [`pack`]: https://www.php.net/manual/en/function.pack.php
/// [`unpack`]: https://www.php.net/manual/en/function.unpack.php
pub unsafe trait PackSlice: Clone {
    /// Creates a Rust slice from a given Zend binary string. Can be used to
    /// pass data from `pack` in PHP to Rust without encoding into another
    /// format. Note that the data *must* be all one type, as this
    /// implementation only unpacks one type.
    ///
    /// There is no way to tell if the data stored in the string is actually of
    /// the given type. The results of this function can also differ from
    /// platform-to-platform due to the different representation of some
    /// types on different platforms. Consult the [`pack`] function
    /// documentation for more details.
    ///
    /// # Safety
    ///
    /// The returned slice aliases the string's bytes, so the caller must
    /// guarantee:
    ///
    /// * `s.len` is a whole multiple of `size_of::<Self>()`.
    /// * `s.val` is aligned to `align_of::<Self>()`. Zend allocates strings on
    ///   `ZEND_MM_ALIGNMENT` (8 bytes) boundaries and `val` sits at an 8-byte
    ///   offset, but the `zend_string` type itself only promises 1-byte
    ///   alignment for `val`.
    ///
    /// [`Zval::binary_slice`](crate::types::Zval::binary_slice) performs both
    /// checks and is the safe entry point.
    ///
    /// # Parameters
    ///
    /// * `s` - The Zend string containing the binary data.
    ///
    /// [`pack`]: https://www.php.net/manual/en/function.pack.php
    unsafe fn unpack_into(s: &zend_string) -> &[Self];
}

/// Implements the [`PackSlice`] trait for a given type.
macro_rules! pack_slice_impl {
    ($t: ty) => {
        pack_slice_impl!($t, <$t>::BITS);
    };

    ($t: ty, $d: expr) => {
        unsafe impl PackSlice for $t {
            unsafe fn unpack_into(s: &zend_string) -> &[Self] {
                let len = s.len / ($d as usize / 8);
                // The caller guarantees `val` is aligned for `Self`, see `PackSlice`.
                #[allow(clippy::cast_ptr_alignment)]
                let ptr = s.val.as_ptr().cast::<$t>();
                // SAFETY: the caller guarantees alignment and that `len` whole elements
                // fit in the string's `s.len` readable bytes.
                unsafe { from_raw_parts(ptr, len) }
            }
        }
    };
}

pack_slice_impl!(u8);
pack_slice_impl!(i8);

pack_slice_impl!(u16);
pack_slice_impl!(i16);

pack_slice_impl!(u32);
pack_slice_impl!(i32);

pack_slice_impl!(u64);
pack_slice_impl!(i64);

pack_slice_impl!(isize);
pack_slice_impl!(usize);

pack_slice_impl!(f32, 32);
pack_slice_impl!(f64, 64);
