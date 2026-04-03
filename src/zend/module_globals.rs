use std::cell::UnsafeCell;
use std::marker::PhantomData;
#[cfg_attr(php_zts, allow(unused_imports))]
use std::mem::MaybeUninit;

/// Trait for types used as PHP module globals.
///
/// Requires [`Default`] for initialization. Override [`ginit`](ModuleGlobal::ginit)
/// and [`gshutdown`](ModuleGlobal::gshutdown) for custom per-thread (ZTS) or
/// per-module (non-ZTS) lifecycle logic.
///
/// # Examples
///
/// ```
/// use ext_php_rs::zend::ModuleGlobal;
///
/// #[derive(Default)]
/// struct MyGlobals {
///     request_count: i64,
///     max_depth: i32,
/// }
///
/// impl ModuleGlobal for MyGlobals {
///     fn ginit(&mut self) {
///         self.max_depth = 512;
///     }
/// }
/// ```
pub trait ModuleGlobal: Default + 'static {
    /// Called after the struct is initialized with [`Default::default()`].
    ///
    /// Use for setup that goes beyond what `Default` can express.
    /// In ZTS mode, called once per thread. In non-ZTS mode, called once at module init.
    fn ginit(&mut self) {}

    /// Called before the struct is dropped.
    ///
    /// Use for cleanup of external resources.
    /// In ZTS mode, called once per thread. In non-ZTS mode, called once at module shutdown.
    fn gshutdown(&mut self) {}
}

unsafe extern "C" {
    #[cfg(php_zts)]
    fn ext_php_rs_tsrmg_bulk(id: i32) -> *mut std::ffi::c_void;
}

/// Thread-safe handle to PHP module globals.
///
/// Declare as a `static` and pass to [`ModuleBuilder::globals()`](crate::builders::ModuleBuilder::globals).
///
/// In ZTS (thread-safe) builds, PHP's TSRM allocates per-thread storage and
/// manages the lifetime via GINIT/GSHUTDOWN callbacks. In non-ZTS builds, the
/// globals live inline in this struct as a plain static.
///
/// # Examples
///
/// ```
/// use ext_php_rs::zend::{ModuleGlobal, ModuleGlobals};
///
/// #[derive(Default)]
/// struct MyGlobals {
///     counter: i64,
/// }
///
/// impl ModuleGlobal for MyGlobals {}
///
/// static MY_GLOBALS: ModuleGlobals<MyGlobals> = ModuleGlobals::new();
/// ```
pub struct ModuleGlobals<T: ModuleGlobal> {
    #[cfg(php_zts)]
    id: UnsafeCell<i32>,
    #[cfg(not(php_zts))]
    inner: UnsafeCell<MaybeUninit<T>>,
    _marker: PhantomData<T>,
}

// SAFETY: In ZTS mode, TSRM guarantees per-thread access. The `id` field is
// only written once during single-threaded module init (MINIT).
// In non-ZTS mode, PHP is single-threaded.
unsafe impl<T: ModuleGlobal> Sync for ModuleGlobals<T> {}

impl<T: ModuleGlobal> Default for ModuleGlobals<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ModuleGlobal> ModuleGlobals<T> {
    /// Creates an uninitialized globals handle.
    ///
    /// Must be passed to [`ModuleBuilder::globals()`](crate::builders::ModuleBuilder::globals)
    /// for PHP to allocate and initialize the storage.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            #[cfg(php_zts)]
            id: UnsafeCell::new(0),
            #[cfg(not(php_zts))]
            inner: UnsafeCell::new(MaybeUninit::uninit()),
            _marker: PhantomData,
        }
    }

    /// Returns a shared reference to the current thread's globals.
    ///
    /// Safe because PHP guarantees single-threaded request processing:
    /// only one request handler runs per thread at a time, and module
    /// globals are initialized before any request begins.
    ///
    /// # Panics
    ///
    /// Debug-asserts that the globals have been registered. In release builds,
    /// calling this before module init is undefined behavior.
    pub fn get(&self) -> &T {
        unsafe {
            #[cfg(php_zts)]
            {
                let id = *self.id.get();
                debug_assert!(id != 0, "ModuleGlobals accessed before registration");
                &*ext_php_rs_tsrmg_bulk(id).cast::<T>()
            }
            #[cfg(not(php_zts))]
            {
                (*self.inner.get()).assume_init_ref()
            }
        }
    }

    /// Returns a mutable reference to the current thread's globals.
    ///
    /// # Safety
    ///
    /// Caller must ensure exclusive access. Typically safe within `RINIT`/`RSHUTDOWN`
    /// or from a `#[php_function]` handler (PHP runs one request per thread), but
    /// NOT from background Rust threads.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get_mut(&self) -> &mut T {
        #[cfg(php_zts)]
        unsafe {
            let id = *self.id.get();
            debug_assert!(id != 0, "ModuleGlobals accessed before registration");
            &mut *ext_php_rs_tsrmg_bulk(id).cast::<T>()
        }
        #[cfg(not(php_zts))]
        unsafe {
            (*self.inner.get()).assume_init_mut()
        }
    }

    /// Returns a raw pointer to the globals for the current thread.
    ///
    /// Escape hatch for power users who need direct access without
    /// lifetime constraints.
    pub fn as_ptr(&self) -> *mut T {
        unsafe {
            #[cfg(php_zts)]
            {
                ext_php_rs_tsrmg_bulk(*self.id.get()).cast::<T>()
            }
            #[cfg(not(php_zts))]
            {
                (*self.inner.get()).as_mut_ptr()
            }
        }
    }

    /// Returns a pointer to the internal ID storage (ZTS) or data storage (non-ZTS).
    ///
    /// Used by [`ModuleBuilder::globals()`](crate::builders::ModuleBuilder::globals)
    /// to wire up the `zend_module_entry`.
    #[cfg(php_zts)]
    pub(crate) fn id_ptr(&self) -> *mut i32 {
        self.id.get()
    }

    /// Returns a pointer to the internal storage.
    ///
    /// Used by [`ModuleBuilder::globals()`](crate::builders::ModuleBuilder::globals)
    /// to wire up the `zend_module_entry`.
    #[cfg(not(php_zts))]
    pub(crate) fn data_ptr(&self) -> *mut std::ffi::c_void {
        self.inner.get().cast()
    }
}

