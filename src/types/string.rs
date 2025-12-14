//! Represents a string in the PHP world. Similar to a C string, but is
//! reference counted and contains the length of the string.

use std::{
    borrow::Cow,
    convert::TryFrom,
    ffi::{CStr, CString},
    fmt::Debug,
    ptr, slice,
};

use parking_lot::{Mutex, const_mutex};

use crate::{
    boxed::{ZBox, ZBoxable},
    convert::{FromZval, IntoZval},
    error::{Error, Result},
    ffi::{
        ext_php_rs_is_known_valid_utf8, ext_php_rs_set_known_valid_utf8,
        ext_php_rs_zend_string_init, ext_php_rs_zend_string_release, zend_string,
        zend_string_init_interned,
    },
    flags::DataType,
    types::Zval,
};

/// A borrowed Zend string.
///
/// Although this object does implement [`Sized`], it is in fact not sized. As C
/// cannot represent unsized types, an array of size 1 is used at the end of the
/// type to represent the contents of the string, therefore this type is
/// actually unsized. All constructors return [`ZBox<ZendStr>`], the owned
/// variant.
///
/// Once the `ptr_metadata` feature lands in stable rust, this type can
/// potentially be changed to a DST using slices and metadata. See the tracking issue here: <https://github.com/rust-lang/rust/issues/81513>
pub type ZendStr = zend_string;

// Adding to the Zend interned string hashtable is not atomic and can be
// contested when PHP is compiled with ZTS, so an empty mutex is used to ensure
// no collisions occur on the Rust side. Not much we can do about collisions
// on the PHP side, but some safety is better than none.
static INTERNED_LOCK: Mutex<()> = const_mutex(());

// Clippy complains about there being no `is_empty` function when implementing
// on the alias `ZendStr` :( <https://github.com/rust-lang/rust-clippy/issues/7702>
#[allow(clippy::len_without_is_empty)]
impl ZendStr {
    /// Creates a new Zend string from a slice of bytes.
    ///
    /// # Parameters
    ///
    /// * `str` - String content.
    /// * `persistent` - Whether the string should persist through the request
    ///   boundary.
    ///
    /// # Panics
    ///
    /// Panics if the function was unable to allocate memory for the Zend
    /// string.
    ///
    /// # Safety
    ///
    /// When passing `persistent` as `false`, the caller must ensure that the
    /// object does not attempt to live after the request finishes. When a
    /// request starts and finishes in PHP, the Zend heap is deallocated and a
    /// new one is created, which would leave a dangling pointer in the
    /// [`ZBox`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendStr;
    ///
    /// let s = ZendStr::new("Hello, world!", false);
    /// let php = ZendStr::new([80, 72, 80], false);
    /// ```
    pub fn new(str: impl AsRef<[u8]>, persistent: bool) -> ZBox<Self> {
        let s = str.as_ref();
        // TODO: we should handle the special cases when length is either 0 or 1
        // see `zend_string_init_fast()` in `zend_string.h`
        unsafe {
            let ptr = ext_php_rs_zend_string_init(s.as_ptr().cast(), s.len(), persistent)
                .as_mut()
                .expect("Failed to allocate memory for new Zend string");
            ZBox::from_raw(ptr)
        }
    }

    /// Creates a new Zend string from a [`CStr`].
    ///
    /// # Parameters
    ///
    /// * `str` - String content.
    /// * `persistent` - Whether the string should persist through the request
    ///   boundary.
    ///
    /// # Panics
    ///
    /// Panics if the function was unable to allocate memory for the Zend
    /// string.
    ///
    /// # Safety
    ///
    /// When passing `persistent` as `false`, the caller must ensure that the
    /// object does not attempt to live after the request finishes. When a
    /// request starts and finishes in PHP, the Zend heap is deallocated and a
    /// new one is created, which would leave a dangling pointer in the
    /// [`ZBox`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendStr;
    /// use std::ffi::CString;
    ///
    /// let c_s = CString::new("Hello world!").unwrap();
    /// let s = ZendStr::from_c_str(&c_s, false);
    /// ```
    #[must_use]
    pub fn from_c_str(str: &CStr, persistent: bool) -> ZBox<Self> {
        unsafe {
            let ptr =
                ext_php_rs_zend_string_init(str.as_ptr(), str.to_bytes().len() as _, persistent);

            ZBox::from_raw(
                ptr.as_mut()
                    .expect("Failed to allocate memory for new Zend string"),
            )
        }
    }

