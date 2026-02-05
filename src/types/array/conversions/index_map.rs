//! `IndexMap` conversions for `ZendHashTable`.
//!
//! Unlike `HashMap`, `IndexMap` preserves insertion order, making it suitable
//! for working with PHP arrays where key order matters.

use super::super::ZendHashTable;
use crate::types::ArrayKey;
use crate::{
    boxed::ZBox,
    convert::{FromZval, IntoZval},
    error::{Error, Result},
    flags::DataType,
    types::Zval,
};
use indexmap::IndexMap;
use std::convert::TryFrom;
use std::hash::{BuildHasher, Hash};

impl<'a, K, V, H> TryFrom<&'a ZendHashTable> for IndexMap<K, V, H>
where
    K: TryFrom<ArrayKey<'a>, Error = Error> + Eq + Hash,
    V: FromZval<'a>,
    H: BuildHasher + Default,
{
    type Error = Error;

    fn try_from(value: &'a ZendHashTable) -> Result<Self> {
        let mut map = Self::with_capacity_and_hasher(value.len(), H::default());

        for (key, val) in value {
            map.insert(
                key.try_into()?,
                V::from_zval(val).ok_or_else(|| Error::ZvalConversion(val.get_type()))?,
            );
        }

        Ok(map)
    }
}

impl<'a, K, V, H> TryFrom<IndexMap<K, V, H>> for ZBox<ZendHashTable>
where
    K: Into<ArrayKey<'a>>,
    V: IntoZval,
    H: BuildHasher + Default,
{
    type Error = Error;

    fn try_from(value: IndexMap<K, V, H>) -> Result<Self> {
        let mut ht = ZendHashTable::with_capacity(
            value.len().try_into().map_err(|_| Error::IntegerOverflow)?,
        );

        for (k, v) in value {
            ht.insert(k.into(), v)?;
        }

        Ok(ht)
    }
}

impl<'a, K, V, H> FromZval<'a> for IndexMap<K, V, H>
where
    K: TryFrom<ArrayKey<'a>, Error = Error> + Hash + Eq,
    V: FromZval<'a>,
    H: BuildHasher + Default,
{
    const TYPE: DataType = DataType::Array;

    fn from_zval(zval: &'a Zval) -> Option<Self> {
        zval.array().and_then(|arr| arr.try_into().ok())
    }
}

impl<'a, V, H> FromZval<'a> for IndexMap<ArrayKey<'a>, V, H>
where
    V: FromZval<'a>,
    H: BuildHasher + Default,
{
    const TYPE: DataType = DataType::Array;

    fn from_zval(zval: &'a Zval) -> Option<Self> {
        zval.array().and_then(|arr| arr.try_into().ok())
    }
}

impl<'a, V, H> TryFrom<&'a ZendHashTable> for IndexMap<ArrayKey<'a>, V, H>
where
    V: FromZval<'a>,
    H: BuildHasher + Default,
{
    type Error = Error;

    fn try_from(value: &'a ZendHashTable) -> Result<Self> {
        let mut map = Self::with_capacity_and_hasher(value.len(), H::default());

        for (key, val) in value {
            map.insert(
                key,
                V::from_zval(val).ok_or_else(|| Error::ZvalConversion(val.get_type()))?,
            );
        }

        Ok(map)
    }
}

