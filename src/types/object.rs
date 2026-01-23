//! Represents an object in PHP. Allows for overriding the internal object used
//! by classes, allowing users to store Rust data inside a PHP object.
//!
//! # Lazy Objects (PHP 8.4+)
//!
//! PHP 8.4 introduced lazy objects, which defer their initialization until
//! their properties are first accessed. This module provides introspection APIs
//! for lazy objects:
//!
//! - [`ZendObject::is_lazy()`] - Check if an object is lazy (ghost or proxy)
//! - [`ZendObject::is_lazy_ghost()`] - Check if an object is a lazy ghost
//! - [`ZendObject::is_lazy_proxy()`] - Check if an object is a lazy proxy
//! - [`ZendObject::is_lazy_initialized()`] - Check if a lazy object has been initialized
//! - [`ZendObject::lazy_init()`] - Trigger initialization of a lazy object
//!
//! ## Lazy Ghosts vs Lazy Proxies
//!
//! - **Lazy Ghosts**: The ghost object itself becomes the real instance when
//!   initialized. After initialization, the ghost is indistinguishable from a
//!   regular object (the `is_lazy()` flag is cleared).
//!
//! - **Lazy Proxies**: A proxy wraps a real instance that is created when first
//!   accessed. The proxy and real instance have different identities. After
//!   initialization, the proxy still reports as lazy (`is_lazy()` returns true).
//!
//! ## Creating Lazy Objects
//!
//! Lazy objects should be created using PHP's `ReflectionClass` API:
//!
//! ```php
//! <?php
//! // Create a lazy ghost
//! $reflector = new ReflectionClass(MyClass::class);
//! $ghost = $reflector->newLazyGhost(function ($obj) {
//!     $obj->__construct('initialized');
//! });
//!
//! // Create a lazy proxy
//! $proxy = $reflector->newLazyProxy(function ($obj) {
//!     return new MyClass('initialized');
//! });
//! ```
//!
//! **Note**: PHP 8.4 lazy objects only work with user-defined PHP classes, not
//! internal classes. Since Rust-defined classes (using `#[php_class]`) are
//! registered as internal classes, they cannot be made lazy using PHP's
//! Reflection API.

use std::{convert::TryInto, fmt::Debug, os::raw::c_char, ptr};

use crate::{
    boxed::{ZBox, ZBoxable},
    class::RegisteredClass,
    convert::{FromZendObject, FromZval, FromZvalMut, IntoZval, IntoZvalDyn},
    error::{Error, Result},
    ffi::{
        HashTable, ZEND_ISEMPTY, ZEND_PROPERTY_EXISTS, ZEND_PROPERTY_ISSET,
        ext_php_rs_zend_object_release, object_properties_init, zend_call_known_function,
        zend_function, zend_hash_str_find_ptr_lc, zend_object, zend_objects_new,
    },
    flags::DataType,
    rc::PhpRc,
    types::{ZendClassObject, ZendStr, Zval},
    zend::{ClassEntry, ExecutorGlobals, ZendObjectHandlers, ce},
};

#[cfg(php84)]
use crate::ffi::{zend_lazy_object_init, zend_lazy_object_mark_as_initialized};

#[cfg(all(feature = "closure", php84))]
use crate::{
    closure::Closure,
    ffi::{
        _zend_fcall_info_cache, ZEND_LAZY_OBJECT_STRATEGY_GHOST, ZEND_LAZY_OBJECT_STRATEGY_PROXY,
        zend_is_callable_ex, zend_object_make_lazy,
    },
};

/// A PHP object.
///
/// This type does not maintain any information about its type, for example,
/// classes with have associated Rust structs cannot be accessed through this
/// type. [`ZendClassObject`] is used for this purpose, and you can convert
/// between the two.
pub type ZendObject = zend_object;