/// GINIT callback invoked by PHP per-thread (ZTS) or once (non-ZTS).
///
/// # Safety
///
/// `globals` must point to uninitialized memory of at least `size_of::<T>()` bytes.
/// Called by PHP's module initialization machinery.
pub(crate) unsafe extern "C" fn ginit_callback<T: ModuleGlobal>(globals: *mut std::ffi::c_void) {
    unsafe {
        let ptr = globals.cast::<T>();
        ptr.write(T::default());
        (*ptr).ginit();
    }
}

/// GSHUTDOWN callback invoked by PHP before freeing globals memory.
///
/// # Safety
///
/// `globals` must point to a valid, initialized `T`.
/// Called by PHP's module shutdown machinery.
pub(crate) unsafe extern "C" fn gshutdown_callback<T: ModuleGlobal>(
    globals: *mut std::ffi::c_void,
) {
    unsafe {
        let ptr = globals.cast::<T>();
        (*ptr).gshutdown();
        std::ptr::drop_in_place(ptr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct TestGlobals {
        value: i32,
        initialized: bool,
    }

    impl ModuleGlobal for TestGlobals {
        fn ginit(&mut self) {
            self.initialized = true;
            self.value = 42;
        }

        fn gshutdown(&mut self) {
            self.initialized = false;
        }
    }

    #[test]
    fn new_is_const() {
        static _G: ModuleGlobals<TestGlobals> = ModuleGlobals::new();
    }

    #[test]
    fn ginit_callback_initializes() {
        let mut storage = MaybeUninit::<TestGlobals>::uninit();
        unsafe {
            ginit_callback::<TestGlobals>(storage.as_mut_ptr().cast());
            let globals = storage.assume_init_ref();
            assert!(globals.initialized);
            assert_eq!(globals.value, 42);
            std::ptr::drop_in_place(storage.as_mut_ptr());
        }
    }

    #[test]
    fn gshutdown_callback_cleans_up() {
        let mut storage = MaybeUninit::<TestGlobals>::uninit();
        unsafe {
            ginit_callback::<TestGlobals>(storage.as_mut_ptr().cast());
            gshutdown_callback::<TestGlobals>(storage.as_mut_ptr().cast());
        }
    }

    #[test]
    #[cfg(not(php_zts))]
    fn non_zts_get_after_init() {
        let globals: ModuleGlobals<TestGlobals> = ModuleGlobals::new();
        unsafe {
            ginit_callback::<TestGlobals>(globals.data_ptr());
        }
        assert!(globals.get().initialized);
        assert_eq!(globals.get().value, 42);
        unsafe {
            gshutdown_callback::<TestGlobals>(globals.data_ptr());
        }
    }

    #[test]
    #[cfg(not(php_zts))]
    fn non_zts_get_mut() {
        let globals: ModuleGlobals<TestGlobals> = ModuleGlobals::new();
        unsafe {
            ginit_callback::<TestGlobals>(globals.data_ptr());
            globals.get_mut().value = 99;
        }
        assert_eq!(globals.get().value, 99);
        unsafe {
            gshutdown_callback::<TestGlobals>(globals.data_ptr());
        }
    }

    #[test]
    #[cfg(not(php_zts))]
    fn non_zts_as_ptr() {
        let globals: ModuleGlobals<TestGlobals> = ModuleGlobals::new();
        unsafe {
            ginit_callback::<TestGlobals>(globals.data_ptr());
        }
        let ptr = globals.as_ptr();
        assert_eq!(unsafe { (*ptr).value }, 42);
        unsafe {
            gshutdown_callback::<TestGlobals>(globals.data_ptr());
        }
    }

    #[derive(Default)]
    struct ZstGlobals;
    impl ModuleGlobal for ZstGlobals {}

    #[test]
    fn zst_size_is_zero() {
        assert_eq!(std::mem::size_of::<ZstGlobals>(), 0);
    }
}
