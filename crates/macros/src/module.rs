use darling::FromAttributes;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{ItemFn, Signature};

use crate::prelude::*;

#[derive(FromAttributes, Default, Debug)]
#[darling(default, attributes(php))]
pub(crate) struct PhpModuleAttribute {
    startup: Option<Ident>,
}

pub fn parser(input: ItemFn) -> Result<TokenStream> {
    let ItemFn { sig, block, .. } = input;
    let Signature { output, inputs, .. } = sig;
    let stmts = &block.stmts;

    let attr = PhpModuleAttribute::from_attributes(&input.attrs)?;
    let startup = if let Some(startup) = attr.startup {
        quote! { #startup(ty, mod_num) }
    } else {
        quote! { 0i32 }
    };

    Ok(quote! {
        #[doc(hidden)]
        #[unsafe(no_mangle)]
        extern "C" fn get_module() -> *mut ::ext_php_rs::zend::ModuleEntry {
            static __EXT_PHP_RS_MODULE_STARTUP: ::ext_php_rs::internal::ModuleStartupMutex =
                ::ext_php_rs::internal::MODULE_STARTUP_INIT;

            extern "C" fn ext_php_rs_startup(ty: i32, mod_num: i32) -> i32 {
                let a = unsafe { #startup };
                let b = __EXT_PHP_RS_MODULE_STARTUP
                    .lock()
                    .take()
                    .map(|startup| {
                        ::ext_php_rs::internal::ext_php_rs_startup();
                        startup.startup(ty, mod_num).map(|_| 0).unwrap_or(1)
                    })
                    .unwrap_or_else(|| {
                        // Module already started, call ext_php_rs_startup for idempotent
                        // initialization (e.g., Closure::build early-returns if already built)
                        ::ext_php_rs::internal::ext_php_rs_startup();
                        0
                    });
                a | b
            }

            #[inline]
            fn internal(#inputs) #output {
                #(#stmts)*
            }

            let builder = internal(::ext_php_rs::builders::ModuleBuilder::new(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))
            .startup_function(ext_php_rs_startup);

            match builder.try_into() {
                Ok((entry, startup)) => {
                    __EXT_PHP_RS_MODULE_STARTUP.lock().replace(startup);
                    entry.into_raw()
                },
                Err(e) => panic!("Failed to build PHP module: {:?}", e),
            }
        }

        #[cfg(debug_assertions)]
        #[unsafe(no_mangle)]
        pub extern "C" fn ext_php_rs_describe_module() -> ::ext_php_rs::describe::Description {
            use ::ext_php_rs::describe::*;

            #[inline]
            fn internal(#inputs) #output {
                #(#stmts)*
            }

            let builder = internal(::ext_php_rs::builders::ModuleBuilder::new(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ));

            Description::new(builder.into())
        }
    })
}
