//! This module defines the `PhpEnum` trait and related types for Rust enums
//! that are exported to PHP.
use std::ptr;

use crate::{
    boxed::ZBox,
    class::RegisteredClass,
    convert::{FromZendObject, FromZval, IntoZendObject, IntoZval},
    describe::DocComments,
    error::{Error, Result},
    ffi::zend_enum_get_case,
    flags::{ClassFlags, DataType},
    rc::PhpRc,
    types::{ZendObject, ZendStr, Zval},
};

/// Implemented on Rust enums which are exported to PHP.
pub trait RegisteredEnum {
    /// The cases of the enum.
    const CASES: &'static [EnumCase];

    /// # Errors
    ///
    /// - [`Error::InvalidProperty`] if the enum does not have a case with the
    ///   given name, an error is returned.
    fn from_name(name: &str) -> Result<Self>
    where
        Self: Sized;

    /// Returns the variant name of the enum as it is registered in PHP.
    fn to_name(&self) -> &'static str;
}

impl<T> FromZendObject<'_> for T
where
    T: RegisteredEnum,
{
    fn from_zend_object(obj: &ZendObject) -> Result<Self> {
        if !ClassFlags::from_bits_truncate(unsafe { (*obj.ce).ce_flags }).contains(ClassFlags::Enum)
        {
            return Err(Error::ZvalConversion(DataType::Object(None)));
        }

        let name = obj
            .get_properties()?
            .get("name")
            .and_then(Zval::indirect)
            .and_then(Zval::str)
            .ok_or_else(|| Error::InvalidProperty {
                property: "name".to_string(),
            })?;

        T::from_name(name)
    }
}

impl<T> FromZval<'_> for T
where
    T: RegisteredEnum + RegisteredClass,
{
    const TYPE: DataType = DataType::Object(Some(T::CLASS_NAME));

    fn from_zval(zval: &Zval) -> Option<Self> {
        zval.object()
            .and_then(|obj| Self::from_zend_object(obj).ok())
    }
}

impl<T> IntoZendObject for T
where
    T: RegisteredEnum + RegisteredClass,
{
    fn into_zend_object(self) -> Result<ZBox<ZendObject>> {
        let case = zend_case(&self);
        // SAFETY: `zend_case` yields a valid object for the rest of the request.
        // The class constant table keeps its own reference, so the box takes a
        // reference of its own before assuming ownership of the pointer.
        unsafe {
            (*case).inc_count();
            Ok(ZBox::from_raw(case))
        }
    }
}

impl<T> IntoZval for T
where
    T: RegisteredEnum + RegisteredClass,
{
    const TYPE: DataType = DataType::Object(Some(T::CLASS_NAME));
    const NULLABLE: bool = false;

    fn set_zval(self, zv: &mut Zval, _persistent: bool) -> Result<()> {
        let case = zend_case(&self);
        // SAFETY: `zend_case` yields a valid object for the rest of the request
        // and `set_object` takes its own reference, so the class constant table
        // keeps the one it already holds.
        zv.set_object(unsafe { &mut *case });
        Ok(())
    }
}

/// Resolves the PHP object backing an enum case.
///
/// The object is owned by the class constant table and stays alive until the
/// end of the request. The pointer is borrowed: whoever stores it must take a
/// reference of its own with [`PhpRc::inc_count`].
fn zend_case<T: RegisteredEnum + RegisteredClass>(case: &T) -> *mut ZendObject {
    let mut name = ZendStr::new(T::to_name(case), false);
    // SAFETY: the class entry is registered and `to_name` only produces names
    // declared as cases of this enum, which `zend_enum_get_case` requires.
    unsafe {
        zend_enum_get_case(
            ptr::from_ref(T::get_metadata().ce()).cast_mut(),
            &raw mut *name,
        )
    }
}

/// Represents a case in a PHP enum.
pub struct EnumCase {
    /// The identifier of the enum case, e.g. `Bar` in `enum Foo { Bar }`.
    pub name: &'static str,
    /// The value of the enum case, which can be an integer or a string.
    pub discriminant: Option<Discriminant>,
    /// The documentation comments for the enum case.
    pub docs: DocComments,
}

impl EnumCase {
    /// Gets the PHP data type of the enum case's discriminant.
    #[must_use]
    pub fn data_type(&self) -> DataType {
        match self.discriminant {
            Some(Discriminant::Int(_)) => DataType::Long,
            Some(Discriminant::String(_)) => DataType::String,
            None => DataType::Undef,
        }
    }
}

/// Represents the discriminant of an enum case in PHP, which can be either an
/// integer or a string.
#[derive(Debug, PartialEq, Eq)]
pub enum Discriminant {
    /// An integer discriminant.
    Int(i64),
    /// A string discriminant.
    String(&'static str),
}

impl TryFrom<&Discriminant> for Zval {
    type Error = Error;

    fn try_from(value: &Discriminant) -> Result<Self> {
        match value {
            Discriminant::Int(i) => i.into_zval(false),
            Discriminant::String(s) => s.into_zval(false),
        }
    }
}

#[cfg(test)]
#[cfg(feature = "embed")]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::embed::Embed;

    #[test]
    fn test_zval_try_from_discriminant() {
        Embed::run(|| {
            let zval_int: Zval = Zval::try_from(&Discriminant::Int(42)).unwrap();
            assert!(zval_int.is_long());
            assert_eq!(zval_int.long().unwrap(), 42);

            let zval_str: Zval = Zval::try_from(&Discriminant::String("foo")).unwrap();
            assert!(zval_str.is_string());
            assert_eq!(zval_str.string().unwrap().clone(), "foo");
        });
    }
}
