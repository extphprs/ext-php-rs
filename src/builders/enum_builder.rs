use std::{ffi::CString, ptr};

use crate::{
    builders::FunctionBuilder,
    convert::IntoZval,
    describe::DocComments,
    enum_::{Discriminant, EnumCase},
    error::Result,
    ffi::{zend_enum_add_case, zend_register_internal_enum},
    flags::{DataType, MethodFlags},
    types::{ZendStr, Zval},
    zend::{ClassEntry, FunctionEntry},
};

/// A builder for PHP enums.
#[must_use]
pub struct EnumBuilder {
    pub(crate) name: String,
    pub(crate) methods: Vec<(FunctionBuilder<'static>, MethodFlags)>,
    pub(crate) cases: Vec<&'static EnumCase>,
    pub(crate) datatype: DataType,
    register: Option<fn(&'static mut ClassEntry)>,
    pub(crate) docs: DocComments,
}

impl EnumBuilder {
    /// Creates a new enum builder with the given name.
    pub fn new<T: Into<String>>(name: T) -> Self {
        Self {
            name: name.into(),
            methods: Vec::default(),
            cases: Vec::default(),
            datatype: DataType::Undef,
            register: None,
            docs: DocComments::default(),
        }
    }

    /// Adds an enum case to the enum.
    ///
    /// # Panics
    ///
    /// If the case's data type does not match the enum's data type
    pub fn case(mut self, case: &'static EnumCase) -> Self {
        let data_type = case.data_type();
        assert!(
            data_type == self.datatype || self.cases.is_empty(),
            "Cannot add case with data type {:?} to enum with data type {:?}",
            data_type,
            self.datatype
        );

        self.datatype = data_type;
        self.cases.push(case);

        self
    }

    /// Adds a method to the enum.
    pub fn method(mut self, method: FunctionBuilder<'static>, flags: MethodFlags) -> Self {
        self.methods.push((method, flags));
        self
    }

    /// Function to register the class with PHP. This function is called after
    /// the class is built.
    ///
    /// # Parameters
    ///
    /// * `register` - The function to call to register the class.
    pub fn registration(mut self, register: fn(&'static mut ClassEntry)) -> Self {
        self.register = Some(register);
        self
    }

    /// Add documentation comments to the enum.
    pub fn docs(mut self, docs: DocComments) -> Self {
        self.docs = docs;
        self
    }

    /// Registers the enum with PHP.
    ///
    /// # Panics
    ///
    /// If the registration function was not set prior to calling this
    /// method.
    ///
    /// # Errors
    ///
    /// If the enum could not be registered, e.g. due to an invalid name or
    /// data type.
    pub fn register(self) -> Result<()> {
        let mut methods = self
            .methods
            .into_iter()
            .map(|(method, flags)| {
                method.build().map(|mut method| {
                    method.flags |= flags.bits();
                    method
                })
            })
            .collect::<Result<Vec<_>>>()?;
        methods.push(FunctionEntry::end());

        let class = unsafe {
            zend_register_internal_enum(
                CString::new(self.name)?.as_ptr(),
                self.datatype.as_u32().try_into()?,
                methods.into_boxed_slice().as_ptr(),
            )
        };

        for case in self.cases {
            let name = ZendStr::new_interned(case.name, true);
            let value = match &case.discriminant {
                Some(value) => Self::create_enum_value(value)?,
                None => ptr::null_mut(),
            };
            unsafe {
                zend_enum_add_case(class, name.into_raw(), value);
            }
        }

        if let Some(register) = self.register {
            register(unsafe { &mut *class });
        } else {
            panic!("Enum was not registered with a registration function");
        }

        Ok(())
    }

    fn create_enum_value(discriminant: &Discriminant) -> Result<*mut Zval> {
        let value: Zval = match discriminant {
            Discriminant::Int(i) => i.into_zval(false)?,
            Discriminant::String(s) => s.into_zval(true)?,
        };

        let boxed_value = Box::new(value);
        Ok(Box::into_raw(boxed_value).cast())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enum_::Discriminant;

    const case1: EnumCase = EnumCase {
        name: "Variant1",
        discriminant: None,
        docs: &[],
    };
    const case2: EnumCase = EnumCase {
        name: "Variant2",
        discriminant: Some(Discriminant::Int(42)),
        docs: &[],
    };
    const case3: EnumCase = EnumCase {
        name: "Variant3",
        discriminant: Some(Discriminant::String("foo")),
        docs: &[],
    };

    #[test]
    fn test_new_enum_builder() {
        let builder = EnumBuilder::new("MyEnum");
        assert_eq!(builder.name, "MyEnum");
        assert!(builder.methods.is_empty());
        assert!(builder.cases.is_empty());
        assert_eq!(builder.datatype, DataType::Undef);
        assert!(builder.register.is_none());
    }

    #[test]
    fn test_enum_case() {
        let builder = EnumBuilder::new("MyEnum").case(&case1);
        assert_eq!(builder.cases.len(), 1);
        assert_eq!(builder.cases[0].name, "Variant1");
        assert_eq!(builder.datatype, DataType::Undef);

        let builder = EnumBuilder::new("MyEnum").case(&case2);
        assert_eq!(builder.cases.len(), 1);
        assert_eq!(builder.cases[0].name, "Variant2");
        assert_eq!(builder.cases[0].discriminant, Some(Discriminant::Int(42)));
        assert_eq!(builder.datatype, DataType::Long);

        let builder = EnumBuilder::new("MyEnum").case(&case3);
        assert_eq!(builder.cases.len(), 1);
        assert_eq!(builder.cases[0].name, "Variant3");
        assert_eq!(
            builder.cases[0].discriminant,
            Some(Discriminant::String("foo"))
        );
        assert_eq!(builder.datatype, DataType::String);
    }

    #[test]
    #[should_panic(expected = "Cannot add case with data type Long to enum with data type Undef")]
    fn test_enum_case_mismatch() {
        #[allow(unused_must_use)]
        EnumBuilder::new("MyEnum").case(&case1).case(&case2); // This should panic because case2 has a different data type
    }

    const docs: DocComments = &["This is a test enum"];
    #[test]
    fn test_docs() {
        let builder = EnumBuilder::new("MyEnum").docs(docs);
        assert_eq!(builder.docs.len(), 1);
        assert_eq!(builder.docs[0], "This is a test enum");
    }
}
