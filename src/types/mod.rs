//! Types defined by the Zend engine used in PHP.
//!
//! Generally, it is easier to work directly with Rust types, converting into
//! these PHP types when required.

pub mod array;
mod callable;
pub mod callable_channel;
mod class_object;
mod iterable;
mod iterator;
mod long;
mod object;
mod string;
mod zval;

pub use array::{ArrayKey, Entry, OccupiedEntry, VacantEntry, ZendEmptyArray, ZendHashTable};
pub use callable::ZendCallable;
pub use callable_channel::{
    CallableChannel, CallableHandle, CallableRequest, CallableResponse, CallableTarget, ClosureId,
    ClosureRegistry, SerializedValue,
};
pub use class_object::ZendClassObject;
pub use iterable::Iterable;
pub use iterator::ZendIterator;
pub use long::ZendLong;
pub use object::{PropertyQuery, ZendObject};
pub use string::ZendStr;
pub use zval::Zval;

use crate::{convert::FromZval, flags::DataType};

into_zval!(f32, set_double, Double);
into_zval!(f64, set_double, Double);
into_zval!(bool, set_bool, Bool);

try_from_zval!(f64, double, Double);
try_from_zval!(bool, bool, Bool);

impl FromZval<'_> for f32 {
    const TYPE: DataType = DataType::Double;

    fn from_zval(zval: &Zval) -> Option<Self> {
        #[allow(clippy::cast_possible_truncation)]
        zval.double().map(|v| v as f32)
    }
}
