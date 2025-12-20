use std::{ffi::CString, mem::MaybeUninit, ptr, rc::Rc};

use crate::{
    builders::FunctionBuilder,
    class::{ClassEntryInfo, ConstructorMeta, ConstructorResult, RegisteredClass},
    convert::{IntoZval, IntoZvalDyn},
    describe::DocComments,
    error::{Error, Result},
    exception::PhpException,
    ffi::{
        zend_declare_class_constant, zend_declare_property, zend_do_implement_interface,
        zend_register_internal_class_ex, zend_register_internal_interface,
    },
    flags::{ClassFlags, MethodFlags, PropertyFlags},
    types::{ZendClassObject, ZendObject, ZendStr, Zval},
    zend::{ClassEntry, ExecuteData, FunctionEntry},
    zend_fastcall,
};

/// A constant entry: (name, `value_closure`, docs, `stub_value`)
type ConstantEntry = (
    String,
    Box<dyn FnOnce() -> Result<Zval>>,
    DocComments,
    String,
);
type PropertyDefault = Option<Box<dyn FnOnce() -> Result<Zval>>>;

/// Builder for registering a class in PHP.
#[must_use]
pub struct ClassBuilder {
    pub(crate) name: String,
    ce: ClassEntry,
    pub(crate) extends: Option<ClassEntryInfo>,
    pub(crate) interfaces: Vec<ClassEntryInfo>,
    pub(crate) methods: Vec<(FunctionBuilder<'static>, MethodFlags)>,
    object_override: Option<unsafe extern "C" fn(class_type: *mut ClassEntry) -> *mut ZendObject>,
    pub(crate) properties: Vec<(String, PropertyFlags, PropertyDefault, DocComments)>,
    pub(crate) constants: Vec<ConstantEntry>,
    register: Option<fn(&'static mut ClassEntry)>,
    pub(crate) docs: DocComments,
}

impl ClassBuilder {
    /// Creates a new class builder, used to build classes
    /// to be exported to PHP.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the class.
    pub fn new<T: Into<String>>(name: T) -> Self {
        Self {
            name: name.into(),
            // SAFETY: A zeroed class entry is in an initialized state, as it is a raw C type
            // whose fields do not have a drop implementation.
            ce: unsafe { MaybeUninit::zeroed().assume_init() },
            extends: None,
            interfaces: vec![],
            methods: vec![],
            object_override: None,
            properties: vec![],
            constants: vec![],
            register: None,
            docs: &[],
        }
    }

    /// Return PHP class flags
    #[must_use]
    pub fn get_flags(&self) -> u32 {
        self.ce.ce_flags
    }

    /// Sets the class builder to extend another class.
    ///
    /// # Parameters
    ///
    /// * `parent` - The parent class to extend.
    pub fn extends(mut self, parent: ClassEntryInfo) -> Self {
        self.extends = Some(parent);
        self
    }

    /// Implements an interface on the class.
    ///
    /// # Parameters
    ///
    /// * `interface` - Interface to implement on the class.
    ///
    /// # Panics
    ///
    /// Panics when the given class entry `interface` is not an interface.
    pub fn implements(mut self, interface: ClassEntryInfo) -> Self {
        self.interfaces.push(interface);
        self
    }

    /// Adds a method to the class.
    ///
    /// # Parameters
    ///
    /// * `func` - The function builder to add to the class.
    /// * `flags` - Flags relating to the function. See [`MethodFlags`].
    pub fn method(mut self, func: FunctionBuilder<'static>, flags: MethodFlags) -> Self {
        self.methods.push((func, flags));
        self
    }

    /// Adds a property to the class.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the property to add to the class.
    /// * `flags` - Flags relating to the property. See [`PropertyFlags`].
    /// * `default` - Optional default value for the property.
    /// * `docs` - Documentation comments for the property.
    pub fn property<T: Into<String>>(
        mut self,
        name: T,
        flags: PropertyFlags,
        default: PropertyDefault,
        docs: DocComments,
    ) -> Self {
        self.properties.push((name.into(), flags, default, docs));
        self
    }

    /// Adds a constant to the class. The type of the constant is defined by the
    /// type of the given default.
    ///
    /// Returns a result containing the class builder if the constant was
    /// successfully added.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the constant to add to the class.
    /// * `value` - The value of the constant.
    /// * `docs` - Documentation comments for the constant.
    ///
    /// # Errors
    ///
    /// TODO: Never?
    pub fn constant<T: Into<String>>(
        mut self,
        name: T,
        value: impl IntoZval + 'static,
        docs: DocComments,
    ) -> Result<Self> {
        // Convert to Zval first to get stub value
        let zval = value.into_zval(true)?;
        let stub = crate::convert::zval_to_stub(&zval);
        self.constants
            .push((name.into(), Box::new(|| Ok(zval)), docs, stub));
        Ok(self)
    }

    /// Adds a constant to the class from a `dyn` object. The type of the
    /// constant is defined by the type of the value.
    ///
    /// Returns a result containing the class builder if the constant was
    /// successfully added.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the constant to add to the class.
    /// * `value` - The value of the constant.
    /// * `docs` - Documentation comments for the constant.
    ///
    /// # Errors
    ///
    /// TODO: Never?
    pub fn dyn_constant<T: Into<String>>(
        mut self,
        name: T,
        value: &'static dyn IntoZvalDyn,
        docs: DocComments,
    ) -> Result<Self> {
        let stub = value.stub_value();
        let value = Rc::new(value);
        self.constants.push((
            name.into(),
            Box::new(move || value.as_zval(true)),
            docs,
            stub,
        ));
        Ok(self)
    }

    /// Sets the flags for the class.
    ///
    /// # Parameters
    ///
    /// * `flags` - Flags relating to the class. See [`ClassFlags`].
    pub fn flags(mut self, flags: ClassFlags) -> Self {
        self.ce.ce_flags = flags.bits();
        self
    }

    /// Overrides the creation of the Zend object which will represent an
    /// instance of this class.
    ///
    /// # Parameters
    ///
    /// * `T` - The type which will override the Zend object. Must implement
    ///   [`RegisteredClass`] which can be derived using the
    ///   [`php_class`](crate::php_class) attribute macro.
    ///
    /// # Panics
    ///
    /// Panics if the class name associated with `T` is not the same as the
    /// class name specified when creating the builder.
    pub fn object_override<T: RegisteredClass>(mut self) -> Self {
        extern "C" fn create_object<T: RegisteredClass>(ce: *mut ClassEntry) -> *mut ZendObject {
            // SAFETY: After calling this function, PHP will always call the constructor
            // defined below, which assumes that the object is uninitialized.
            let obj = unsafe { ZendClassObject::<T>::new_uninit(ce.as_ref()) };
            obj.into_raw().get_mut_zend_obj()
        }

        zend_fastcall! {
            extern fn constructor<T: RegisteredClass>(ex: &mut ExecuteData, _: &mut Zval) {
                let Some(ConstructorMeta { constructor, .. }) = T::constructor() else {
                    PhpException::default("You cannot instantiate this class from PHP.".into())
                        .throw()
                        .expect("Failed to throw exception when constructing class");
                    return;
                };

                let this = match constructor(ex) {
                    ConstructorResult::Ok(this) => this,
                    ConstructorResult::Exception(e) => {
                        e.throw()
                            .expect("Failed to throw exception while constructing class");
                        return;
                    }
                    ConstructorResult::ArgError => return,
                };

                let Some(this_obj) = ex.get_object::<T>() else {
                    PhpException::default("Failed to retrieve reference to `this` object.".into())
                        .throw()
                        .expect("Failed to throw exception while constructing class");
                    return;
                };

                this_obj.initialize(this);
            }
        }

        debug_assert_eq!(
            self.name.as_str(),
            T::CLASS_NAME,
            "Class name in builder does not match class name in `impl RegisteredClass`."
        );
        self.object_override = Some(create_object::<T>);
        let is_interface = T::FLAGS.contains(ClassFlags::Interface);

        let (func, visibility) = if let Some(ConstructorMeta {
            build_fn, flags, ..
        }) = T::constructor()
        {
            let func = if is_interface {
                FunctionBuilder::new_abstract("__construct")
            } else {
                FunctionBuilder::new("__construct", constructor::<T>)
            };

            (build_fn(func), flags.unwrap_or(MethodFlags::Public))
        } else {
            (
                if is_interface {
                    FunctionBuilder::new_abstract("__construct")
                } else {
                    FunctionBuilder::new("__construct", constructor::<T>)
                },
                MethodFlags::Public,
            )
        };

        self.method(func, visibility)
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

    /// Sets the documentation for the class.
    ///
    /// # Parameters
    ///
    /// * `docs` - The documentation comments for the class.
    pub fn docs(mut self, docs: DocComments) -> Self {
        self.docs = docs;
        self
    }

    /// Builds and registers the class.
    ///
    /// # Errors
    ///
    /// * [`Error::InvalidPointer`] - If the class could not be registered.
    /// * [`Error::InvalidCString`] - If the class name is not a valid C string.
    /// * [`Error::IntegerOverflow`] - If the property flags are not valid.
    /// * If a method or property could not be built.
    ///
    /// # Panics
    ///
    /// If no registration function was provided.
    pub fn register(mut self) -> Result<()> {
        self.ce.name = ZendStr::new_interned(&self.name, true).into_raw();

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
        let func = Box::into_raw(methods.into_boxed_slice()) as *const FunctionEntry;
        self.ce.info.internal.builtin_functions = func;

        let class = if self.ce.flags().contains(ClassFlags::Interface) {
            unsafe {
                zend_register_internal_interface(&raw mut self.ce)
                    .as_mut()
                    .ok_or(Error::InvalidPointer)?
            }
        } else {
            unsafe {
                zend_register_internal_class_ex(
                    &raw mut self.ce,
                    match self.extends {
                        Some((ptr, _)) => ptr::from_ref(ptr()).cast_mut(),
                        None => std::ptr::null_mut(),
                    },
                )
                .as_mut()
                .ok_or(Error::InvalidPointer)?
            }
        };

        // disable serialization if the class has an associated object
        if self.object_override.is_some() {
            cfg_if::cfg_if! {
                if #[cfg(php81)] {
                    class.ce_flags |= ClassFlags::NotSerializable.bits();
                } else {
                    class.serialize = Some(crate::ffi::zend_class_serialize_deny);
                    class.unserialize = Some(crate::ffi::zend_class_unserialize_deny);
                }
            }
        }

        for (iface, _) in self.interfaces {
            let interface = iface();
            assert!(
                interface.is_interface(),
                "Given class entry was not an interface."
            );

            unsafe { zend_do_implement_interface(class, ptr::from_ref(interface).cast_mut()) };
        }

        for (name, flags, default, _) in self.properties {
            let mut default_zval = match default {
                Some(f) => f()?,
                None => Zval::new(),
            };
            unsafe {
                zend_declare_property(
                    class,
                    CString::new(name.as_str())?.as_ptr(),
                    name.len() as _,
                    &raw mut default_zval,
                    flags.bits().try_into()?,
                );
            }
        }

        for (name, value, _, _) in self.constants {
            let value = Box::into_raw(Box::new(value()?));
            unsafe {
                zend_declare_class_constant(
                    class,
                    CString::new(name.as_str())?.as_ptr(),
                    name.len(),
                    value,
                );
            };
        }

        if let Some(object_override) = self.object_override {
            class.__bindgen_anon_2.create_object = Some(object_override);
        }

        if let Some(register) = self.register {
            register(class);
        } else {
            panic!("Class {} was not registered.", self.name);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test::test_function;

    use super::*;

    #[test]
    #[allow(unpredictable_function_pointer_comparisons)]
    fn test_new() {
        let class = ClassBuilder::new("Foo");
        assert_eq!(class.name, "Foo");
        assert_eq!(class.extends, None);
        assert_eq!(class.interfaces, vec![]);
        assert_eq!(class.methods.len(), 0);
        assert_eq!(class.object_override, None);
        assert!(class.properties.is_empty());
        assert_eq!(class.constants.len(), 0);
        assert_eq!(class.register, None);
        assert_eq!(class.docs, &[] as DocComments);
    }

    #[test]
    fn test_extends() {
        let extends: ClassEntryInfo = (|| todo!(), "Bar");
        let class = ClassBuilder::new("Foo").extends(extends);
        assert_eq!(class.extends, Some(extends));
    }

    #[test]
    fn test_implements() {
        let implements: ClassEntryInfo = (|| todo!(), "Bar");
        let class = ClassBuilder::new("Foo").implements(implements);
        assert_eq!(class.interfaces, vec![implements]);
    }

    #[test]
    fn test_method() {
        let method = FunctionBuilder::new("foo", test_function);
        let class = ClassBuilder::new("Foo").method(method, MethodFlags::Public);
        assert_eq!(class.methods.len(), 1);
    }

    #[test]
    fn test_property() {
        let class =
            ClassBuilder::new("Foo").property("bar", PropertyFlags::Public, None, &["Doc 1"]);
        assert_eq!(class.properties.len(), 1);
        assert_eq!(class.properties[0].0, "bar");
        assert_eq!(class.properties[0].1, PropertyFlags::Public);
        assert!(class.properties[0].2.is_none());
        assert_eq!(class.properties[0].3, &["Doc 1"] as DocComments);
    }

    #[test]
    #[cfg(feature = "embed")]
    fn test_constant() {
        let class = ClassBuilder::new("Foo")
            .constant("bar", 42, &["Doc 1"])
            .expect("Failed to create constant");
        assert_eq!(class.constants.len(), 1);
        assert_eq!(class.constants[0].0, "bar");
        assert_eq!(class.constants[0].2, &["Doc 1"] as DocComments);
    }

    #[test]
    #[cfg(feature = "embed")]
    fn test_dyn_constant() {
        let class = ClassBuilder::new("Foo")
            .dyn_constant("bar", &42, &["Doc 1"])
            .expect("Failed to create constant");
        assert_eq!(class.constants.len(), 1);
        assert_eq!(class.constants[0].0, "bar");
        assert_eq!(class.constants[0].2, &["Doc 1"] as DocComments);
    }

    #[test]
    fn test_flags() {
        let class = ClassBuilder::new("Foo").flags(ClassFlags::Abstract);
        assert_eq!(class.ce.ce_flags, ClassFlags::Abstract.bits());
    }

    #[test]
    fn test_registration() {
        let class = ClassBuilder::new("Foo").registration(|_| {});
        assert!(class.register.is_some());
    }

    #[test]
    fn test_registration_interface() {
        let class = ClassBuilder::new("Foo")
            .flags(ClassFlags::Interface)
            .registration(|_| {});
        assert!(class.register.is_some());
    }

    #[test]
    fn test_docs() {
        let class = ClassBuilder::new("Foo").docs(&["Doc 1"]);
        assert_eq!(class.docs, &["Doc 1"] as DocComments);
    }

    // TODO: Test the register function
}
