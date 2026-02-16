//! Hotload ABI - shared interface between host and loaded modules
//!
//! Modules must export these symbols:
//! - `hotload_info() -> *const PluginInfo`
//! - `hotload_init()`

use std::ffi::c_char;

/// Plugin metadata
#[repr(C)]
pub struct PluginInfo {
    /// Plugin name (null-terminated)
    pub name: *const c_char,
    /// Plugin version (null-terminated)
    pub version: *const c_char,
    /// Number of functions exported
    pub num_functions: u32,
    /// Array of function descriptors
    pub functions: *const FunctionInfo,
}

/// Function metadata (just the name for cleanup purposes)
#[repr(C)]
pub struct FunctionInfo {
    /// Function name as it appears in PHP (null-terminated)
    pub name: *const c_char,
}

/// Type for the plugin info function
pub type PluginInfoFn = unsafe extern "C" fn() -> *const PluginInfo;

/// Type for the plugin init function (called after loading)
pub type PluginInitFn = unsafe extern "C" fn();

/// ABI version for compatibility checking
#[allow(dead_code)]
pub const ABI_VERSION: u32 = 1;
