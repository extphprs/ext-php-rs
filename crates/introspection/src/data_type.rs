use std::fmt::Display;

use crate::abi::{Str, assert_ffi_safe};

/// Valid data types for PHP.
///
/// The Zend integer representation of each variant depends on the PHP version
/// and lives in `ext-php-rs`, behind bindgen. This crate only carries the
/// ABI-stable enum so `cargo-php` can read it from a compiled extension.
#[repr(C, u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DataType {
    /// Undefined
    Undef,
    /// `null`
    Null,
    /// `false`
    False,
    /// `true`
    True,
    /// Integer (the irony)
    Long,
    /// Floating point number
    Double,
    /// String
    String,
    /// Array
    Array,
    /// Iterable
    Iterable,
    /// Object, with an optional class or interface name. An empty name means
    /// any object. Use [`DataType::object`] or [`DataType::ANY_OBJECT`] to
    /// build it and [`DataType::class_name`] to read it.
    Object(Str),
    /// Resource
    Resource,
    /// Reference
    Reference,
    /// Callable
    Callable,
    /// Constant expression
    ConstantExpression,
    /// Void
    #[default]
    Void,
    /// Mixed
    Mixed,
    /// Boolean
    Bool,
    /// Pointer
    Ptr,
    /// Indirect (internal)
    Indirect,
}

assert_ffi_safe!(DataType, size = 24, align = 8);

impl DataType {
    /// Any object, whatever its class.
    pub const ANY_OBJECT: Self = Self::Object(Str::new(""));

    /// An object of the given class or interface.
    #[must_use]
    pub const fn object(class_name: &'static str) -> Self {
        Self::Object(Str::new(class_name))
    }

    /// The class name of a [`DataType::Object`], if it has one.
    #[must_use]
    pub const fn class_name(&self) -> Option<&'static str> {
        match self {
            Self::Object(name) if !name.str().is_empty() => Some(name.str()),
            _ => None,
        }
    }
}

impl Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::Undef => write!(f, "Undefined"),
            DataType::Null => write!(f, "Null"),
            DataType::False => write!(f, "False"),
            DataType::True => write!(f, "True"),
            DataType::Long => write!(f, "Long"),
            DataType::Double => write!(f, "Double"),
            DataType::String => write!(f, "String"),
            DataType::Array => write!(f, "Array"),
            DataType::Object(_) => write!(f, "{}", self.class_name().unwrap_or("Object")),
            DataType::Resource => write!(f, "Resource"),
            DataType::Reference => write!(f, "Reference"),
            DataType::Callable => write!(f, "Callable"),
            DataType::ConstantExpression => write!(f, "Constant Expression"),
            DataType::Void => write!(f, "Void"),
            DataType::Bool => write!(f, "Bool"),
            DataType::Mixed => write!(f, "Mixed"),
            DataType::Ptr => write!(f, "Pointer"),
            DataType::Indirect => write!(f, "Indirect"),
            DataType::Iterable => write!(f, "Iterable"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[repr(C)]
    struct ObjectLayout {
        tag: u8,
        name: Str,
    }

    fn layout(ty: &DataType) -> &ObjectLayout {
        unsafe { &*std::ptr::from_ref(ty).cast::<ObjectLayout>() }
    }

    #[test]
    fn object_stores_a_c_layout_tag_and_class_name() {
        let ty = DataType::object("Foo");
        let layout = layout(&ty);
        assert_eq!(layout.tag, 9);
        assert_eq!(layout.name, Str::new("Foo"));
    }

    #[test]
    fn any_object_stores_a_c_layout_none_payload() {
        let ty = DataType::ANY_OBJECT;
        let layout = layout(&ty);
        assert_eq!(layout.tag, 9);
        assert_eq!(layout.name, Str::new(""));
    }

    #[test]
    fn class_name_returns_the_class_of_an_object() {
        assert_eq!(DataType::object("Foo").class_name(), Some("Foo"));
    }

    #[test]
    fn class_name_is_none_for_any_object() {
        assert_eq!(DataType::ANY_OBJECT.class_name(), None);
    }

    #[test]
    fn class_name_is_none_for_a_scalar() {
        assert_eq!(DataType::Long.class_name(), None);
    }

    #[test]
    fn display_uses_the_class_name() {
        assert_eq!(DataType::object("Foo\\Bar").to_string(), "Foo\\Bar");
    }

    #[test]
    fn display_falls_back_to_object() {
        assert_eq!(DataType::ANY_OBJECT.to_string(), "Object");
    }

    #[test]
    fn objects_compare_by_class_name_content() {
        let owned = String::from("Foo");
        let leaked: &'static str = Box::leak(owned.into_boxed_str());
        assert_eq!(DataType::object(leaked), DataType::object("Foo"));
        assert_ne!(DataType::object("Foo"), DataType::object("Bar"));
        assert_ne!(DataType::object("Foo"), DataType::ANY_OBJECT);
    }
}
