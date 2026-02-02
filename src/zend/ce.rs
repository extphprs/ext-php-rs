//! Stock class entries registered with PHP, primarily exceptions.

#![allow(clippy::unwrap_used)]

use crate::ffi::{
    php_json_serializable_ce, zend_ce_aggregate, zend_ce_argument_count_error,
    zend_ce_arithmetic_error, zend_ce_arrayaccess, zend_ce_compile_error, zend_ce_countable,
    zend_ce_division_by_zero_error, zend_ce_error_exception, zend_ce_exception, zend_ce_iterator,
    zend_ce_parse_error, zend_ce_serializable, zend_ce_stringable, zend_ce_throwable,
    zend_ce_traversable, zend_ce_type_error, zend_ce_unhandled_match_error, zend_ce_value_error,
    zend_standard_class_def,
};

use super::ClassEntry;

/// Returns the base [`stdClass`](https://www.php.net/manual/en/class.stdclass.php) class.
///
/// # Panics
///
/// If stdclass [`ClassEntry`] is not available
pub fn stdclass() -> &'static ClassEntry {
    unsafe { zend_standard_class_def.as_ref() }.unwrap()
}

/// Returns the base [`Throwable`](https://www.php.net/manual/en/class.throwable.php) class.
///
/// # Panics
///
/// If throwable [`ClassEntry`] is not available
pub fn throwable() -> &'static ClassEntry {
    unsafe { zend_ce_throwable.as_ref() }.unwrap()
}

/// Returns the base [`Exception`](https://www.php.net/manual/en/class.exception.php) class.
///
/// # Panics
///
/// If exception [`ClassEntry`] is not available
pub fn exception() -> &'static ClassEntry {
    unsafe { zend_ce_exception.as_ref() }.unwrap()
}

/// Returns the base [`ErrorException`](https://www.php.net/manual/en/class.errorexception.php) class.
///
/// # Panics
///
/// If error exception [`ClassEntry`] is not available
pub fn error_exception() -> &'static ClassEntry {
    unsafe { zend_ce_error_exception.as_ref() }.unwrap()
}

/// Returns the base [`CompileError`](https://www.php.net/manual/en/class.compileerror.php) class.
///
/// # Panics
///
/// If compile error [`ClassEntry`] is not available
pub fn compile_error() -> &'static ClassEntry {
    unsafe { zend_ce_compile_error.as_ref() }.unwrap()
}

/// Returns the base [`ParseError`](https://www.php.net/manual/en/class.parseerror.php) class.
///
/// # Panics
///
/// If parse error [`ClassEntry`] is not available
pub fn parse_error() -> &'static ClassEntry {
    unsafe { zend_ce_parse_error.as_ref() }.unwrap()
}

/// Returns the base [`TypeError`](https://www.php.net/manual/en/class.typeerror.php) class.
///
/// # Panics
///
/// If type error [`ClassEntry`] is not available
pub fn type_error() -> &'static ClassEntry {
    unsafe { zend_ce_type_error.as_ref() }.unwrap()
}

/// Returns the base [`ArgumentCountError`](https://www.php.net/manual/en/class.argumentcounterror.php) class.
///
/// # Panics
///
/// If argument count error [`ClassEntry`] is not available
pub fn argument_count_error() -> &'static ClassEntry {
    unsafe { zend_ce_argument_count_error.as_ref() }.unwrap()
}

/// Returns the base [`ValueError`](https://www.php.net/manual/en/class.valueerror.php) class.
///
/// # Panics
///
/// If value error [`ClassEntry`] is not available
pub fn value_error() -> &'static ClassEntry {
    unsafe { zend_ce_value_error.as_ref() }.unwrap()
}

/// Returns the base [`ArithmeticError`](https://www.php.net/manual/en/class.arithmeticerror.php) class.
///
/// # Panics
///
/// If arithmetic error [`ClassEntry`] is not available
pub fn arithmetic_error() -> &'static ClassEntry {
    unsafe { zend_ce_arithmetic_error.as_ref() }.unwrap()
}

/// Returns the base [`DivisionByZeroError`](https://www.php.net/manual/en/class.divisionbyzeroerror.php) class.
///
/// # Panics
///
/// If division by zero error [`ClassEntry`] is not available
pub fn division_by_zero_error() -> &'static ClassEntry {
    unsafe { zend_ce_division_by_zero_error.as_ref() }.unwrap()
}

/// Returns the base [`UnhandledMatchError`](https://www.php.net/manual/en/class.unhandledmatcherror.php) class.
///
/// # Panics
///
/// If unhandled match error [`ClassEntry`] is not available
pub fn unhandled_match_error() -> &'static ClassEntry {
    unsafe { zend_ce_unhandled_match_error.as_ref() }.unwrap()
}

/// Returns the [`Traversable`](https://www.php.net/manual/en/class.traversable.php) interface.
///
/// # Panics
///
/// If traversable [`ClassEntry`] is not available
pub fn traversable() -> &'static ClassEntry {
    unsafe { zend_ce_traversable.as_ref() }.unwrap()
}

/// Returns the [`IteratorAggregate`](https://www.php.net/manual/en/class.iteratoraggregate.php) interface.
///
/// # Panics
///
/// If aggregate [`ClassEntry`] is not available
pub fn aggregate() -> &'static ClassEntry {
    unsafe { zend_ce_aggregate.as_ref() }.unwrap()
}

/// Returns the [`Iterator`](https://www.php.net/manual/en/class.iterator.php) interface.
///
/// # Panics
///
/// If iterator [`ClassEntry`] is not available
pub fn iterator() -> &'static ClassEntry {
    unsafe { zend_ce_iterator.as_ref() }.unwrap()
}

