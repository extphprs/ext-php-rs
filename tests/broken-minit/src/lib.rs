//! An extension whose module entry builds fine but whose MINIT fails: a constant
//! name carries a NUL byte, so registration returns an error. Loading it must
//! make PHP refuse the module, not abort the process.
#![cfg_attr(windows, feature(abi_vectorcall))]
#![allow(missing_docs)]

use ext_php_rs::prelude::*;

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.constant(("BAD\0CONST", 1i64, &[]))
}
