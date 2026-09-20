//! An extension whose `ModuleBuilder` cannot become a module entry: one function
//! name carries a NUL byte. Loading it must fail MINIT, not abort the process.
#![cfg_attr(windows, feature(abi_vectorcall))]
#![allow(missing_docs)]

use ext_php_rs::builders::FunctionBuilder;
use ext_php_rs::prelude::*;
use ext_php_rs::types::Zval;
use ext_php_rs::zend::ExecuteData;

ext_php_rs::zend_fastcall! {
    extern fn noop(_ex: &mut ExecuteData, _retval: &mut Zval) {}
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.function(FunctionBuilder::new("bad\0name", noop))
}
