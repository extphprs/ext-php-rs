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

impl<'a, V, H> FromZval<'a> for HashSet<V, H>
where
    V: FromZval<'a> + Eq + Hash,
    H: BuildHasher + Default,
{
    const TYPE: DataType = DataType::Array;

    fn from_zval(zval: &'a Zval) -> Option<Self> {
        zval.array().and_then(|arr| arr.try_into().ok())
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

#[cfg(test)]
#[cfg(feature = "embed")]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::HashSet;

    use crate::boxed::ZBox;
    use crate::convert::FromZval;
    use crate::embed::Embed;
    use crate::types::{ZendHashTable, Zval};

    #[test]
    fn test_hash_table_try_from_hash_set() {
        Embed::run(|| {
            let mut set = HashSet::new();
            set.insert("one");
            let ht: ZBox<ZendHashTable> = set.try_into().unwrap();
            assert_eq!(ht.len(), 1);
            assert!(ht.get(0).is_some());
        });
    }

    #[test]
    fn test_hash_set_try_from_hash_table() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();
            ht.insert(0, "value1").unwrap();
            ht.insert(1, "value2").unwrap();
            ht.insert(2, "value3").unwrap();
            let mut zval = Zval::new();
            zval.set_hashtable(ht);

            let map = HashSet::<String>::from_zval(&zval).unwrap();
            assert_eq!(map.len(), 3);
            assert!(map.contains("value1"));
            assert!(map.contains("value2"));
            assert!(map.contains("value3"));
        });
    }
}
