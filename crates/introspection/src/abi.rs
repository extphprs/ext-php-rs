//! ABI-stable standard library types.
//!
//! The description module is used by the `cargo-php` sub-command to retrieve
//! information about the extension. As Rust does not have a stable ABI, it is
//! not as simple as working in the Rust domain, as if the CLI and extension
//! Rust versions do not match, it cannot be assumed that the types have the
//! same memory layout.
//!
//! This module contains thin wrappers around standard library types used by the
//! describe function to provide some sort of ABI-stability.
//!
//! As a general rule of thumb, no Rust type is ABI-stable. Strictly speaking,
//! [`usize`] should not be in use, but rather `size_t` or a similar type,
//! however these are currently unstable.

use std::{fmt::Display, ops::Deref, vec::Vec as StdVec};

/// An immutable, ABI-stable [`Vec`][std::vec::Vec].
#[repr(C)]
#[derive(Debug)]
pub struct Vec<T> {
    ptr: *mut T,
    len: usize,
}

impl<T> Deref for Vec<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        unsafe {
            let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(self.ptr, self.len));
        };
    }
}

impl<T> From<StdVec<T>> for Vec<T> {
    fn from(vec: StdVec<T>) -> Self {
        let vec = vec.into_boxed_slice();
        let len = vec.len();
        let ptr = Box::into_raw(vec).cast::<T>();

        Self { ptr, len }
    }
}

impl<T> PartialEq for Vec<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.as_ref() == other.as_ref()
    }
}

/// An immutable, ABI-stable borrowed [`&'static str`][str].
#[repr(C)]
#[derive(Debug)]
pub struct Str {
    ptr: *const u8,
    len: usize,
}

impl Str {
    /// Returns the string as a string slice.
    ///
    /// The lifetime is `'static` and can outlive the [`Str`] object, as you can
    /// only initialize a [`Str`] through a static reference.
    #[must_use]
    pub fn str(&self) -> &'static str {
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.ptr, self.len)) }
    }
}

impl From<&'static str> for Str {
    fn from(val: &'static str) -> Self {
        let ptr = val.as_ptr();
        let len = val.len();
        Self { ptr, len }
    }
}

impl AsRef<str> for Str {
    fn as_ref(&self) -> &str {
        self.str()
    }
}

impl Display for Str {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.str().fmt(f)
    }
}

impl PartialEq for Str {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.str() == other.str()
    }
}

/// An ABI-stable String
#[repr(C)]
#[derive(Debug, PartialEq)]
pub struct RString {
    inner: Vec<u8>,
}

impl RString {
    /// Returns the string as a string slice.
    ///
    /// # Panics
    ///
    /// * If the string is not valid UTF-8
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.inner).expect("RString value is not valid UTF-8")
    }
}

impl From<&str> for RString {
    fn from(s: &str) -> Self {
        Self {
            inner: s.as_bytes().to_vec().into(),
        }
    }
}

impl From<String> for RString {
    fn from(s: String) -> Self {
        Self {
            inner: s.into_bytes().into(),
        }
    }
}

impl AsRef<str> for RString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for RString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

/// An ABI-stable [`Option`][std::option::Option].
#[repr(C, u8)]
#[derive(Debug)]
pub enum Option<T> {
    /// [`Option::Some`][std::option::Option::Some] variant.
    Some(T),
    /// [`Option::None`][std::option::Option::None] variant.
    None,
}

impl<T> From<std::option::Option<T>> for Option<T> {
    fn from(opt: std::option::Option<T>) -> Self {
        match opt {
            Some(val) => Self::Some(val),
            None => Self::None,
        }
    }
}

impl<T> PartialEq for Option<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Some(a), Self::Some(b)) => a == b,
            (Self::None, Self::None) => true,
            _ => false,
        }
    }
}
