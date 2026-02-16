use crate::{
    describe::DocComments,
    flags::{DataType, PropertyFlags},
    props::Property,
};

pub struct PropertyInfo<'a, T> {
    pub prop: Property<'a, T>,
    pub flags: PropertyFlags,
    pub docs: DocComments,
    /// The PHP type of the property (for stub generation).
    pub ty: Option<DataType>,
    /// Whether the property is nullable (for stub generation).
    pub nullable: bool,
    /// Default value as a PHP-compatible string (for stub generation).
    pub default: Option<&'static str>,
}
