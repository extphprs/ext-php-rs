//! Represents an integer introduced in PHP. Note that the size of this integer
//! differs. On a 32-bit system, a [`ZendLong`] is 32-bits, while on a 64-bit
//! system it is 64-bits.

use crate::{
    convert::IntoZval,
    error::{Error, Result},
    ffi::zend_long,
    flags::DataType,
    types::Zval,
};

use std::convert::{TryFrom, TryInto};

/// A PHP long.
///
/// The type size depends on the system architecture. On 32-bit systems, it is
/// 32-bits, while on a 64-bit system, it is 64-bits.
pub type ZendLong = zend_long;

into_zval!(i8, set_long, Long);
into_zval!(i16, set_long, Long);
into_zval!(i32, set_long, Long);

into_zval!(u8, set_long, Long);
into_zval!(u16, set_long, Long);

macro_rules! try_into_zval_int {
    ($type: ty) => {
        impl TryFrom<$type> for Zval {
            type Error = Error;

            fn try_from(val: $type) -> Result<Self> {
                let mut zv = Self::new();
                let val: ZendLong = val.try_into().map_err(|_| Error::IntegerOverflow)?;
                zv.set_long(val);
                Ok(zv)
            }
        }

        impl IntoZval for $type {
            const TYPE: DataType = DataType::Long;
            const NULLABLE: bool = false;

            fn set_zval(self, zv: &mut Zval, _: bool) -> Result<()> {
                let val: ZendLong = self.try_into().map_err(|_| Error::IntegerOverflow)?;
                zv.set_long(val);
                Ok(())
            }
        }
    };
}

try_into_zval_int!(i64);
try_into_zval_int!(u32);
try_into_zval_int!(u64);

try_into_zval_int!(isize);
try_into_zval_int!(usize);

try_from_zval!(i8, long, Long);
try_from_zval!(i16, long, Long);
try_from_zval!(i32, long, Long);
try_from_zval!(i64, long, Long);

try_from_zval!(u8, long, Long);
try_from_zval!(u16, long, Long);
try_from_zval!(u32, long, Long);
try_from_zval!(u64, long, Long);

try_from_zval!(usize, long, Long);
try_from_zval!(isize, long, Long);