    /// Creates a new interned Zend string from a slice of bytes.
    ///
    /// An interned string is only ever stored once and is immutable. PHP stores
    /// the string in an internal hashtable which stores the interned
    /// strings.
    ///
    /// As Zend hashtables are not thread-safe, a mutex is used to prevent two
    /// interned strings from being created at the same time.
    ///
    /// Interned strings are not used very often. You should almost always use a
    /// regular zend string, except in the case that you know you will use a
    /// string that PHP will already have interned, such as "PHP".
    ///
    /// # Parameters
    ///
    /// * `str` - String content.
    /// * `persistent` - Whether the string should persist through the request
    ///   boundary.
    ///
    /// # Panics
    ///
    /// Panics under the following circumstances:
    ///
    /// * The function used to create interned strings has not been set.
    /// * The function could not allocate enough memory for the Zend string.
    ///
    /// # Safety
    ///
    /// When passing `persistent` as `false`, the caller must ensure that the
    /// object does not attempt to live after the request finishes. When a
    /// request starts and finishes in PHP, the Zend heap is deallocated and a
    /// new one is created, which would leave a dangling pointer in the
    /// [`ZBox`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendStr;
    ///
    /// let s = ZendStr::new_interned("PHP", true);
    /// ```
    pub fn new_interned(str: impl AsRef<[u8]>, persistent: bool) -> ZBox<Self> {
        let _lock = INTERNED_LOCK.lock();
        let s = str.as_ref();
        unsafe {
            let init = zend_string_init_interned.expect("`zend_string_init_interned` not ready");
            let ptr = init(s.as_ptr().cast(), s.len() as _, persistent)
                .as_mut()
                .expect("Failed to allocate memory for new Zend string");
            ZBox::from_raw(ptr)
        }
    }

    /// Creates a new interned Zend string from a [`CStr`].
    ///
    /// An interned string is only ever stored once and is immutable. PHP stores
    /// the string in an internal hashtable which stores the interned
    /// strings.
    ///
    /// As Zend hashtables are not thread-safe, a mutex is used to prevent two
    /// interned strings from being created at the same time.
    ///
    /// Interned strings are not used very often. You should almost always use a
    /// regular zend string, except in the case that you know you will use a
    /// string that PHP will already have interned, such as "PHP".
    ///
    /// # Parameters
    ///
    /// * `str` - String content.
    /// * `persistent` - Whether the string should persist through the request
    ///   boundary.
    ///
    /// # Panics
    ///
    /// Panics under the following circumstances:
    ///
    /// * The function used to create interned strings has not been set.
    /// * The function could not allocate enough memory for the Zend string.
    ///
    /// # Safety
    ///
    /// When passing `persistent` as `false`, the caller must ensure that the
    /// object does not attempt to live after the request finishes. When a
    /// request starts and finishes in PHP, the Zend heap is deallocated and a
    /// new one is created, which would leave a dangling pointer in the
    /// [`ZBox`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendStr;
    /// use std::ffi::CString;
    ///
    /// let c_s = CString::new("PHP").unwrap();
    /// let s = ZendStr::interned_from_c_str(&c_s, true);
    /// ```
    pub fn interned_from_c_str(str: &CStr, persistent: bool) -> ZBox<Self> {
        let _lock = INTERNED_LOCK.lock();

        unsafe {
            let init = zend_string_init_interned.expect("`zend_string_init_interned` not ready");
            let ptr = init(str.as_ptr(), str.to_bytes().len() as _, persistent);

            ZBox::from_raw(
                ptr.as_mut()
                    .expect("Failed to allocate memory for new Zend string"),
            )
        }
    }

    /// Returns the length of the string.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendStr;
    ///
    /// let s = ZendStr::new("hello, world!", false);
    /// assert_eq!(s.len(), 13);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if the string is empty, false otherwise.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendStr;
    ///
    /// let s = ZendStr::new("hello, world!", false);
    /// assert_eq!(s.is_empty(), false);
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Attempts to return a reference to the underlying bytes inside the Zend
    /// string as a [`CStr`].
    ///
    /// # Errors
    ///
    /// Returns an [`Error::InvalidCString`] variant if the string contains null
    /// bytes.
    pub fn as_c_str(&self) -> Result<&CStr> {
        let bytes_with_null =
            unsafe { slice::from_raw_parts(self.val.as_ptr().cast(), self.len() + 1) };
        CStr::from_bytes_with_nul(bytes_with_null).map_err(|_| Error::InvalidCString)
    }

