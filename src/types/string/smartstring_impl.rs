use super::{DataType, FromZval, IntoZval, Result, Zval};
use smartstring::{LazyCompact, SmartString};

impl<M: smartstring::SmartStringMode> IntoZval for SmartString<M> {
    const TYPE: DataType = DataType::String;
    const NULLABLE: bool = false;

    fn set_zval(self, zv: &mut Zval, persistent: bool) -> Result<()> {
        zv.set_string(self.as_str(), persistent)
    }
}

impl FromZval<'_> for SmartString<LazyCompact> {
    const TYPE: DataType = DataType::String;

    fn from_zval(zval: &Zval) -> Option<Self> {
        zval.str().map(SmartString::from)
    }
}

#[cfg(test)]
#[cfg(feature = "embed")]
mod tests {
    use super::*;
    use crate::convert::FromZval;
    use crate::embed::Embed;

    #[test]
    fn test_smartstring_from_zval() {
        Embed::run(|| {
            let result = Embed::eval("'hello smartstring';");
            assert!(result.is_ok());

            let zval = result.as_ref().unwrap();
            let smart: Option<SmartString<LazyCompact>> = FromZval::from_zval(zval);
            assert_eq!(smart, Some(SmartString::from("hello smartstring")));
        });
    }

    #[test]
    fn test_smartstring_into_zval() {
        Embed::run(|| {
            let smart: SmartString<LazyCompact> = SmartString::from("test string");
            let zval = smart.into_zval(false).unwrap();

            assert!(zval.is_string());
            assert_eq!(zval.str(), Some("test string"));
        });
    }
}
