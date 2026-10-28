//! Types defined by the Zend engine used in PHP.
//!
//! Generally, it is easier to work directly with Rust types, converting into
//! these PHP types when required.

pub mod array;
mod callable;
mod class_object;
mod iterable;
mod iterator;
mod long;
mod object;
mod payload;
mod php_ref;
mod separated;
mod string;
mod zval;

pub use array::{ArrayKey, Entry, OccupiedEntry, VacantEntry, ZendEmptyArray, ZendHashTable};
pub use callable::ZendCallable;
pub use class_object::ZendClassObject;
pub use iterable::Iterable;
pub use iterator::ZendIterator;
pub use long::ZendLong;
pub use object::{PropertyQuery, ZendObject};
pub use payload::{BorrowError, ClassRef, ClassRefMut};
pub use php_ref::PhpRef;
pub use separated::Separated;
pub use string::ZendStr;
pub use zval::Zval;

use crate::{
    convert::FromZval,
    error::{Error, Result},
    flags::DataType,
};

into_zval!(f32, set_double, Double);
into_zval!(f64, set_double, Double);
into_zval!(bool, set_bool, Bool);

try_from_zval!(bool, bool, Bool);

impl FromZval<'_> for f64 {
    const TYPE: DataType = DataType::Double;

    #[allow(
        clippy::cast_precision_loss,
        reason = "PHP casts an int given for a float parameter the same way"
    )]
    fn from_zval(zval: &Zval) -> Option<Self> {
        zval.double().or_else(|| zval.long().map(|v| v as f64))
    }
}

impl TryFrom<Zval> for f64 {
    type Error = Error;

    fn try_from(value: Zval) -> Result<Self> {
        Self::from_zval(&value).ok_or(Error::ZvalConversion(value.get_type()))
    }
}

impl FromZval<'_> for f32 {
    const TYPE: DataType = DataType::Double;

    fn from_zval(zval: &Zval) -> Option<Self> {
        #[allow(clippy::cast_possible_truncation)]
        f64::from_zval(zval).map(|v| v as f32)
    }
}

#[cfg(test)]
#[cfg(feature = "embed")]
mod tests {
    use super::*;

    #[test]
    fn floats_accept_a_php_int() {
        assert_eq!(f64::from_zval(&Zval::from(2)), Some(2.0));
        assert_eq!(f32::from_zval(&Zval::from(2)), Some(2.0));
    }
}
