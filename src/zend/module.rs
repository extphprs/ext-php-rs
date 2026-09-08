//! Builder and objects for creating modules in PHP. A module is the base of a
//! PHP extension.

use std::cell::UnsafeCell;
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::sync::Once;

use crate::args::ArgInfo;
use crate::ffi::zend_module_entry;
use crate::zend::FunctionEntry;

/// A Zend module entry, also known as an extension.
pub type ModuleEntry = zend_module_entry;

/// The heap allocations a [`ModuleEntry`] points into.
///
/// A C extension puts its `zend_function_entry[]`, name and version in
/// `.rodata`, so the linker owns them for the life of the process. ext-php-rs
/// builds them at MINIT from an arbitrary `#[php_module]` body, so this struct
/// owns them instead, stored inside the extension's own [`StaticModuleEntry`].
///
/// None of it can be freed. `module_destructor` walks `module->functions`
/// reading `fname` *after* MSHUTDOWN for `dl()`-loaded modules,
/// `get_extension_funcs()` walks it during a request, and
/// `zend_register_functions` keeps `zend_internal_function.arg_info` pointing
/// into `arg_info` for the whole process.
pub struct ModuleAllocations {
    /// The module's function table, NUL-terminated.
    pub functions: Box<[FunctionEntry]>,
    /// Argument info of each module function, one table per function.
    pub arg_info: Box<[Box<[ArgInfo]>]>,
    /// The module name handed to PHP.
    pub name: CString,
    /// The module version handed to PHP.
    pub version: CString,
}

/// Static storage for a [`ModuleEntry`] and everything it points into.
///
/// Mimics how C extensions declare a `static zend_module_entry` alongside a
/// `static zend_function_entry[]`: the extension owns its own tables, and
/// nothing is shared between extensions.
pub struct StaticModuleEntry {
    init: Once,
    inner: UnsafeCell<MaybeUninit<ModuleEntry>>,
    owned: UnsafeCell<MaybeUninit<ModuleAllocations>>,
}

// SAFETY: both cells are written exactly once, inside `Once::call_once`, before
// PHP has spawned any request thread, and are never read back through this
// handle afterwards. The raw pointers they hold are handed to the engine, which
// only reads through them.
unsafe impl Sync for StaticModuleEntry {}

impl Default for StaticModuleEntry {
    fn default() -> Self {
        Self::new()
    }
}

impl StaticModuleEntry {
    /// Creates a new uninitialized static module entry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            init: Once::new(),
            inner: UnsafeCell::new(MaybeUninit::uninit()),
            owned: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    /// Initialises the entry on first call, returning a stable `*mut` pointer.
    ///
    /// `f` returns the entry together with the allocations it points into; both
    /// are stored here for the life of the process. Subsequent calls skip `f`
    /// and return the same pointer.
    pub fn get_or_init(
        &self,
        f: impl FnOnce() -> (ModuleEntry, ModuleAllocations),
    ) -> *mut ModuleEntry {
        self.init.call_once(|| {
            let (entry, owned) = f();
            unsafe {
                (*self.owned.get()).write(owned);
                (*self.inner.get()).write(entry);
            }
        });
        unsafe { (*self.inner.get()).as_mut_ptr() }
    }
}
