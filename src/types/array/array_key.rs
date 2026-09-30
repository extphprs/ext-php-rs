use crate::{
    boxed::ZBox, convert::FromZval, error::Error, flags::DataType, types::ZendStr, types::Zval,
};
use std::{convert::TryFrom, fmt::Display};

/// Represents the key of a PHP array, which can be either a long or a string.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArrayKey<'a> {
    /// A numerical key.
    /// In Zend API it's represented by `u64` (`zend_ulong`), so the value needs
    /// to be cast to `zend_ulong` before passing into Zend functions.
    Long(i64),
    /// A string key.
    String(String),
    /// A string key by reference.
    Str(&'a str),
    /// A PHP `zend_string` key.
    /// Allows bypassing repeated `zend_string_init` allocations and re-hashing
    /// when working with pre-existing or interned PHP strings
    ZendString(&'a ZendStr),
}

/// Parses a string array key as an integer the same way PHP does
/// (`ZEND_HANDLE_NUMERIC_STR`).
///
/// Only canonical decimal integers are numeric: an optional `-` followed by
/// digits without leading zeros, within range. Strings such as `"+1"`, `"-0"`
/// or `"01"` stay string keys.
fn parse_numeric_key(key: &str) -> Option<i64> {
    let digits = key.strip_prefix('-').unwrap_or(key);
    let canonical = match digits.as_bytes() {
        [b'0'] => key == "0",
        [b'1'..=b'9', rest @ ..] => rest.iter().all(u8::is_ascii_digit),
        _ => false,
    };
    if canonical { key.parse().ok() } else { None }
}

impl From<String> for ArrayKey<'_> {
    fn from(value: String) -> Self {
        match parse_numeric_key(&value) {
            Some(index) => Self::Long(index),
            None => Self::String(value),
        }
    }
}

impl TryFrom<ArrayKey<'_>> for String {
    type Error = Error;

    fn try_from(value: ArrayKey<'_>) -> Result<Self, Self::Error> {
        match value {
            ArrayKey::String(s) => Ok(s),
            ArrayKey::Str(s) => Ok(s.to_string()),
            ArrayKey::Long(l) => Ok(l.to_string()),
            ArrayKey::ZendString(s) => s.as_str().map(ToString::to_string),
        }
    }
}

impl TryFrom<ArrayKey<'_>> for i64 {
    type Error = Error;

    fn try_from(value: ArrayKey<'_>) -> Result<Self, Self::Error> {
        let key = match &value {
            ArrayKey::Long(i) => return Ok(*i),
            ArrayKey::String(s) => s.as_str(),
            ArrayKey::Str(s) => s,
            ArrayKey::ZendString(s) => s.as_str().map_err(|_| Error::InvalidUtf8)?,
        };

        parse_numeric_key(key).ok_or(Error::ZvalConversion(DataType::String))
    }
}

impl ArrayKey<'_> {
    /// Check if the key is an integer.
    ///
    /// # Returns
    ///
    /// Returns true if the key is an integer, false otherwise.
    #[must_use]
    pub fn is_long(&self) -> bool {
        match self {
            ArrayKey::Long(_) => true,
            ArrayKey::String(_) | ArrayKey::Str(_) | ArrayKey::ZendString(_) => false,
        }
    }
}

impl Display for ArrayKey<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArrayKey::Long(key) => write!(f, "{key}"),
            ArrayKey::String(key) => write!(f, "{key}"),
            ArrayKey::Str(key) => write!(f, "{key}"),
            ArrayKey::ZendString(key) => match key.as_str() {
                Ok(key) => write!(f, "{key}"),
                Err(_) => write!(f, "{}", String::from_utf8_lossy(key.as_bytes())),
            },
        }
    }
}

impl<'a> From<&'a str> for ArrayKey<'a> {
    fn from(value: &'a str) -> ArrayKey<'a> {
        match parse_numeric_key(value) {
            Some(index) => ArrayKey::Long(index),
            None => ArrayKey::Str(value),
        }
    }
}

impl<'a> From<i32> for ArrayKey<'a> {
    fn from(index: i32) -> ArrayKey<'a> {
        ArrayKey::Long(i64::from(index))
    }
}

impl<'a> From<i64> for ArrayKey<'a> {
    fn from(index: i64) -> ArrayKey<'a> {
        ArrayKey::Long(index)
    }
}

impl<'a> From<u64> for ArrayKey<'a> {
    fn from(index: u64) -> ArrayKey<'a> {
        if let Ok(index) = i64::try_from(index) {
            ArrayKey::Long(index)
        } else {
            ArrayKey::String(index.to_string())
        }
    }
}