/// Returns the [`ArrayAccess`](https://www.php.net/manual/en/class.arrayaccess.php) interface.
///
/// # Panics
///
/// If arrayaccess [`ClassEntry`] is not available
pub fn arrayaccess() -> &'static ClassEntry {
    unsafe { zend_ce_arrayaccess.as_ref() }.unwrap()
}

/// Returns the [`Serializable`](https://www.php.net/manual/en/class.serializable.php) interface.
///
/// # Panics
///
/// If serializable [`ClassEntry`] is not available
pub fn serializable() -> &'static ClassEntry {
    unsafe { zend_ce_serializable.as_ref() }.unwrap()
}

/// Returns the [`Countable`](https://www.php.net/manual/en/class.countable.php) interface.
///
/// # Panics
///
/// If countable [`ClassEntry`] is not available
pub fn countable() -> &'static ClassEntry {
    unsafe { zend_ce_countable.as_ref() }.unwrap()
}

/// Returns the [`Stringable`](https://www.php.net/manual/en/class.stringable.php) interface.
///
/// # Panics
///
/// If stringable [`ClassEntry`] is not available
pub fn stringable() -> &'static ClassEntry {
    unsafe { zend_ce_stringable.as_ref() }.unwrap()
}

/// Returns the [`JsonSerializable`](https://www.php.net/manual/en/class.jsonserializable.php) interface.
///
/// # Panics
///
/// If jsonserializable [`ClassEntry`] is not available
pub fn jsonserializable() -> &'static ClassEntry {
    unsafe { php_json_serializable_ce.as_ref() }.unwrap()
}

#[cfg(test)]
#[cfg(feature = "embed")]
mod tests {
    use super::*;
    use crate::embed::Embed;

    #[test]
    fn test_stdclass() {
        Embed::run(|| {
            let stdclass = stdclass();
            assert_eq!(stdclass.name(), Some("stdClass"));
        });
    }

    #[test]
    fn test_throwable() {
        Embed::run(|| {
            let throwable = throwable();
            assert_eq!(throwable.name(), Some("Throwable"));
        });
    }

    #[test]
    fn test_exception() {
        Embed::run(|| {
            let exception = exception();
            assert_eq!(exception.name(), Some("Exception"));
        });
    }

    #[test]
    fn test_error_exception() {
        Embed::run(|| {
            let error_exception = error_exception();
            assert_eq!(error_exception.name(), Some("ErrorException"));
        });
    }

    #[test]
    fn test_compile_error() {
        Embed::run(|| {
            let compile_error = compile_error();
            assert_eq!(compile_error.name(), Some("CompileError"));
        });
    }

    #[test]
    fn test_parse_error() {
        Embed::run(|| {
            let parse_error = parse_error();
            assert_eq!(parse_error.name(), Some("ParseError"));
        });
    }

    #[test]
    fn test_type_error() {
        Embed::run(|| {
            let type_error = type_error();
            assert_eq!(type_error.name(), Some("TypeError"));
        });
    }

    #[test]
    fn test_argument_count_error() {
        Embed::run(|| {
            let argument_count_error = argument_count_error();
            assert_eq!(argument_count_error.name(), Some("ArgumentCountError"));
        });
    }

    #[test]
    fn test_value_error() {
        Embed::run(|| {
            let value_error = value_error();
            assert_eq!(value_error.name(), Some("ValueError"));
        });
    }

    #[test]
    fn test_arithmetic_error() {
        Embed::run(|| {
            let arithmetic_error = arithmetic_error();
            assert_eq!(arithmetic_error.name(), Some("ArithmeticError"));
        });
    }

    #[test]
    fn test_division_by_zero_error() {
        Embed::run(|| {
            let division_by_zero_error = division_by_zero_error();
            assert_eq!(division_by_zero_error.name(), Some("DivisionByZeroError"));
        });
    }

    #[test]
    fn test_unhandled_match_error() {
        Embed::run(|| {
            let unhandled_match_error = unhandled_match_error();
            assert_eq!(unhandled_match_error.name(), Some("UnhandledMatchError"));
        });
    }

    #[test]
    fn test_traversable() {
        Embed::run(|| {
            let traversable = traversable();
            assert_eq!(traversable.name(), Some("Traversable"));
        });
    }

    #[test]
    fn test_aggregate() {
        Embed::run(|| {
            let aggregate = aggregate();
            assert_eq!(aggregate.name(), Some("IteratorAggregate"));
        });
    }

    #[test]
    fn test_iterator() {
        Embed::run(|| {
            let iterator = iterator();
            assert_eq!(iterator.name(), Some("Iterator"));
        });
    }

    #[test]
    fn test_arrayaccess() {
        Embed::run(|| {
            let arrayaccess = arrayaccess();
            assert_eq!(arrayaccess.name(), Some("ArrayAccess"));
        });
    }

    #[test]
    fn test_serializable() {
        Embed::run(|| {
            let serializable = serializable();
            assert_eq!(serializable.name(), Some("Serializable"));
        });
    }

    #[test]
    fn test_countable() {
        Embed::run(|| {
            let countable = countable();
            assert_eq!(countable.name(), Some("Countable"));
        });
    }

    #[test]
    fn test_stringable() {
        Embed::run(|| {
            let stringable = stringable();
            assert_eq!(stringable.name(), Some("Stringable"));
        });
    }

    #[test]
    fn test_jsonserializable() {
        Embed::run(|| {
            let jsonserializable = jsonserializable();
            assert_eq!(jsonserializable.name(), Some("JsonSerializable"));
        });
    }
}
