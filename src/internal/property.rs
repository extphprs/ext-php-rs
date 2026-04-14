use crate::{
    describe::DocComments, exception::PhpResult, flags::DataType, flags::PropertyFlags, types::Zval,
};

/// Describes a property on a PHP class backed by Rust.
///
/// Stores function pointers for get/set operations instead of boxed closures.
/// Fully const-constructible: lives in `static` items with zero heap allocation.
///
/// # Type Parameters
///
/// * `T` - The Rust struct type that owns this property.
pub struct PropertyDescriptor<T: 'static> {
    /// Property name as seen from PHP (camelCase after conversion).
    pub name: &'static str,
    /// PHP-convention mangled name for `get_properties` output.
    ///
    /// For field properties this is a compile-time literal emitted by the proc
    /// macro (`"\0ClassName\0prop"` for private, `"\0*\0prop"` for protected,
    /// or the bare name for public). For method properties the macro sets this
    /// to the unmangled `name`; the real mangled form is provided at runtime by
    /// [`ClassMetadata::method_mangled_names`](crate::class::ClassMetadata::method_mangled_names).
    pub mangled_name: &'static str,
    /// Getter function. Takes `&T` (immutable) and writes into the Zval.
    /// `None` means the property is write-only.
    pub get: Option<fn(&T, &mut Zval) -> PhpResult>,
    /// Setter function. Takes `&mut T` and reads from the Zval.
    /// `None` means the property is read-only.
    pub set: Option<fn(&mut T, &Zval) -> PhpResult>,
    /// Visibility flags (Public, Protected, Private).
    pub flags: PropertyFlags,
    /// Doc comments from the Rust source.
    pub docs: DocComments,
    /// The PHP data type of this property.
    pub ty: DataType,
    /// Whether the property is nullable.
    pub nullable: bool,
    /// Whether the property is read-only.
    pub readonly: bool,
}

// 64-bit: 96 bytes, 32-bit: ~56 bytes.
// Bound: 12 pointer-sized words = 96 on 64-bit, 48 on 32-bit.
const _: () = assert!(
    std::mem::size_of::<PropertyDescriptor<()>>() <= 12 * std::mem::size_of::<usize>(),
    "PropertyDescriptor grew beyond expected size"
);
