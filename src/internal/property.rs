use crate::{describe::DocComments, flags::DataType, flags::PropertyFlags, props::Property};

pub struct PropertyInfo<'a, T> {
    pub prop: Property<'a, T>,
    pub flags: PropertyFlags,
    pub docs: DocComments,
    pub ty: DataType,
    pub nullable: bool,
    pub readonly: bool,
}