impl<'a, K, V, H> IntoZval for IndexMap<K, V, H>
where
    K: Into<ArrayKey<'a>>,
    V: IntoZval,
    H: BuildHasher + Default,
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
    use indexmap::IndexMap;

    use crate::boxed::ZBox;
    use crate::convert::{FromZval, IntoZval};
    use crate::embed::Embed;
    use crate::error::Error;
    use crate::types::{ArrayKey, ZendHashTable, Zval};

    #[test]
    fn test_hash_table_try_from_index_map() {
        Embed::run(|| {
            let mut map = IndexMap::new();
            map.insert("key1", "value1");
            map.insert("key2", "value2");
            map.insert("key3", "value3");

            let ht: ZBox<ZendHashTable> = map.try_into().unwrap();
            assert_eq!(ht.len(), 3);
            assert_eq!(ht.get("key1").unwrap().string().unwrap(), "value1");
            assert_eq!(ht.get("key2").unwrap().string().unwrap(), "value2");
            assert_eq!(ht.get("key3").unwrap().string().unwrap(), "value3");

            let mut map_i64 = IndexMap::new();
            map_i64.insert(1, "value1");
            map_i64.insert(2, "value2");
            map_i64.insert(3, "value3");

            let ht_i64: ZBox<ZendHashTable> = map_i64.try_into().unwrap();
            assert_eq!(ht_i64.len(), 3);
            assert_eq!(ht_i64.get(1).unwrap().string().unwrap(), "value1");
            assert_eq!(ht_i64.get(2).unwrap().string().unwrap(), "value2");
            assert_eq!(ht_i64.get(3).unwrap().string().unwrap(), "value3");
        });
    }

    #[test]
    fn test_index_map_into_zval() {
        Embed::run(|| {
            let mut map = IndexMap::new();
            map.insert("key1", "value1");
            map.insert("key2", "value2");
            map.insert("key3", "value3");

            let zval = map.into_zval(false).unwrap();
            assert!(zval.is_array());
            let ht: &ZendHashTable = zval.array().unwrap();
            assert_eq!(ht.len(), 3);
            assert_eq!(ht.get("key1").unwrap().string().unwrap(), "value1");
            assert_eq!(ht.get("key2").unwrap().string().unwrap(), "value2");
            assert_eq!(ht.get("key3").unwrap().string().unwrap(), "value3");

            let mut map_i64 = IndexMap::new();
            map_i64.insert(1, "value1");
            map_i64.insert(2, "value2");
            map_i64.insert(3, "value3");
            let zval_i64 = map_i64.into_zval(false).unwrap();
            assert!(zval_i64.is_array());
            let ht_i64: &ZendHashTable = zval_i64.array().unwrap();
            assert_eq!(ht_i64.len(), 3);
            assert_eq!(ht_i64.get(1).unwrap().string().unwrap(), "value1");
            assert_eq!(ht_i64.get(2).unwrap().string().unwrap(), "value2");
            assert_eq!(ht_i64.get(3).unwrap().string().unwrap(), "value3");
        });
    }

    #[test]
    fn test_index_map_from_zval() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();
            ht.insert("key1", "value1").unwrap();
            ht.insert("key2", "value2").unwrap();
            ht.insert("key3", "value3").unwrap();
            let mut zval = Zval::new();
            zval.set_hashtable(ht);

            let map = IndexMap::<String, String>::from_zval(&zval).unwrap();
            assert_eq!(map.len(), 3);
            assert_eq!(map.get("key1").unwrap(), "value1");
            assert_eq!(map.get("key2").unwrap(), "value2");
            assert_eq!(map.get("key3").unwrap(), "value3");

            let mut ht_i64 = ZendHashTable::new();
            ht_i64.insert(1, "value1").unwrap();
            ht_i64.insert("2", "value2").unwrap();
            ht_i64.insert(3, "value3").unwrap();
            let mut zval_i64 = Zval::new();
            zval_i64.set_hashtable(ht_i64);

            let map_i64 = IndexMap::<i64, String>::from_zval(&zval_i64).unwrap();
            assert_eq!(map_i64.len(), 3);
            assert_eq!(map_i64.get(&1).unwrap(), "value1");
            assert_eq!(map_i64.get(&2).unwrap(), "value2");
            assert_eq!(map_i64.get(&3).unwrap(), "value3");

            let mut ht_mixed = ZendHashTable::new();
            ht_mixed.insert("key1", "value1").unwrap();
            ht_mixed.insert(2, "value2").unwrap();
            ht_mixed.insert("3", "value3").unwrap();
            let mut zval_mixed = Zval::new();
            zval_mixed.set_hashtable(ht_mixed);

            let map_mixed = IndexMap::<String, String>::from_zval(&zval_mixed);
            assert!(map_mixed.is_some());
        });
    }

    #[test]
    fn test_index_map_array_key_from_zval() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();
            ht.insert("key1", "value1").unwrap();
            ht.insert(2, "value2").unwrap();
            ht.insert("3", "value3").unwrap();
            let mut zval = Zval::new();
            zval.set_hashtable(ht);

            let map = IndexMap::<ArrayKey, String>::from_zval(&zval).unwrap();
            assert_eq!(map.len(), 3);
            assert_eq!(
                map.get(&ArrayKey::String("key1".to_string())).unwrap(),
                "value1"
            );
            assert_eq!(map.get(&ArrayKey::Long(2)).unwrap(), "value2");
            assert_eq!(map.get(&ArrayKey::Long(3)).unwrap(), "value3");
        });
    }

    #[test]
    fn test_index_map_i64_v_try_from_hash_table() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();
            ht.insert(1, "value1").unwrap();
            ht.insert("2", "value2").unwrap();

            let map: IndexMap<i64, String> = ht.as_ref().try_into().unwrap();
            assert_eq!(map.len(), 2);
            assert_eq!(map.get(&1).unwrap(), "value1");
            assert_eq!(map.get(&2).unwrap(), "value2");

            let mut ht2 = ZendHashTable::new();
            ht2.insert("key1", "value1").unwrap();
            ht2.insert("key2", "value2").unwrap();

            let map_err: crate::error::Result<IndexMap<i64, String>> = ht2.as_ref().try_into();
            assert!(map_err.is_err());
            assert!(matches!(map_err.unwrap_err(), Error::InvalidProperty));
        });
    }

    #[test]
    fn test_index_map_string_v_try_from_hash_table() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();
            ht.insert("key1", "value1").unwrap();
            ht.insert("key2", "value2").unwrap();

            let map: IndexMap<String, String> = ht.as_ref().try_into().unwrap();
            assert_eq!(map.len(), 2);
            assert_eq!(map.get("key1").unwrap(), "value1");
            assert_eq!(map.get("key2").unwrap(), "value2");

            let mut ht2 = ZendHashTable::new();
            ht2.insert(1, "value1").unwrap();
            ht2.insert(2, "value2").unwrap();

            let map2: crate::error::Result<IndexMap<String, String>> = ht2.as_ref().try_into();
            assert!(map2.is_ok());
        });
    }

    #[test]
    fn test_index_map_array_key_v_try_from_hash_table() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();
            ht.insert("key1", "value1").unwrap();
            ht.insert(2, "value2").unwrap();
            ht.insert("3", "value3").unwrap();

            let map: IndexMap<ArrayKey, String> = ht.as_ref().try_into().unwrap();
            assert_eq!(map.len(), 3);
            assert_eq!(
                map.get(&ArrayKey::String("key1".to_string())).unwrap(),
                "value1"
            );
            assert_eq!(map.get(&ArrayKey::Long(2)).unwrap(), "value2");
            assert_eq!(map.get(&ArrayKey::Long(3)).unwrap(), "value3");
        });
    }

    #[test]
    fn test_index_map_preserves_order() {
        Embed::run(|| {
            let mut map = IndexMap::new();
            map.insert("z", 1);
            map.insert("a", 2);
            map.insert("m", 3);

            let ht: ZBox<ZendHashTable> = map.try_into().unwrap();

            // Verify order is preserved by iterating
            let keys: Vec<_> = ht.iter().map(|(k, _)| k.to_string()).collect();
            assert_eq!(keys, vec!["z", "a", "m"]);

            // Convert back and verify order
            let map_back: IndexMap<String, i64> = ht.as_ref().try_into().unwrap();
            let keys_back: Vec<_> = map_back.keys().cloned().collect();
            assert_eq!(keys_back, vec!["z", "a", "m"]);
        });
    }
}
