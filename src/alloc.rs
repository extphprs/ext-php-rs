//! Functions relating to the Zend Memory Manager, used to allocate
//! request-bound memory.

use cfg_if::cfg_if;

use crate::ffi::{_efree, _emalloc, _estrdup, ZEND_MM_ALIGNMENT};
use std::{
    alloc::Layout,
    ffi::{CString, c_char, c_void},
};

/// Uses the PHP memory allocator to allocate request-bound memory.
///
/// The Zend allocator only guarantees `ZEND_MM_ALIGNMENT` (8 bytes) and has
/// no aligned-allocation entry point, so a `layout` requesting a stricter
/// alignment cannot be honoured and yields a null pointer.
///
/// # Parameters
///
/// * `layout` - The layout of the requested memory.
///
/// # Returns
///
/// A pointer to the memory allocated, or null if `layout.align()` exceeds
/// `ZEND_MM_ALIGNMENT`.
#[must_use]
pub fn emalloc(layout: Layout) -> *mut u8 {
    if layout.align() > ZEND_MM_ALIGNMENT.unsigned_abs() {
        return std::ptr::null_mut();
    }
    let size = layout.size();

    (unsafe {
        cfg_if! {
            if #[cfg(php_debug)] {
                #[allow(clippy::used_underscore_items)]
                _emalloc(size as _, std::ptr::null_mut(), 0, std::ptr::null_mut(), 0)
            } else {
                #[allow(clippy::used_underscore_items)]
                _emalloc(size as _)
            }
        }
    })
    .cast::<u8>()
}

/// Frees a given memory pointer which was allocated through the PHP memory
/// manager.
///
/// # Parameters
///
/// * `ptr` - The pointer to the memory to free.
///
/// # Safety
///
/// Caller must guarantee that the given pointer is valid (aligned and non-null)
/// and was originally allocated through the Zend memory manager.
pub unsafe fn efree(ptr: *mut u8) {
    cfg_if! {
        if #[cfg(php_debug)] {
            #[allow(clippy::used_underscore_items)]
            unsafe {
                _efree(
                    ptr.cast::<c_void>(),
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    0,
                );
            }
        } else {
            #[allow(clippy::used_underscore_items)]
            unsafe {
                _efree(ptr.cast::<c_void>());
            }
        }
    }
}

/// Duplicates a string using the PHP memory manager.
///
/// # Parameters
///
/// * `string` - The string to duplicate, which can be any type that can be
///   converted into a `Vec<u8>`.
///
/// # Returns
///
/// A pointer to the duplicated string in the PHP memory manager.
pub fn estrdup(string: impl Into<Vec<u8>>) -> *mut c_char {
    let string = unsafe { CString::from_vec_unchecked(string.into()) }.into_raw();

    let result = unsafe {
        cfg_if! {
            if #[cfg(php_debug)] {
                #[allow(clippy::used_underscore_items)]
                _estrdup(string, std::ptr::null_mut(), 0, std::ptr::null_mut(), 0)
            } else {
                #[allow(clippy::used_underscore_items)]
                _estrdup(string)
            }
        }
    };

    drop(unsafe { CString::from_raw(string) });
    result
}

#[cfg(test)]
#[cfg(feature = "embed")]
mod test {
    use super::*;
    use crate::embed::Embed;
    use std::ffi::CStr;

    #[test]
    fn test_emalloc() {
        Embed::run(|| {
            let layout = Layout::from_size_align(16, 8).expect("should create layout");
            let ptr = emalloc(layout);
            assert!(!ptr.is_null());
            unsafe { efree(ptr) };
        });
    }

    #[test]
    fn test_estrdup() {
        Embed::run(|| {
            let original = "Hello, world!";
            let duplicated = estrdup(original);
            assert!(!duplicated.is_null());

            let duplicated_str = unsafe { CStr::from_ptr(duplicated) };
            assert_eq!(
                duplicated_str.to_str().expect("should convert to str"),
                original
            );

            unsafe { efree(duplicated.cast::<u8>()) }
        });
    }
}
