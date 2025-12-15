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

impl<'a, V> FromZval<'a> for BTreeSet<V>
where
    V: FromZval<'a> + Ord,
{
    const TYPE: DataType = DataType::Array;

    fn from_zval(zval: &'a Zval) -> Option<Self> {
        zval.array().and_then(|arr| arr.try_into().ok())
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

#[cfg(test)]
#[cfg(feature = "embed")]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::BTreeSet;

    use crate::boxed::ZBox;
    use crate::convert::FromZval;
    use crate::embed::Embed;
    use crate::types::{ZendHashTable, Zval};

    #[test]
    fn test_hash_table_try_from_btree_set() {
        Embed::run(|| {
            let mut set = BTreeSet::new();
            set.insert("one");
            let ht: ZBox<ZendHashTable> = set.try_into().unwrap();
            assert_eq!(ht.len(), 1);
            assert!(ht.get(0).is_some());
        });
    }

    #[test]
    fn test_btree_set_try_from_hash_table() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();
            ht.insert(0, "value1").unwrap();
            ht.insert(1, "value2").unwrap();
            ht.insert(2, "value3").unwrap();
            let mut zval = Zval::new();
            zval.set_hashtable(ht);

            let map = BTreeSet::<String>::from_zval(&zval).unwrap();
            assert_eq!(map.len(), 3);
            let mut it = map.iter();
            assert_eq!(it.next().unwrap(), "value1");
            assert_eq!(it.next().unwrap(), "value2");
            assert_eq!(it.next().unwrap(), "value3");
            assert_eq!(it.next(), None);
        });
    }
}
