//! `IndexSet` conversions for `ZendHashTable`.
//!
//! Unlike `HashSet`, `IndexSet` preserves insertion order, making it suitable
//! for working with PHP arrays where element order matters.

use super::super::ZendHashTable;
use crate::{
    boxed::ZBox,
    convert::{FromZval, IntoZval},
    error::{Error, Result},
    flags::DataType,
    types::Zval,
};
use indexmap::IndexSet;
use std::convert::TryFrom;
use std::hash::{BuildHasher, Hash};

impl<'a, V, H> TryFrom<&'a ZendHashTable> for IndexSet<V, H>
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

impl<V, H> TryFrom<IndexSet<V, H>> for ZBox<ZendHashTable>
where
    V: IntoZval,
    H: BuildHasher,
{
    type Error = Error;

    fn try_from(value: IndexSet<V, H>) -> Result<Self> {
        let mut ht = ZendHashTable::with_capacity(
            value.len().try_into().map_err(|_| Error::IntegerOverflow)?,
        );

        for (k, v) in value.into_iter().enumerate() {
            ht.insert(k, v)?;
        }

        Ok(ht)
    }
}

impl<'a, V, H> FromZval<'a> for IndexSet<V, H>
where
    V: FromZval<'a> + Eq + Hash,
    H: BuildHasher + Default,
{
    const TYPE: DataType = DataType::Array;

    fn from_zval(zval: &'a Zval) -> Option<Self> {
        zval.array().and_then(|arr| arr.try_into().ok())
    }
}

impl<V, H> IntoZval for IndexSet<V, H>
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
    use indexmap::IndexSet;

    use crate::boxed::ZBox;
    use crate::convert::{FromZval, IntoZval};
    use crate::embed::Embed;
    use crate::types::{ZendHashTable, Zval};

    #[test]
    fn test_hash_table_try_from_index_set() {
        Embed::run(|| {
            let mut set = IndexSet::new();
            set.insert("one");
            set.insert("two");
            set.insert("three");

            let ht: ZBox<ZendHashTable> = set.try_into().unwrap();
            assert_eq!(ht.len(), 3);
            assert_eq!(ht.get(0).unwrap().string().unwrap(), "one");
            assert_eq!(ht.get(1).unwrap().string().unwrap(), "two");
            assert_eq!(ht.get(2).unwrap().string().unwrap(), "three");
        });
    }

    #[test]
    fn test_index_set_into_zval() {
        Embed::run(|| {
            let mut set = IndexSet::new();
            set.insert("one");
            set.insert("two");
            set.insert("three");

            let zval = set.into_zval(false).unwrap();
            assert!(zval.is_array());
            let ht: &ZendHashTable = zval.array().unwrap();
            assert_eq!(ht.len(), 3);
            assert_eq!(ht.get(0).unwrap().string().unwrap(), "one");
            assert_eq!(ht.get(1).unwrap().string().unwrap(), "two");
            assert_eq!(ht.get(2).unwrap().string().unwrap(), "three");
        });
    }

    #[test]
    fn test_index_set_try_from_hash_table() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();
            ht.insert(0, "value1").unwrap();
            ht.insert(1, "value2").unwrap();
            ht.insert(2, "value3").unwrap();
            let mut zval = Zval::new();
            zval.set_hashtable(ht);

            let set = IndexSet::<String>::from_zval(&zval).unwrap();
            assert_eq!(set.len(), 3);
            assert!(set.contains("value1"));
            assert!(set.contains("value2"));
            assert!(set.contains("value3"));
        });
    }

    #[test]
    fn test_index_set_preserves_order() {
        Embed::run(|| {
            let mut set = IndexSet::new();
            set.insert("z");
            set.insert("a");
            set.insert("m");

            let ht: ZBox<ZendHashTable> = set.try_into().unwrap();

            // Verify order is preserved by iterating
            let values: Vec<_> = ht.iter().map(|(_, v)| v.string().unwrap()).collect();
            assert_eq!(values, vec!["z", "a", "m"]);

            // Convert back and verify order
            let set_back: IndexSet<String> = ht.as_ref().try_into().unwrap();
            let values_back: Vec<_> = set_back.iter().cloned().collect();
            assert_eq!(values_back, vec!["z", "a", "m"]);
        });
    }
}