impl<'a> From<usize> for ArrayKey<'a> {
    fn from(index: usize) -> ArrayKey<'a> {
        if let Ok(index) = i64::try_from(index) {
            ArrayKey::Long(index)
        } else {
            ArrayKey::String(index.to_string())
        }
    }
}

impl<'a> From<&'a ZBox<ZendStr>> for ArrayKey<'a> {
    fn from(value: &'a ZBox<ZendStr>) -> Self {
        ArrayKey::from(value.as_ref())
    }
}

impl<'a> From<&'a ZendStr> for ArrayKey<'a> {
    fn from(value: &'a ZendStr) -> Self {
        if let Ok(text) = value.as_str()
            && let Some(index) = parse_numeric_key(text)
        {
            return ArrayKey::Long(index);
        }
        ArrayKey::ZendString(value)
    }
}

impl<'a> FromZval<'a> for ArrayKey<'_> {
    const TYPE: DataType = DataType::String;

    fn from_zval(zval: &'a Zval) -> Option<Self> {
        if let Some(key) = zval.long() {
            return Some(ArrayKey::Long(key));
        }
        if let Some(key) = zval.string() {
            return Some(ArrayKey::String(key));
        }
        None
    }
}

#[cfg(test)]
#[cfg(feature = "embed")]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::error::Error;
    use crate::types::ArrayKey;

    #[test]
    fn test_string_try_from_array_key() {
        let key = ArrayKey::String("test".to_string());
        let result: crate::error::Result<String, _> = key.try_into();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test".to_string());

        let key = ArrayKey::Str("test");
        let result: crate::error::Result<String, _> = key.try_into();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test".to_string());

        let key = ArrayKey::Long(42);
        let result: crate::error::Result<String, _> = key.try_into();
        assert_eq!(result.unwrap(), "42".to_string());

        let key = ArrayKey::String("42".to_string());
        let result: crate::error::Result<String, _> = key.try_into();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "42".to_string());

        let key = ArrayKey::Str("123");
        let result: crate::error::Result<i64, _> = key.try_into();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 123);
    }

    #[test]
    fn test_i64_try_from_array_key() {
        let key = ArrayKey::Long(42);
        let result: crate::error::Result<i64, _> = key.try_into();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);

        let key = ArrayKey::String("42".to_string());
        let result: crate::error::Result<i64, _> = key.try_into();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);

        let key = ArrayKey::Str("123");
        let result: crate::error::Result<i64, _> = key.try_into();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 123);

        let key = ArrayKey::String("not a number".to_string());
        let result: crate::error::Result<i64, _> = key.try_into();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::ZvalConversion(_)));

        for key in ["+1", "01", "-0", "-01"] {
            let result: crate::error::Result<i64, _> = ArrayKey::Str(key).try_into();
            assert!(
                matches!(result, Err(Error::ZvalConversion(_))),
                "{key:?} should not convert to i64"
            );
        }
    }

    #[test]
    fn test_from_str_with_leading_zeros() {
        let key: ArrayKey = "00".into();
        assert_eq!(key, ArrayKey::Str("00"));
        let key: ArrayKey = "071".into();
        assert_eq!(key, ArrayKey::Str("071"));
        let key: ArrayKey = "0".into();
        assert_eq!(key, ArrayKey::Long(0));
    }

    #[test]
    fn test_from_str_matches_php_numeric_keys() {
        for (key, expected) in [
            ("1", 1),
            ("-1", -1),
            ("0", 0),
            ("9223372036854775807", i64::MAX),
            ("-9223372036854775808", i64::MIN),
        ] {
            assert_eq!(ArrayKey::from(key), ArrayKey::Long(expected), "{key:?}");
            assert_eq!(
                ArrayKey::from(key.to_string()),
                ArrayKey::Long(expected),
                "{key:?}"
            );
        }

        for key in [
            "+1",
            "+0",
            "-0",
            "-01",
            "-",
            "",
            " 1",
            "1 ",
            "9223372036854775808",
            "-9223372036854775809",
        ] {
            assert_eq!(ArrayKey::from(key), ArrayKey::Str(key), "{key:?}");
            assert_eq!(
                ArrayKey::from(key.to_string()),
                ArrayKey::String(key.to_string()),
                "{key:?}"
            );
        }
    }

    #[test]
    fn test_from_string_with_leading_zeros() {
        let key = ArrayKey::String("042".to_string());
        let result: crate::error::Result<String, _> = key.try_into();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "042");
        let key = ArrayKey::String("00".to_string());
        let result: crate::error::Result<String, _> = key.try_into();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "00");
        let key = ArrayKey::String("0".to_string());
        let result: crate::error::Result<i64, _> = key.try_into();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }
}
