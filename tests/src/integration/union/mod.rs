//! Integration tests for union and intersection types (PHP 8.0+)

use ext_php_rs::args::Arg;
#[cfg(php82)]
use ext_php_rs::args::TypeGroup;
use ext_php_rs::builders::FunctionBuilder;
use ext_php_rs::flags::DataType;
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;
use ext_php_rs::zend::ExecuteData;

// ==== PhpUnion derive macro tests ====

/// Rust enum representing a PHP `int|string` union type.
/// Uses the `PhpUnion` derive macro to auto-generate `FromZval`/`IntoZval` and
/// union type info.
#[derive(Debug, Clone, PhpUnion)]
pub enum IntOrString {
    Int(i64),
    Str(String),
}

/// Function using `PhpUnion` enum parameter.
/// The `#[php(union_enum)]` attribute tells the macro to use
/// `PhpUnion::union_types()`.
#[php_function]
pub fn test_php_union_enum(#[php(union_enum)] value: IntOrString) -> String {
    match value {
        IntOrString::Int(n) => format!("int:{n}"),
        IntOrString::Str(s) => format!("string:{s}"),
    }
}

/// Rust enum representing a PHP `float|bool` union type.
#[derive(Debug, Clone, PhpUnion)]
pub enum FloatOrBool {
    Float(f64),
    Bool(bool),
}

/// Function using `PhpUnion` enum parameter with two different types.
#[php_function]
pub fn test_php_union_float_bool(#[php(union_enum)] value: FloatOrBool) -> String {
    match value {
        FloatOrBool::Float(f) => format!("float:{f}"),
        FloatOrBool::Bool(b) => format!("bool:{b}"),
    }
}

// Note: PhpUnion derive with interface/class types is more complex since object
// references don't implement IntoZval (which takes self by value). For object
// types, users should use the macro-based syntax `#[php(types = "...")]` instead.
// The DNF functionality is tested thoroughly via the macro-based tests below.

// ==== Macro-based union type tests ====

/// Function using macro syntax for union types
/// Accepts int|string via #[php(types = "...")]
#[php_function]
pub fn test_macro_union_int_string(#[php(types = "int|string")] _value: &Zval) -> String {
    "macro_ok".to_string()
}

/// Function using macro syntax for union type with null
/// Accepts float|bool|null
#[php_function]
pub fn test_macro_union_float_bool_null(#[php(types = "float|bool|null")] _value: &Zval) -> String {
    "macro_ok".to_string()
}

// ==== Macro-based intersection type tests (PHP 8.1+) ====

/// Function using macro syntax for intersection types
/// Accepts Countable&Traversable
#[php_function]
pub fn test_macro_intersection(#[php(types = "Countable&Traversable")] _value: &Zval) -> String {
    "macro_intersection_ok".to_string()
}

// ==== DNF (Disjunctive Normal Form) type tests (PHP 8.2+) ====

/// Function using macro syntax for DNF types
/// Accepts (Countable&Traversable)|ArrayAccess
#[cfg(php82)]
#[php_function]
pub fn test_macro_dnf(
    #[php(types = "(Countable&Traversable)|ArrayAccess")] _value: &Zval,
) -> String {
    "macro_dnf_ok".to_string()
}

/// Function using macro syntax for DNF with multiple intersection groups
/// Accepts (Countable&Traversable)|(Iterator&ArrayAccess)
#[cfg(php82)]
#[php_function]
pub fn test_macro_dnf_multi(
    #[php(types = "(Countable&Traversable)|(Iterator&ArrayAccess)")] _value: &Zval,
) -> String {
    "macro_dnf_multi_ok".to_string()
}

/// Handler for `test_union_int_string` function.
/// Accepts `int|string` and returns the type name.
#[cfg(not(windows))]
extern "C" fn test_union_int_string_handler(_: &mut ExecuteData, retval: &mut Zval) {
    // For now, just return "ok" to indicate we received the value
    // The important part is that PHP reflection sees the correct union type
    let _ = retval.set_string("ok", false);
}

#[cfg(windows)]
extern "vectorcall" fn test_union_int_string_handler(_: &mut ExecuteData, retval: &mut Zval) {
    let _ = retval.set_string("ok", false);
}

/// Handler for `test_union_int_string_null` function.
/// Accepts `int|string|null`.
#[cfg(not(windows))]
extern "C" fn test_union_int_string_null_handler(_: &mut ExecuteData, retval: &mut Zval) {
    let _ = retval.set_string("ok", false);
}