impl ZendObject {
    /// Creates a new [`ZendObject`], returned inside an [`ZBox<ZendObject>`]
    /// wrapper.
    ///
    /// # Parameters
    ///
    /// * `ce` - The type of class the new object should be an instance of.
    ///
    /// # Panics
    ///
    /// Panics when allocating memory for the new object fails.
    #[must_use]
    pub fn new(ce: &ClassEntry) -> ZBox<Self> {
        // SAFETY: Using emalloc to allocate memory inside Zend arena. Casting `ce` to
        // `*mut` is valid as the function will not mutate `ce`.
        unsafe {
            let ptr = match ce.__bindgen_anon_2.create_object {
                None => {
                    let ptr = zend_objects_new(ptr::from_ref(ce).cast_mut());
                    assert!(!ptr.is_null(), "Failed to allocate memory for Zend object");

                    object_properties_init(ptr, ptr::from_ref(ce).cast_mut());
                    ptr
                }
                Some(v) => v(ptr::from_ref(ce).cast_mut()),
            };

            ZBox::from_raw(
                ptr.as_mut()
                    .expect("Failed to allocate memory for Zend object"),
            )
        }
    }

    /// Creates a new `stdClass` instance, returned inside an
    /// [`ZBox<ZendObject>`] wrapper.
    ///
    /// # Panics
    ///
    /// Panics if allocating memory for the object fails, or if the `stdClass`
    /// class entry has not been registered with PHP yet.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use ext_php_rs::types::ZendObject;
    ///
    /// let mut obj = ZendObject::new_stdclass();
    ///
    /// obj.set_property("hello", "world");
    /// ```
    #[must_use]
    pub fn new_stdclass() -> ZBox<Self> {
        // SAFETY: This will be `NULL` until it is initialized. `as_ref()` checks for
        // null, so we can panic if it's null.
        Self::new(ce::stdclass())
    }

    /// Converts a class object into an owned [`ZendObject`]. This removes any
    /// possibility of accessing the underlying attached Rust struct.
    #[must_use]
    pub fn from_class_object<T: RegisteredClass>(obj: ZBox<ZendClassObject<T>>) -> ZBox<Self> {
        let this = obj.into_raw();
        // SAFETY: Consumed box must produce a well-aligned non-null pointer.
        unsafe { ZBox::from_raw(this.get_mut_zend_obj()) }
    }

