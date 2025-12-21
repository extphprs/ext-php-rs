#![cfg_attr(windows, feature(abi_vectorcall))]
#![allow(
    clippy::must_use_candidate,
    clippy::missing_panics_doc,
    clippy::needless_pass_by_value,
    clippy::implicit_hasher
)]
use ext_php_rs::prelude::*;

mod integration;

#[php_module]
pub fn build_module(module: ModuleBuilder) -> ModuleBuilder {
    let mut module = integration::array::build_module(module);
    module = integration::binary::build_module(module);
    module = integration::bool::build_module(module);
    module = integration::callable::build_module(module);
    module = integration::class::build_module(module);
    module = integration::closure::build_module(module);
    module = integration::defaults::build_module(module);
    #[cfg(feature = "enum")]
    {
        module = integration::enum_::build_module(module);
    }
    module = integration::exception::build_module(module);
    module = integration::globals::build_module(module);
    module = integration::iterator::build_module(module);
    module = integration::magic_method::build_module(module);
    module = integration::nullable::build_module(module);
    module = integration::number::build_module(module);
    module = integration::object::build_module(module);
    module = integration::persistent_string::build_module(module);
    module = integration::string::build_module(module);
    module = integration::variadic_args::build_module(module);
    module = integration::interface::build_module(module);

    module
}