#[cfg(windows)]
extern "vectorcall" fn test_union_int_string_null_handler(_: &mut ExecuteData, retval: &mut Zval) {
    let _ = retval.set_string("ok", false);
}

/// Handler for `test_union_array_bool` function.
/// Accepts `array|bool`.
#[cfg(not(windows))]
extern "C" fn test_union_array_bool_handler(_: &mut ExecuteData, retval: &mut Zval) {
    let _ = retval.set_string("ok", false);
}

#[cfg(windows)]
extern "vectorcall" fn test_union_array_bool_handler(_: &mut ExecuteData, retval: &mut Zval) {
    let _ = retval.set_string("ok", false);
}

/// Handler for `test_intersection_countable_traversable` function.
/// Accepts `Countable&Traversable` (PHP 8.1+).
#[cfg(not(windows))]
extern "C" fn test_intersection_handler(_: &mut ExecuteData, retval: &mut Zval) {
    let _ = retval.set_string("intersection_ok", false);
}

#[cfg(windows)]
extern "vectorcall" fn test_intersection_handler(_: &mut ExecuteData, retval: &mut Zval) {
    let _ = retval.set_string("intersection_ok", false);
}

/// Handler for `test_dnf` function.
/// Accepts `(Countable&Traversable)|ArrayAccess` (PHP 8.2+).
#[cfg(all(php82, not(windows)))]
extern "C" fn test_dnf_handler(_: &mut ExecuteData, retval: &mut Zval) {
    let _ = retval.set_string("dnf_ok", false);
}

#[cfg(all(php82, windows))]
extern "vectorcall" fn test_dnf_handler(_: &mut ExecuteData, retval: &mut Zval) {
    let _ = retval.set_string("dnf_ok", false);
}

pub fn build_module(builder: ModuleBuilder) -> ModuleBuilder {
    // Function with int|string parameter
    let union_int_string =
        FunctionBuilder::new("test_union_int_string", test_union_int_string_handler)
            .arg(Arg::new_union(
                "value",
                vec![DataType::Long, DataType::String],
            ))
            .returns(DataType::String, false, false);

    // Function with int|string|null parameter
    let union_int_string_null = FunctionBuilder::new(
        "test_union_int_string_null",
        test_union_int_string_null_handler,
    )
    .arg(Arg::new_union(
        "value",
        vec![DataType::Long, DataType::String, DataType::Null],
    ))
    .returns(DataType::String, false, false);

    // Function with array|bool parameter
    let union_array_bool =
        FunctionBuilder::new("test_union_array_bool", test_union_array_bool_handler)
            .arg(Arg::new_union(
                "value",
                vec![DataType::Array, DataType::Bool],
            ))
            .returns(DataType::String, false, false);

    // Function with intersection type Countable&Traversable (PHP 8.1+)
    let intersection_countable_traversable = FunctionBuilder::new(
        "test_intersection_countable_traversable",
        test_intersection_handler,
    )
    .arg(Arg::new_intersection(
        "value",
        vec!["Countable".to_string(), "Traversable".to_string()],
    ))
    .returns(DataType::String, false, false);

    let builder = builder
        .function(union_int_string)
        .function(union_int_string_null)
        .function(union_array_bool)
        .function(wrap_function!(test_macro_union_int_string))
        .function(wrap_function!(test_macro_union_float_bool_null))
        .function(intersection_countable_traversable)
        .function(wrap_function!(test_macro_intersection))
        // PhpUnion derive macro tests
        .function(wrap_function!(test_php_union_enum))
        .function(wrap_function!(test_php_union_float_bool));

    // DNF types are PHP 8.2+ only
    #[cfg(php82)]
    let builder = {
        // Function with DNF type (Countable&Traversable)|ArrayAccess (PHP 8.2+)
        let dnf_type = FunctionBuilder::new("test_dnf", test_dnf_handler)
            .arg(Arg::new_dnf(
                "value",
                vec![
                    TypeGroup::Intersection(vec![
                        "Countable".to_string(),
                        "Traversable".to_string(),
                    ]),
                    TypeGroup::Single("ArrayAccess".to_string()),
                ],
            ))
            .returns(DataType::String, false, false);

        builder
            .function(dnf_type)
            .function(wrap_function!(test_macro_dnf))
            .function(wrap_function!(test_macro_dnf_multi))
    };

    builder
}

#[cfg(test)]
mod tests {
    #[test]
    fn union_types_work() {
        assert!(crate::integration::test::run_php("union/union.php"));
    }
}