    /// Returns the [`ClassEntry`] associated with this object.
    ///
    /// # Panics
    ///
    /// Panics if the class entry is invalid.
    #[must_use]
    pub fn get_class_entry(&self) -> &'static ClassEntry {
        // SAFETY: it is OK to panic here since PHP would segfault anyway
        // when encountering an object with no class entry.
        unsafe { self.ce.as_ref() }.expect("Could not retrieve class entry.")
    }

    /// Attempts to retrieve the class name of the object.
    ///
    /// # Errors
    ///
    /// * `Error::InvalidScope` - If the object handlers or the class name
    ///   cannot be retrieved.
    pub fn get_class_name(&self) -> Result<String> {
        unsafe {
            self.handlers()?
                .get_class_name
                .and_then(|f| f(self).as_ref())
                .ok_or(Error::InvalidScope)
                .and_then(TryInto::try_into)
        }
    }

    /// Returns whether this object is an instance of the given [`ClassEntry`].
    ///
    /// This method checks the class and interface inheritance chain.
    ///
    /// # Panics
    ///
    /// Panics if the class entry is invalid.
    #[must_use]
    pub fn instance_of(&self, ce: &ClassEntry) -> bool {
        self.get_class_entry().instance_of(ce)
    }

    /// Checks if the given object is an instance of a registered class with
    /// Rust type `T`.
    ///
    /// This method doesn't check the class and interface inheritance chain.
    #[must_use]
    pub fn is_instance<T: RegisteredClass>(&self) -> bool {
        (self.ce.cast_const()).eq(&ptr::from_ref(T::get_metadata().ce()))
    }

    /// Returns whether this object is an instance of \Traversable
    ///
    /// # Panics
    ///
    /// Panics if the class entry is invalid.
    #[must_use]
    pub fn is_traversable(&self) -> bool {
        self.instance_of(ce::traversable())
    }

    /// Tries to call a method on the object.
    ///
    /// # Returns
    ///
    /// Returns the return value of the method, or an error if the method
    /// could not be found or called.
    ///
    /// # Errors
    ///
    /// * `Error::Callable` - If the method could not be found.
    /// * If a parameter could not be converted to a zval.
    /// * If the parameter count is bigger than `u32::MAX`.
    // TODO: Measure this
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub fn try_call_method(&self, name: &str, params: Vec<&dyn IntoZvalDyn>) -> Result<Zval> {
        let mut retval = Zval::new();
        let len = params.len();
        let params = params
            .into_iter()
            .map(|val| val.as_zval(false))
            .collect::<Result<Vec<_>>>()?;
        let packed = params.into_boxed_slice();

        unsafe {
            let res = zend_hash_str_find_ptr_lc(
                &raw const (*self.ce).function_table,
                name.as_ptr().cast::<c_char>(),
                name.len(),
            )
            .cast::<zend_function>();

            if res.is_null() {
                return Err(Error::Callable);
            }

            zend_call_known_function(
                res,
                ptr::from_ref(self).cast_mut(),
                self.ce,
                &raw mut retval,
                len.try_into()?,
                packed.as_ptr().cast_mut(),
                std::ptr::null_mut(),
            );
        };

        Ok(retval)
    }

    /// Attempts to read a property from the Object. Returns a result containing
    /// the value of the property if it exists and can be read, and an
    /// [`Error`] otherwise.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the property.
    /// * `query` - The type of query to use when attempting to get a property.
    ///
    /// # Errors
    ///
    /// * `Error::InvalidScope` - If the object handlers or the properties
    ///   cannot be retrieved.
    pub fn get_property<'a, T>(&'a self, name: &str) -> Result<T>
    where
        T: FromZval<'a>,
    {
        if !self.has_property(name, PropertyQuery::Exists)? {
            return Err(Error::InvalidProperty);
        }

        let mut name = ZendStr::new(name, false);
        let mut rv = Zval::new();

        let zv = unsafe {
            self.handlers()?.read_property.ok_or(Error::InvalidScope)?(
                self.mut_ptr(),
                &raw mut *name,
                1,
                ptr::null_mut(),
                &raw mut rv,
            )
            .as_ref()
        }
        .ok_or(Error::InvalidScope)?;

        T::from_zval(zv).ok_or_else(|| Error::ZvalConversion(zv.get_type()))
    }

    /// Attempts to set a property on the object.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the property.
    /// * `value` - The value to set the property to.
    ///
    /// # Errors
    ///
    /// * `Error::InvalidScope` - If the object handlers or the properties
    ///   cannot be retrieved.
    pub fn set_property(&mut self, name: &str, value: impl IntoZval) -> Result<()> {
        let mut name = ZendStr::new(name, false);
        let mut value = value.into_zval(false)?;

        unsafe {
            self.handlers()?.write_property.ok_or(Error::InvalidScope)?(
                self,
                &raw mut *name,
                &raw mut value,
                ptr::null_mut(),
            )
            .as_ref()
        }
        .ok_or(Error::InvalidScope)?;
        Ok(())
    }

    /// Checks if a property exists on an object. Takes a property name and
    /// query parameter, which defines what classifies if a property exists
    /// or not. See [`PropertyQuery`] for more information.
    ///
    /// # Parameters
    ///
    /// * `name` - The name of the property.
    /// * `query` - The 'query' to classify if a property exists.
    ///
    /// # Errors
    ///
    /// * `Error::InvalidScope` - If the object handlers or the properties
    ///   cannot be retrieved.
    pub fn has_property(&self, name: &str, query: PropertyQuery) -> Result<bool> {
        let mut name = ZendStr::new(name, false);

        Ok(unsafe {
            self.handlers()?.has_property.ok_or(Error::InvalidScope)?(
                self.mut_ptr(),
                &raw mut *name,
                query as _,
                std::ptr::null_mut(),
            )
        } > 0)
    }

    /// Attempts to retrieve the properties of the object. Returned inside a
    /// Zend Hashtable.
    ///
    /// # Errors
    ///
    /// * `Error::InvalidScope` - If the object handlers or the properties
    ///   cannot be retrieved.
    pub fn get_properties(&self) -> Result<&HashTable> {
        unsafe {
            self.handlers()?
                .get_properties
                .and_then(|props| props(self.mut_ptr()).as_ref())
                .ok_or(Error::InvalidScope)
        }
    }

    /// Extracts some type from a Zend object.
    ///
    /// This is a wrapper function around `FromZendObject::extract()`.
    ///
    /// # Errors
    ///
    /// Returns an error if the conversion fails.
    pub fn extract<'a, T>(&'a self) -> Result<T>
    where
        T: FromZendObject<'a>,
    {
        T::from_zend_object(self)
    }

    /// Returns an unique identifier for the object.
    ///
    /// The id is guaranteed to be unique for the lifetime of the object.
    /// Once the object is destroyed, it may be reused for other objects.
    /// This is equivalent to calling the [`spl_object_id`] PHP function.
    ///
    /// [`spl_object_id`]: https://www.php.net/manual/function.spl-object-id
    #[inline]
    #[must_use]
    pub fn get_id(&self) -> u32 {
        self.handle
    }

    /// Computes an unique hash for the object.
    ///
    /// The hash is guaranteed to be unique for the lifetime of the object.
    /// Once the object is destroyed, it may be reused for other objects.
    /// This is equivalent to calling the [`spl_object_hash`] PHP function.
    ///
    /// [`spl_object_hash`]: https://www.php.net/manual/function.spl-object-hash.php
    #[must_use]
    pub fn hash(&self) -> String {
        format!("{:016x}0000000000000000", self.handle)
    }

    // Object extra_flags constants for lazy object detection.
    // PHP 8.4+ lazy object constants
    // These are checked on zend_object.extra_flags before calling zend_lazy_object_get_flags.
    // IS_OBJ_LAZY_UNINITIALIZED = (1U<<31) - Virtual proxy or uninitialized Ghost
    #[cfg(php84)]
    const IS_OBJ_LAZY_UNINITIALIZED: u32 = 1 << 31;
    // IS_OBJ_LAZY_PROXY = (1U<<30) - Virtual proxy (may be initialized)
    #[cfg(php84)]
    const IS_OBJ_LAZY_PROXY: u32 = 1 << 30;

    /// Returns whether this object is a lazy object (ghost or proxy).
    ///
    /// Lazy objects are objects whose initialization is deferred until
    /// one of their properties is accessed.
    ///
    /// This is a PHP 8.4+ feature.
    #[cfg(php84)]
    #[must_use]
    pub fn is_lazy(&self) -> bool {
        // Check extra_flags directly - safe for all objects
        (self.extra_flags & (Self::IS_OBJ_LAZY_UNINITIALIZED | Self::IS_OBJ_LAZY_PROXY)) != 0
    }

    /// Returns whether this object is a lazy proxy.
    ///
    /// Lazy proxies wrap a real instance that is created when the proxy
    /// is first accessed. The proxy and real instance have different identities.
    ///
    /// This is a PHP 8.4+ feature.
    #[cfg(php84)]
    #[must_use]
    pub fn is_lazy_proxy(&self) -> bool {
        // Check extra_flags directly - safe for all objects
        (self.extra_flags & Self::IS_OBJ_LAZY_PROXY) != 0
    }

    /// Returns whether this object is a lazy ghost.
    ///
    /// Lazy ghosts are indistinguishable from non-lazy objects once initialized.
    /// The ghost object itself becomes the real instance.
    ///
    /// This is a PHP 8.4+ feature.
    #[cfg(php84)]
    #[must_use]
    pub fn is_lazy_ghost(&self) -> bool {
        // A lazy ghost has IS_OBJ_LAZY_UNINITIALIZED set but NOT IS_OBJ_LAZY_PROXY
        (self.extra_flags & Self::IS_OBJ_LAZY_UNINITIALIZED) != 0
            && (self.extra_flags & Self::IS_OBJ_LAZY_PROXY) == 0
    }

    /// Returns whether this lazy object has been initialized.
    ///
    /// Returns `false` for non-lazy objects.
    ///
    /// This is a PHP 8.4+ feature.
    #[cfg(php84)]
    #[must_use]
    pub fn is_lazy_initialized(&self) -> bool {
        if !self.is_lazy() {
            return false;
        }
        // A lazy object is initialized when IS_OBJ_LAZY_UNINITIALIZED is NOT set.
        // For ghosts: both flags clear when initialized
        // For proxies: IS_OBJ_LAZY_PROXY stays but IS_OBJ_LAZY_UNINITIALIZED clears
        (self.extra_flags & Self::IS_OBJ_LAZY_UNINITIALIZED) == 0
    }

    /// Triggers initialization of a lazy object.
    ///
    /// If the object is a lazy ghost, this populates the object in place.
    /// If the object is a lazy proxy, this creates the real instance.
    ///
    /// Returns `None` if the object is not lazy or initialization fails.
    ///
    /// This is a PHP 8.4+ feature.
    #[cfg(php84)]
    #[must_use]
    pub fn lazy_init(&mut self) -> Option<&mut Self> {
        if !self.is_lazy() {
            return None;
        }
        unsafe { zend_lazy_object_init(self).as_mut() }
    }

    /// Marks a lazy object as initialized without calling the initializer.
    ///
    /// This can be used to manually initialize a lazy object's properties
    /// and then mark it as initialized.
    ///
    /// Returns `None` if the object is not lazy.
    ///
    /// This is a PHP 8.4+ feature.
    #[cfg(php84)]
    #[must_use]
    pub fn mark_lazy_initialized(&mut self) -> Option<&mut Self> {
        if !self.is_lazy() {
            return None;
        }
        unsafe { zend_lazy_object_mark_as_initialized(self).as_mut() }
    }

    /// For lazy proxies, returns the real instance after initialization.
    ///
    /// Returns `None` if this is not a lazy proxy or if not initialized.
    ///
    /// This is a PHP 8.4+ feature.
    #[cfg(php84)]
    #[must_use]
    pub fn lazy_get_instance(&mut self) -> Option<&mut Self> {
        if !self.is_lazy_proxy() || !self.is_lazy_initialized() {
            return None;
        }
        // Note: We use zend_lazy_object_init here because zend_lazy_object_get_instance
        // is not exported (no ZEND_API) in PHP and cannot be linked on Windows.
        // zend_lazy_object_init returns the real instance for already-initialized proxies.
        unsafe { zend_lazy_object_init(self).as_mut() }
    }

    /// Converts this object into a lazy ghost with the given initializer.
    ///
    /// The initializer closure will be called when the object's properties are
    /// first accessed. The closure should perform initialization logic.
    ///
    /// # Parameters
    ///
    /// * `initializer` - A closure that performs initialization. The closure
    ///   returns `()`. Any state needed for initialization should be captured
    ///   in the closure.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the object was successfully made lazy, or an error
    /// if the operation failed.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ext_php_rs::types::ZendObject;
    ///
    /// fn make_lazy_example(obj: &mut ZendObject) -> ext_php_rs::error::Result<()> {
    ///     let init_value = "initialized".to_string();
    ///     obj.make_lazy_ghost(Box::new(move || {
    ///         // Use captured state for initialization
    ///         println!("Initializing with: {}", init_value);
    ///     }) as Box<dyn Fn()>)?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the initializer closure cannot be converted to a
    /// PHP callable or if the object cannot be made lazy.
    ///
    /// # Safety
    ///
    /// This is a PHP 8.4+ feature. The closure must be `'static` as it may be
    /// called at any time during the object's lifetime.
    ///
    /// **Note**: PHP 8.4 lazy objects only work with user-defined PHP classes,
    /// not internal classes. Rust-defined classes cannot be made lazy.
    #[cfg(all(feature = "closure", php84))]
    #[cfg_attr(docs, doc(cfg(all(feature = "closure", php84))))]
    #[allow(clippy::cast_possible_truncation)]
    pub fn make_lazy_ghost<F>(&mut self, initializer: F) -> Result<()>
    where
        F: Fn() + 'static,
    {
        self.make_lazy_internal(initializer, ZEND_LAZY_OBJECT_STRATEGY_GHOST as u8)
    }

    /// Converts this object into a lazy proxy with the given initializer.
    ///
    /// The initializer closure will be called when the object's properties are
    /// first accessed. The closure should return the real instance that the
    /// proxy will forward to.
    ///
    /// # Parameters
    ///
    /// * `initializer` - A closure that returns `Option<ZBox<ZendObject>>`,
    ///   the real instance. Any state needed should be captured in the closure.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the object was successfully made lazy, or an error
    /// if the operation failed.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ext_php_rs::types::ZendObject;
    /// use ext_php_rs::boxed::ZBox;
    ///
    /// fn make_proxy_example(obj: &mut ZendObject) -> ext_php_rs::error::Result<()> {
    ///     obj.make_lazy_proxy(Box::new(|| {
    ///         // Create and return the real instance
    ///         Some(ZendObject::new_stdclass())
    ///     }) as Box<dyn Fn() -> Option<ZBox<ZendObject>>>)?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the initializer closure cannot be converted to a
    /// PHP callable or if the object cannot be made lazy.
    ///
    /// # Safety
    ///
    /// This is a PHP 8.4+ feature. The closure must be `'static` as it may be
    /// called at any time during the object's lifetime.
    ///
    /// **Note**: PHP 8.4 lazy objects only work with user-defined PHP classes,
    /// not internal classes. Rust-defined classes cannot be made lazy.
    #[cfg(all(feature = "closure", php84))]
    #[cfg_attr(docs, doc(cfg(all(feature = "closure", php84))))]
    #[allow(clippy::cast_possible_truncation)]
    pub fn make_lazy_proxy<F>(&mut self, initializer: F) -> Result<()>
    where
        F: Fn() -> Option<ZBox<ZendObject>> + 'static,
    {
        self.make_lazy_internal(initializer, ZEND_LAZY_OBJECT_STRATEGY_PROXY as u8)
    }

    /// Internal implementation for making an object lazy.
    #[cfg(all(feature = "closure", php84))]
    fn make_lazy_internal<F, R>(&mut self, initializer: F, strategy: u8) -> Result<()>
    where
        F: Fn() -> R + 'static,
        R: IntoZval + 'static,
    {
        // Check if the class can be made lazy
        let ce = unsafe { self.ce.as_ref() }.ok_or(Error::InvalidPointer)?;
        if !ce.can_be_lazy() {
            return Err(Error::LazyObjectFailed);
        }

        // Cannot make an already-lazy uninitialized object lazy again
        if self.is_lazy() && !self.is_lazy_initialized() {
            return Err(Error::LazyObjectFailed);
        }

        // Wrap the Rust closure in a PHP-callable Closure
        let closure = Closure::wrap(Box::new(initializer) as Box<dyn Fn() -> R>);

        // Convert the closure to a zval
        let mut initializer_zv = Zval::new();
        closure.set_zval(&mut initializer_zv, false)?;

        // Initialize the fcc structure
        let mut fcc: _zend_fcall_info_cache = unsafe { std::mem::zeroed() };

        // Populate the fcc using zend_is_callable_ex
        let is_callable = unsafe {
            zend_is_callable_ex(
                &raw mut initializer_zv,
                ptr::null_mut(),
                0,
                ptr::null_mut(),
                &raw mut fcc,
                ptr::null_mut(),
            )
        };

        if !is_callable {
            return Err(Error::Callable);
        }

        // Get the class entry
        let ce = self.ce;

        // Make the object lazy
        let result = unsafe {
            zend_object_make_lazy(self, ce, &raw mut initializer_zv, &raw mut fcc, strategy)
        };

        if result.is_null() {
            Err(Error::LazyObjectFailed)
        } else {
            Ok(())
        }
    }

    /// Attempts to retrieve a reference to the object handlers.
    #[inline]
    unsafe fn handlers(&self) -> Result<&ZendObjectHandlers> {
        unsafe { self.handlers.as_ref() }.ok_or(Error::InvalidScope)
    }

    /// Returns a mutable pointer to `self`, regardless of the type of
    /// reference. Only to be used in situations where a C function requires
    /// a mutable pointer but does not modify the underlying data.
    #[inline]
    fn mut_ptr(&self) -> *mut Self {
        ptr::from_ref(self).cast_mut()
    }
}