    /// Attempts to return a reference to the underlying bytes inside the Zend
    /// string.
    ///
    /// # Errors
    ///
    /// Returns an [`Error::InvalidUtf8`] variant if the [`str`] contains
    /// non-UTF-8 characters.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendStr;
    ///
    /// let s = ZendStr::new("hello, world!", false);
    /// assert!(s.as_str().is_ok());
    /// ```
    pub fn as_str(&self) -> Result<&str> {
        if unsafe { ext_php_rs_is_known_valid_utf8(self.as_ptr()) } {
            let str = unsafe { std::str::from_utf8_unchecked(self.as_bytes()) };
            return Ok(str);
        }
        let str = std::str::from_utf8(self.as_bytes()).map_err(|_| Error::InvalidUtf8)?;
        unsafe { ext_php_rs_set_known_valid_utf8(self.as_ptr().cast_mut()) };
        Ok(str)
    }

    /// Returns a reference to the underlying bytes inside the Zend string.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.val.as_ptr().cast(), self.len()) }
    }

    /// Returns a raw pointer to this object
    #[must_use]
    pub fn as_ptr(&self) -> *const ZendStr {
        ptr::from_ref(self)
    }

    /// Returns a mutable pointer to this object
    pub fn as_mut_ptr(&mut self) -> *mut ZendStr {
        ptr::from_mut(self)
    }
}

unsafe impl ZBoxable for ZendStr {
    fn free(&mut self) {
        unsafe { ext_php_rs_zend_string_release(self) };
    }
}

impl Debug for ZendStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_str().fmt(f)
    }
}

impl AsRef<[u8]> for ZendStr {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl<T> PartialEq<T> for ZendStr
where
    T: AsRef<[u8]>,
{
    fn eq(&self, other: &T) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl ToOwned for ZendStr {
    type Owned = ZBox<ZendStr>;

    fn to_owned(&self) -> Self::Owned {
        Self::new(self.as_bytes(), false)
    }
}

impl<'a> TryFrom<&'a ZendStr> for &'a CStr {
    type Error = Error;

    fn try_from(value: &'a ZendStr) -> Result<Self> {
        value.as_c_str()
    }
}

impl<'a> TryFrom<&'a ZendStr> for &'a str {
    type Error = Error;

    fn try_from(value: &'a ZendStr) -> Result<Self> {
        value.as_str()
    }
}

impl TryFrom<&ZendStr> for String {
    type Error = Error;

    fn try_from(value: &ZendStr) -> Result<Self> {
        value.as_str().map(ToString::to_string)
    }
}

impl<'a> From<&'a ZendStr> for Cow<'a, ZendStr> {
    fn from(value: &'a ZendStr) -> Self {
        Cow::Borrowed(value)
    }
}

impl From<&CStr> for ZBox<ZendStr> {
    fn from(value: &CStr) -> Self {
        ZendStr::from_c_str(value, false)
    }
}

impl From<CString> for ZBox<ZendStr> {
    fn from(value: CString) -> Self {
        ZendStr::from_c_str(&value, false)
    }
}

impl From<&str> for ZBox<ZendStr> {
    fn from(value: &str) -> Self {
        ZendStr::new(value.as_bytes(), false)
    }
}

impl From<String> for ZBox<ZendStr> {
    fn from(value: String) -> Self {
        ZendStr::new(value.as_str(), false)
    }
}

impl From<ZBox<ZendStr>> for Cow<'_, ZendStr> {
    fn from(value: ZBox<ZendStr>) -> Self {
        Cow::Owned(value)
    }
}

impl From<Cow<'_, ZendStr>> for ZBox<ZendStr> {
    fn from(value: Cow<'_, ZendStr>) -> Self {
        value.into_owned()
    }
}

macro_rules! try_into_zval_str {
    ($type: ty) => {
        impl TryFrom<$type> for Zval {
            type Error = Error;

            fn try_from(value: $type) -> Result<Self> {
                let mut zv = Self::new();
                zv.set_string(&value, false)?;
                Ok(zv)
            }
        }

        impl IntoZval for $type {
            const TYPE: DataType = DataType::String;
            const NULLABLE: bool = false;

            fn set_zval(self, zv: &mut Zval, persistent: bool) -> Result<()> {
                zv.set_string(&self, persistent)
            }
        }
    };
}

try_into_zval_str!(String);
try_into_zval_str!(&str);
try_from_zval!(String, string, String);

impl<'a> FromZval<'a> for &'a str {
    const TYPE: DataType = DataType::String;

    fn from_zval(zval: &'a Zval) -> Option<Self> {
        zval.str()
    }
}

#[cfg(test)]
#[cfg(feature = "embed")]
mod tests {
    use crate::embed::Embed;

    #[test]
    fn test_string() {
        Embed::run(|| {
            let result = Embed::eval("'foo';");

            assert!(result.is_ok());

            let zval = result.as_ref().expect("Unreachable");

            assert!(zval.is_string());
            assert_eq!(zval.string(), Some("foo".to_string()));
        });
    }
}
