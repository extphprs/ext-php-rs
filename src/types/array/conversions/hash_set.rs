use super::super::ZendHashTable;
use crate::{
    boxed::ZBox,
    convert::{FromZval, IntoZval},
    error::{Error, Result},
    flags::DataType,
    types::Zval,
};
use std::collections::HashSet;
use std::convert::TryFrom;
use std::hash::{BuildHasher, Hash};

impl<'a, V, H> TryFrom<&'a ZendHashTable> for HashSet<V, H>
where
    V: FromZval<'a> + Eq + Hash,
    H: BuildHasher + Default,
{
    type Error = Error;

    fn try_from(value: &'a ZendHashTable) -> Result<Self> {
        let mut set = Self::with_capacity_and_hasher(value.len(), H::default());

        for (_key, val) in value {
            set.insert(V::from_zval(val).ok_or_else(|| Error::ZvalConversion(val.get_type()))?);
        }

        Ok(set)
    }
}

impl<V, H> TryFrom<HashSet<V, H>> for ZBox<ZendHashTable>
where
    V: IntoZval,
    H: BuildHasher,
{
    type Error = Error;

    fn try_from(value: HashSet<V, H>) -> Result<Self> {
        let mut ht = ZendHashTable::with_capacity(
            value.len().try_into().map_err(|_| Error::IntegerOverflow)?,
        );

        for (k, v) in value.into_iter().enumerate() {
            ht.insert(k, v)?;
        }

        Ok(ht)
    }
}

impl<V, H> IntoZval for HashSet<V, H>
where
    V: IntoZval,
    H: BuildHasher,
{
    const TYPE: DataType = DataType::Array;
    const NULLABLE: bool = false;

    fn set_zval(self, zv: &mut Zval, _: bool) -> Result<()> {
        let arr = self.try_into()?;
        zv.set_hashtable(arr);
        Ok(())
    }
}
