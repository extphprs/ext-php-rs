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
    /// Property name as seen from PHP (camelCase after conversion). The
    /// mangled form used by `get_properties` is computed once at runtime by
    /// [`ClassMetadata::mangled_names`](crate::class::ClassMetadata::mangled_names).
    pub name: &'static str,
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

// 64-bit: 80 bytes, 32-bit: ~48 bytes.
// Bound: 12 pointer-sized words = 96 on 64-bit, 48 on 32-bit.
const _: () = assert!(
    std::mem::size_of::<PropertyDescriptor<()>>() <= 12 * std::mem::size_of::<usize>(),
    "PropertyDescriptor grew beyond expected size"
);