unsafe impl ZBoxable for ZendObject {
    fn free(&mut self) {
        unsafe { ext_php_rs_zend_object_release(self) }
    }
}

impl Debug for ZendObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct(
            self.get_class_name()
                .unwrap_or_else(|_| "ZendObject".to_string())
                .as_str(),
        );

        if let Ok(props) = self.get_properties() {
            for (key, val) in props {
                dbg.field(key.to_string().as_str(), val);
            }
        }

        dbg.finish()
    }
}

impl<'a> FromZval<'a> for &'a ZendObject {
    const TYPE: DataType = DataType::Object(None);

    fn from_zval(zval: &'a Zval) -> Option<Self> {
        zval.object()
    }
}

impl<'a> FromZvalMut<'a> for &'a mut ZendObject {
    const TYPE: DataType = DataType::Object(None);

    fn from_zval_mut(zval: &'a mut Zval) -> Option<Self> {
        zval.object_mut()
    }
}

impl IntoZval for ZBox<ZendObject> {
    const TYPE: DataType = DataType::Object(None);
    const NULLABLE: bool = false;

    #[inline]
    fn set_zval(mut self, zv: &mut Zval, _: bool) -> Result<()> {
        // We must decrement the refcounter on the object before inserting into the
        // zval, as the reference counter will be incremented on add.
        // NOTE(david): again is this needed, we increment in `set_object`.
        self.dec_count();
        zv.set_object(self.into_raw());
        Ok(())
    }
}

