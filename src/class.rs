//! Types and traits used for registering classes with PHP.

use std::{
    collections::HashMap,
    marker::PhantomData,
    sync::atomic::{AtomicPtr, Ordering},
};

use once_cell::sync::OnceCell;

use crate::{
    builders::{ClassBuilder, FunctionBuilder},
    convert::IntoZvalDyn,
    describe::DocComments,
    exception::PhpException,
    flags::{ClassFlags, MethodFlags, PropertyFlags},
    internal::property::PropertyInfo,
    zend::{ClassEntry, ExecuteData, ZendObjectHandlers},
};

/// A type alias for a tuple containing a function pointer to a class entry
/// and a string representing the class name used in stubs.
pub type ClassEntryInfo = (fn() -> &'static ClassEntry, &'static str);

/// Implemented on Rust types which are exported to PHP. Allows users to get and
/// set PHP properties on the object.
pub trait RegisteredClass: Sized + 'static {
    /// PHP class name of the registered class.
    const CLASS_NAME: &'static str;

    /// Function to be called when building the class. Allows user to modify the
    /// class at runtime (add runtime constants etc).
    const BUILDER_MODIFIER: Option<fn(ClassBuilder) -> ClassBuilder>;

    /// Parent class entry. Optional.
    const EXTENDS: Option<ClassEntryInfo>;

    /// Interfaces implemented by the class.
    const IMPLEMENTS: &'static [ClassEntryInfo];

    /// PHP flags applied to the class.
    const FLAGS: ClassFlags = ClassFlags::empty();

    /// Doc comments for the class.
    const DOC_COMMENTS: DocComments = &[];

    /// Returns a reference to the class metadata, which stores the class entry
    /// and handlers.
    ///
    /// This must be statically allocated, and is usually done through the
    /// [`macro@php_class`] macro.
    ///
    /// [`macro@php_class`]: crate::php_class
    fn get_metadata() -> &'static ClassMetadata<Self>;

    /// Returns a hash table containing the properties of the class.
    ///
    /// The key should be the name of the property and the value should be a
    /// reference to the property with reference to `self`. The value is a
    /// [`PropertyInfo`].
    ///
    /// Instead of using this method directly, you should access the properties
    /// through the [`ClassMetadata::get_properties`] function, which builds the
    /// hashmap one and stores it in memory.
    fn get_properties<'a>() -> HashMap<&'static str, PropertyInfo<'a, Self>>;

    /// Returns the method builders required to build the class.
    fn method_builders() -> Vec<(FunctionBuilder<'static>, MethodFlags)>;

    /// Returns the class constructor (if any).
    fn constructor() -> Option<ConstructorMeta<Self>>;

    /// Returns the constants provided by the class.
    fn constants() -> &'static [(&'static str, &'static dyn IntoZvalDyn, DocComments)];

    /// Returns the static properties provided by the class.
    ///
    /// Static properties are declared at the class level and managed by PHP,
    /// not by Rust handlers. Each tuple contains (name, flags, default, docs).
    /// The default value is optional - `None` means null default.
    #[must_use]
    fn static_properties() -> &'static [(
        &'static str,
        PropertyFlags,
        Option<&'static (dyn IntoZvalDyn + Sync)>,
        DocComments,
    )] {
        &[]
    }
}

/// Stores metadata about a classes Rust constructor, including the function
/// pointer and the arguments of the function.
pub struct ConstructorMeta<T> {
    /// Constructor function.
    pub constructor: fn(&mut ExecuteData) -> ConstructorResult<T>,
    /// Function called to build the constructor function. Usually adds
    /// arguments.
    pub build_fn: fn(FunctionBuilder) -> FunctionBuilder,
    /// Add constructor modification
    pub flags: Option<MethodFlags>,
}

/// Result returned from a constructor of a class.
pub enum ConstructorResult<T> {
    /// Successfully constructed the class, contains the new class object.
    Ok(T),
    /// An exception occurred while constructing the class.
    Exception(PhpException),
    /// Invalid arguments were given to the constructor.
    ArgError,
}

impl<T, E> From<std::result::Result<T, E>> for ConstructorResult<T>
where
    E: Into<PhpException>,
{
    fn from(result: std::result::Result<T, E>) -> Self {
        match result {
            Ok(x) => Self::Ok(x),
            Err(e) => Self::Exception(e.into()),
        }
    }
}

impl<T> From<T> for ConstructorResult<T> {
    fn from(result: T) -> Self {
        Self::Ok(result)
    }
}

/// Stores the class entry and handlers for a Rust type which has been exported
/// to PHP. Usually allocated statically.
pub struct ClassMetadata<T> {
    handlers: OnceCell<ZendObjectHandlers>,
    properties: OnceCell<HashMap<&'static str, PropertyInfo<'static, T>>>,
    ce: AtomicPtr<ClassEntry>,

    // `AtomicPtr` is used here because it is `Send + Sync`.
    // fn() -> T could have been used but that is incompatible with const fns at
    // the moment.
    phantom: PhantomData<AtomicPtr<T>>,
}

impl<T> ClassMetadata<T> {
    /// Creates a new class metadata instance.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            handlers: OnceCell::new(),
            properties: OnceCell::new(),
            ce: AtomicPtr::new(std::ptr::null_mut()),
            phantom: PhantomData,
        }
    }
}

impl<T> Default for ClassMetadata<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: RegisteredClass> ClassMetadata<T> {
    /// Returns an immutable reference to the object handlers contained inside
    /// the class metadata.
    pub fn handlers(&self) -> &ZendObjectHandlers {
        self.handlers.get_or_init(ZendObjectHandlers::new::<T>)
    }

    /// Checks if the class entry has been stored, returning a boolean.
    pub fn has_ce(&self) -> bool {
        !self.ce.load(Ordering::SeqCst).is_null()
    }

    /// Retrieves a reference to the stored class entry.
    ///
    /// # Panics
    ///
    /// Panics if there is no class entry stored inside the class metadata.
    pub fn ce(&self) -> &'static ClassEntry {
        // SAFETY: There are only two values that can be stored in the atomic ptr: null
        // or a static reference to a class entry. On the latter case,
        // `as_ref()` will return `None` and the function will panic.
        unsafe { self.ce.load(Ordering::SeqCst).as_ref() }
            .expect("Attempted to retrieve class entry before it has been stored.")
    }

    /// Stores a reference to a class entry inside the class metadata.
    ///
    /// # Parameters
    ///
    /// * `ce` - The class entry to store.
    ///
    /// # Panics
    ///
    /// Panics if the class entry has already been set in the class metadata.
    /// This function should only be called once.
    pub fn set_ce(&self, ce: &'static mut ClassEntry) {
        self.ce
            .compare_exchange(
                std::ptr::null_mut(),
                ce,
                Ordering::SeqCst,
                Ordering::Relaxed,
            )
            .expect("Class entry has already been set");
    }

    /// Retrieves a reference to the hashmap storing the classes property
    /// accessors.
    ///
    /// # Returns
    ///
    /// Immutable reference to the properties hashmap.
    pub fn get_properties(&self) -> &HashMap<&'static str, PropertyInfo<'static, T>> {
        self.properties.get_or_init(T::get_properties)
    }
}
