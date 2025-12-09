use super::super::ZendHashTable;
use crate::{
    boxed::ZBox,
    convert::{FromZval, IntoZval},
    error::{Error, Result},
    flags::DataType,
    types::Zval,
};
use std::collections::BTreeSet;
use std::convert::TryFrom;

impl<'a, V> TryFrom<&'a ZendHashTable> for BTreeSet<V>
where
    V: FromZval<'a> + Ord,
{
    type Error = Error;

    fn try_from(value: &'a ZendHashTable) -> Result<Self> {
        let mut set = Self::new();

        for (_key, val) in value {
            set.insert(V::from_zval(val).ok_or_else(|| Error::ZvalConversion(val.get_type()))?);
        }

        Ok(set)
    }
}

impl<V> TryFrom<BTreeSet<V>> for ZBox<ZendHashTable>
where
    V: IntoZval,
{
    type Error = Error;

    fn try_from(value: BTreeSet<V>) -> Result<Self> {
        let mut set = ZendHashTable::with_capacity(
            value.len().try_into().map_err(|_| Error::IntegerOverflow)?,
        );

        for (k, v) in value.into_iter().enumerate() {
            set.insert(k, v)?;
        }

        Ok(set)
    }
}

impl<V> IntoZval for BTreeSet<V>
where
    V: IntoZval,
{
    const TYPE: DataType = DataType::Array;
    const NULLABLE: bool = false;

    fn set_zval(self, zv: &mut Zval, _: bool) -> Result<()> {
        let arr = self.try_into()?;
        zv.set_hashtable(arr);
        Ok(())
    }
}
