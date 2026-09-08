use std::{ffi::CString, mem::ManuallyDrop, ptr};

use crate::{
    args::ArgInfoTables,
    builders::{FunctionBuilder, function::free_registered_entries},
    convert::IntoZval,
    describe::DocComments,
    enum_::{Discriminant, EnumCase},
    error::Result,
    ffi::{zend_enum_add_case, zend_register_internal_enum},
    flags::{DataType, DataTypeExt, MethodFlags},
    types::{ZendStr, Zval},
    zend::{ClassEntry, ExecutorGlobals, FunctionEntry},
};

/// A builder for PHP enums.
#[must_use]
pub struct EnumBuilder {
    pub(crate) name: String,
    pub(crate) methods: Vec<(FunctionBuilder<'static>, MethodFlags)>,
    pub(crate) cases: Vec<&'static EnumCase>,
    pub(crate) datatype: DataType,
    register: Option<fn(&'static mut ClassEntry, ArgInfoTables)>,
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

    /// Function to register the enum with PHP, called once it is built.
    ///
    /// See [`ClassBuilder::registration`] for the ownership contract of the
    /// argument info tables.
    ///
    /// # Parameters
    ///
    /// * `register` - The function to call to register the enum.
    ///
    /// [`ClassBuilder::registration`]: crate::builders::ClassBuilder::registration
    pub fn registration(mut self, register: fn(&'static mut ClassEntry, ArgInfoTables)) -> Self {
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
    /// * If called outside a module startup (MINIT) function.
    /// * If the registration function was not set prior to calling this
    ///   method.
    ///
    /// # Errors
    ///
    /// If the enum could not be registered, e.g. due to an invalid name or
    /// data type.
    pub fn register(self) -> Result<()> {
        assert!(
            !ExecutorGlobals::get().current_module.is_null(),
            "Enums can only be registered from a module startup (MINIT) function: \
             `do_register_internal_class` dereferences `EG(current_module)`."
        );

        let mut arg_info = Vec::with_capacity(self.methods.len());
        let mut methods = Vec::with_capacity(self.methods.len() + 1);
        for (method, flags) in self.methods {
            let (mut entry, args) = method.build()?;
            entry.flags |= flags.bits();
            methods.push(entry);
            arg_info.push(args);
        }

        // The engine keeps `zend_internal_function.arg_info` pointing into these
        // for the life of the process, so `register` parks them.
        let arg_info = ManuallyDrop::new(arg_info.into_boxed_slice());

        methods.push(FunctionEntry::end());

        let name = CString::new(self.name)?;
        let backing_type = self.datatype.as_u32().try_into()?;
        let entries = Box::into_raw(methods.into_boxed_slice());

        let class = unsafe {
            zend_register_internal_enum(
                name.as_ptr(),
                backing_type,
                entries.cast::<FunctionEntry>(),
            )
        };

        // SAFETY: `zend_register_internal_enum` funnels through
        // `do_register_internal_class`, which has interned every `fname` and read
        // `builtin_functions` for the last time.
        unsafe { free_registered_entries(entries) };
        unsafe { (*class).info.internal.builtin_functions = ptr::null() };

        for case in self.cases {
            let name = ZendStr::new_interned(case.name, true);
            // `zend_enum_add_case` interns a string discriminant in place and
            // `create_enum_case_ast` takes the payload with `ZVAL_COPY_VALUE`, so the
            // case AST owns it and the `Zval` must not run its destructor.
            let mut value = match &case.discriminant {
                Some(value) => Some(ManuallyDrop::new(Self::create_enum_value(value)?)),
                None => None,
            };
            let value = value
                .as_mut()
                .map_or(ptr::null_mut(), |value| &raw mut **value);
            unsafe {
                zend_enum_add_case(class, name.into_raw(), value);
            }
        }

        if let Some(register) = self.register {
            register(unsafe { &mut *class }, ManuallyDrop::into_inner(arg_info));
        } else {
            panic!("Enum was not registered with a registration function");
        }

        Ok(())
    }

    fn create_enum_value(discriminant: &Discriminant) -> Result<Zval> {
        Ok(match discriminant {
            Discriminant::Int(i) => i.into_zval(false)?,
            Discriminant::String(s) => s.into_zval(true)?,
        })
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
