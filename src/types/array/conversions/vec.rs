use std::convert::TryFrom;

use crate::{
    boxed::ZBox,
    convert::{FromZval, IntoZval},
    error::{Error, Result},
    flags::DataType,
    types::Zval,
};

use super::super::{ArrayKey, ZendHashTable};

///////////////////////////////////////////
// Vec<(K, V)> conversions
///////////////////////////////////////////

impl<'a, K, V> TryFrom<&'a ZendHashTable> for Vec<(K, V)>
where
    K: TryFrom<ArrayKey<'a>, Error = Error>,
    V: FromZval<'a>,
{
    type Error = Error;

    fn try_from(value: &'a ZendHashTable) -> Result<Self> {
        let mut vec = Vec::with_capacity(value.len());

        for (key, val) in value {
            vec.push((
                key.try_into()?,
                V::from_zval(val).ok_or_else(|| Error::ZvalConversion(val.get_type()))?,
            ));
        }

        Ok(vec)
    }
}

impl<'a, V> TryFrom<&'a ZendHashTable> for Vec<(ArrayKey<'a>, V)>
where
    V: FromZval<'a>,
{
    type Error = Error;

    fn try_from(value: &'a ZendHashTable) -> Result<Self> {
        let mut vec = Vec::with_capacity(value.len());

        for (key, val) in value {
            vec.push((
                key,
                V::from_zval(val).ok_or_else(|| Error::ZvalConversion(val.get_type()))?,
            ));
        }

        Ok(vec)
    }
}

impl<'a, K, V> TryFrom<Vec<(K, V)>> for ZBox<ZendHashTable>
where
    K: Into<ArrayKey<'a>>,
    V: IntoZval,
{
    type Error = Error;

    fn try_from(value: Vec<(K, V)>) -> Result<Self> {
        let mut ht = ZendHashTable::with_capacity(
            value.len().try_into().map_err(|_| Error::IntegerOverflow)?,
        );

        for (k, v) in value {
            ht.insert(k, v)?;
        }

        Ok(ht)
    }
}

impl<'a, K, V> IntoZval for Vec<(K, V)>
where
    K: Into<ArrayKey<'a>>,
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

impl<'a, K, V> FromZval<'a> for Vec<(K, V)>
where
    K: TryFrom<ArrayKey<'a>, Error = Error>,
    V: FromZval<'a>,
{
    const TYPE: DataType = DataType::Array;

    fn from_zval(zval: &'a Zval) -> Option<Self> {
        zval.array().and_then(|arr| arr.try_into().ok())
    }
}

impl<'a, V> FromZval<'a> for Vec<(ArrayKey<'a>, V)>
where
    V: FromZval<'a>,
{
    const TYPE: DataType = DataType::Array;

    fn from_zval(zval: &'a Zval) -> Option<Self> {
        zval.array().and_then(|arr| arr.try_into().ok())
    }
}

///////////////////////////////////////////
// Vec<T> conversions
///////////////////////////////////////////

impl<'a, T> TryFrom<&'a ZendHashTable> for Vec<T>
where
    T: FromZval<'a>,
{
    type Error = Error;

    fn try_from(value: &'a ZendHashTable) -> Result<Self> {
        let mut vec = Vec::with_capacity(value.len());

        for (_, val) in value {
            vec.push(T::from_zval(val).ok_or_else(|| Error::ZvalConversion(val.get_type()))?);
        }

        Ok(vec)
    }
}

impl<T> TryFrom<Vec<T>> for ZBox<ZendHashTable>
where
    T: IntoZval,
{
    type Error = Error;

    fn try_from(value: Vec<T>) -> Result<Self> {
        let mut ht = ZendHashTable::with_capacity(
            value.len().try_into().map_err(|_| Error::IntegerOverflow)?,
        );

        for val in value {
            ht.push(val)?;
        }

        Ok(ht)
    }
}

impl<T> IntoZval for Vec<T>
where
    T: IntoZval,
{
    const TYPE: DataType = DataType::Array;
    const NULLABLE: bool = false;

    fn set_zval(self, zv: &mut Zval, _: bool) -> Result<()> {
        let arr = self.try_into()?;
        zv.set_hashtable(arr);
        Ok(())
    }
}

impl<'a, T> FromZval<'a> for Vec<T>
where
    T: FromZval<'a>,
{
    const TYPE: DataType = DataType::Array;

    fn from_zval(zval: &'a Zval) -> Option<Self> {
        zval.array().and_then(|arr| arr.try_into().ok())
    }
}

#[cfg(test)]
#[cfg(feature = "embed")]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::boxed::ZBox;
    use crate::convert::{FromZval, IntoZval};
    use crate::embed::Embed;
    use crate::error::Error;
    use crate::types::{ArrayKey, ZendHashTable, Zval};

    #[test]
    fn test_hash_table_try_from_vec() {
        Embed::run(|| {
            let vec = vec![("key1", "value1"), ("key2", "value2"), ("key3", "value3")];

            let ht: ZBox<ZendHashTable> = vec.try_into().unwrap();
            assert_eq!(ht.len(), 3);
            assert_eq!(ht.get("key1").unwrap().string().unwrap(), "value1");
            assert_eq!(ht.get("key2").unwrap().string().unwrap(), "value2");
            assert_eq!(ht.get("key3").unwrap().string().unwrap(), "value3");

            let vec_i64 = vec![(1, "value1"), (2, "value2"), (3, "value3")];

            let ht_i64: ZBox<ZendHashTable> = vec_i64.try_into().unwrap();
            assert_eq!(ht_i64.len(), 3);
            assert_eq!(ht_i64.get(1).unwrap().string().unwrap(), "value1");
            assert_eq!(ht_i64.get(2).unwrap().string().unwrap(), "value2");
            assert_eq!(ht_i64.get(3).unwrap().string().unwrap(), "value3");
        });
    }

    #[test]
    fn test_vec_k_v_into_zval() {
        Embed::run(|| {
            let vec = vec![("key1", "value1"), ("key2", "value2"), ("key3", "value3")];

            let zval = vec.into_zval(false).unwrap();
            assert!(zval.is_array());
            let ht: &ZendHashTable = zval.array().unwrap();
            assert_eq!(ht.len(), 3);
            assert_eq!(ht.get("key1").unwrap().string().unwrap(), "value1");
            assert_eq!(ht.get("key2").unwrap().string().unwrap(), "value2");
            assert_eq!(ht.get("key3").unwrap().string().unwrap(), "value3");

            let vec_i64 = vec![(1, "value1"), (2, "value2"), (3, "value3")];
            let zval_i64 = vec_i64.into_zval(false).unwrap();
            assert!(zval_i64.is_array());
            let ht_i64: &ZendHashTable = zval_i64.array().unwrap();
            assert_eq!(ht_i64.len(), 3);
            assert_eq!(ht_i64.get(1).unwrap().string().unwrap(), "value1");
            assert_eq!(ht_i64.get(2).unwrap().string().unwrap(), "value2");
            assert_eq!(ht_i64.get(3).unwrap().string().unwrap(), "value3");
        });
    }

    #[test]
    fn test_vec_k_v_from_zval() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();
            ht.insert("key1", "value1").unwrap();
            ht.insert("key2", "value2").unwrap();
            ht.insert("key3", "value3").unwrap();
            let mut zval = Zval::new();
            zval.set_hashtable(ht);

            let vec: Vec<(String, String)> = Vec::<(String, String)>::from_zval(&zval).unwrap();
            assert_eq!(vec.len(), 3);
            assert_eq!(vec[0].0, "key1");
            assert_eq!(vec[0].1, "value1");
            assert_eq!(vec[1].0, "key2");
            assert_eq!(vec[1].1, "value2");
            assert_eq!(vec[2].0, "key3");
            assert_eq!(vec[2].1, "value3");

            let mut ht_i64 = ZendHashTable::new();
            ht_i64.insert(1, "value1").unwrap();
            ht_i64.insert("2", "value2").unwrap();
            ht_i64.insert(3, "value3").unwrap();
            let mut zval_i64 = Zval::new();
            zval_i64.set_hashtable(ht_i64);

            let vec_i64: Vec<(i64, String)> = Vec::<(i64, String)>::from_zval(&zval_i64).unwrap();
            assert_eq!(vec_i64.len(), 3);
            assert_eq!(vec_i64[0].0, 1);
            assert_eq!(vec_i64[0].1, "value1");
            assert_eq!(vec_i64[1].0, 2);
            assert_eq!(vec_i64[1].1, "value2");
            assert_eq!(vec_i64[2].0, 3);
            assert_eq!(vec_i64[2].1, "value3");

            let mut ht_mixed = ZendHashTable::new();
            ht_mixed.insert("key1", "value1").unwrap();
            ht_mixed.insert(2, "value2").unwrap();
            ht_mixed.insert("3", "value3").unwrap();
            let mut zval_mixed = Zval::new();
            zval_mixed.set_hashtable(ht_mixed);

            let vec_mixed: Option<Vec<(String, String)>> =
                Vec::<(String, String)>::from_zval(&zval_mixed);
            assert!(vec_mixed.is_some());
        });
    }

    #[test]
    fn test_vec_array_key_v_from_zval() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();
            ht.insert("key1", "value1").unwrap();
            ht.insert(2, "value2").unwrap();
            ht.insert("3", "value3").unwrap();
            let mut zval = Zval::new();
            zval.set_hashtable(ht);

            let vec: Vec<(ArrayKey, String)> = Vec::<(ArrayKey, String)>::from_zval(&zval).unwrap();
            assert_eq!(vec.len(), 3);
            assert_eq!(vec[0].0, ArrayKey::String("key1".to_string()));
            assert_eq!(vec[0].1, "value1");
            assert_eq!(vec[1].0, ArrayKey::Long(2));
            assert_eq!(vec[1].1, "value2");
            assert_eq!(vec[2].0, ArrayKey::Long(3));
            assert_eq!(vec[2].1, "value3");
        });
    }

    #[test]
    fn test_vec_i64_v_try_from_hash_table() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();
            ht.insert(1, "value1").unwrap();
            ht.insert("2", "value2").unwrap();

            let vec: Vec<(i64, String)> = ht.as_ref().try_into().unwrap();
            assert_eq!(vec.len(), 2);
            assert_eq!(vec[0].0, 1);
            assert_eq!(vec[0].1, "value1");
            assert_eq!(vec[1].0, 2);
            assert_eq!(vec[1].1, "value2");

            let mut ht2 = ZendHashTable::new();
            ht2.insert("key1", "value1").unwrap();
            ht2.insert("key2", "value2").unwrap();

            let vec2: crate::error::Result<Vec<(i64, String)>> = ht2.as_ref().try_into();
            assert!(vec2.is_err());
            assert!(matches!(vec2.unwrap_err(), Error::ZvalConversion(_)));
        });
    }

    #[test]
    fn test_vec_string_v_try_from_hash_table() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();
            ht.insert("key1", "value1").unwrap();
            ht.insert("key2", "value2").unwrap();

            let vec: Vec<(String, String)> = ht.as_ref().try_into().unwrap();
            assert_eq!(vec.len(), 2);
            assert_eq!(vec[0].0, "key1");
            assert_eq!(vec[0].1, "value1");
            assert_eq!(vec[1].0, "key2");
            assert_eq!(vec[1].1, "value2");

            let mut ht2 = ZendHashTable::new();
            ht2.insert(1, "value1").unwrap();
            ht2.insert(2, "value2").unwrap();

            let vec2: crate::error::Result<Vec<(String, String)>> = ht2.as_ref().try_into();
            assert!(vec2.is_ok());
        });
    }

    #[test]
    fn test_vec_array_key_v_try_from_hash_table() {
        Embed::run(|| {
            let mut ht = ZendHashTable::new();
            ht.insert("key1", "value1").unwrap();
            ht.insert(2, "value2").unwrap();
            ht.insert("3", "value3").unwrap();

            let vec: Vec<(ArrayKey, String)> = ht.as_ref().try_into().unwrap();
            assert_eq!(vec.len(), 3);
            assert_eq!(vec[0].0, ArrayKey::String("key1".to_string()));
            assert_eq!(vec[0].1, "value1");
            assert_eq!(vec[1].0, ArrayKey::Long(2));
            assert_eq!(vec[1].1, "value2");
            assert_eq!(vec[2].0, ArrayKey::Long(3));
            assert_eq!(vec[2].1, "value3");
        });
    }
}