impl IntoZval for &mut ZendObject {
    const TYPE: DataType = DataType::Object(None);
    const NULLABLE: bool = false;

    #[inline]
    fn set_zval(self, zv: &mut Zval, _: bool) -> Result<()> {
        zv.set_object(self);
        Ok(())
    }
}

impl FromZendObject<'_> for String {
    fn from_zend_object(obj: &ZendObject) -> Result<Self> {
        let mut ret = Zval::new();
        unsafe {
            zend_call_known_function(
                (*obj.ce).__tostring,
                ptr::from_ref(obj).cast_mut(),
                obj.ce,
                &raw mut ret,
                0,
                ptr::null_mut(),
                ptr::null_mut(),
            );
        }

        if let Some(err) = ExecutorGlobals::take_exception() {
            // TODO: become an error
            let class_name = obj.get_class_name();
            panic!(
                "Uncaught exception during call to {}::__toString(): {:?}",
                class_name.expect("unable to determine class name"),
                err
            );
        } else if let Some(output) = ret.extract() {
            Ok(output)
        } else {
            // TODO: become an error
            let class_name = obj.get_class_name();
            panic!(
                "{}::__toString() must return a string",
                class_name.expect("unable to determine class name"),
            );
        }
    }
}

impl<T: RegisteredClass> From<ZBox<ZendClassObject<T>>> for ZBox<ZendObject> {
    #[inline]
    fn from(obj: ZBox<ZendClassObject<T>>) -> Self {
        ZendObject::from_class_object(obj)
    }
}

/// Different ways to query if a property exists.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum PropertyQuery {
    /// Property exists and is not NULL.
    Isset = ZEND_PROPERTY_ISSET,
    /// Property is not empty.
    NotEmpty = ZEND_ISEMPTY,
    /// Property exists.
    Exists = ZEND_PROPERTY_EXISTS,
}
