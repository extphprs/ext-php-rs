use darling::FromAttributes;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{ItemFn, Signature};

use crate::prelude::*;

#[derive(FromAttributes, Default, Debug)]
#[darling(default, attributes(php))]
pub(crate) struct PhpModuleAttribute {
    startup: Option<Ident>,
}

pub fn parser(input: ItemFn) -> Result<TokenStream> {
    let crate_name = std::env::var("CARGO_CRATE_NAME").ok();
    let static_ext = std::env::var("EXT_PHP_RS_STATIC_EXT").is_ok_and(|v| v == "1");
    parser_impl(input, crate_name.as_deref(), static_ext)
}

fn get_module_delegate(
    input: &ItemFn,
    crate_name: Option<&str>,
    static_ext: bool,
) -> Result<TokenStream> {
    Ok(match crate_name {
        Some(name) => {
            let ident = format_ident!("{}_get_module", name);
            quote! {
                #[doc(hidden)]
                #[allow(non_snake_case)]
                #[unsafe(no_mangle)]
                extern "C" fn #ident() -> *mut ::ext_php_rs::zend::ModuleEntry {
                    get_module()
                }
            }
        }
        None if static_ext => bail!(
            input => "EXT_PHP_RS_STATIC_EXT=1 requires the CARGO_CRATE_NAME environment variable (set by cargo) to derive the exported symbol name"
        ),
        None => quote! {},
    })
}

fn parser_impl(input: ItemFn, crate_name: Option<&str>, static_ext: bool) -> Result<TokenStream> {
    let delegate = get_module_delegate(&input, crate_name, static_ext)?;

    // An unmangled `get_module` collides with any other extension exporting the
    // same symbol when statically linked into one PHP binary, so
    // EXT_PHP_RS_STATIC_EXT=1 drops the export and the crate-prefixed delegate
    // above becomes the entry point. Toggling the variable re-expands this
    // macro because ext-php-rs's build.rs declares rerun-if-env-changed for it,
    // which cascades a rebuild of dependent crates.
    let get_module_no_mangle = (!static_ext).then(|| quote! { #[unsafe(no_mangle)] });

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
        #get_module_no_mangle
        extern "C" fn get_module() -> *mut ::ext_php_rs::zend::ModuleEntry {
            static __EXT_PHP_RS_MODULE_ENTRY: ::ext_php_rs::zend::StaticModuleEntry =
                ::ext_php_rs::zend::StaticModuleEntry::new();
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

            // Stores the user's original shutdown callback so we can chain it.
            static __EXT_PHP_RS_USER_SHUTDOWN: ::std::sync::OnceLock<
                Option<unsafe extern "C" fn(i32, i32) -> i32>,
            > = ::std::sync::OnceLock::new();

            extern "C" fn ext_php_rs_shutdown(ty: i32, mod_num: i32) -> i32 {
                let user_result = __EXT_PHP_RS_USER_SHUTDOWN
                    .get()
                    .and_then(|opt| *opt)
                    .map_or(0, |f| unsafe { f(ty, mod_num) });

                let entry = __EXT_PHP_RS_MODULE_ENTRY.get_or_init(|| unreachable!());
                // Only free when loaded as a shared extension (handle != NULL).
                // Statically linked modules (embed SAPI) have no DL_UNLOAD, and
                // PHP may still reference the pointers during later shutdown phases.
                if !unsafe { (*entry).handle }.is_null() {
                    unsafe {
                        ::ext_php_rs::zend::cleanup_module_allocations(entry);
                    }
                }

                user_result
            }

            __EXT_PHP_RS_MODULE_ENTRY.get_or_init(|| {
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
                    Ok((mut entry, startup)) => {
                        __EXT_PHP_RS_MODULE_STARTUP.lock().replace(startup);
                        // Chain our cleanup into MSHUTDOWN, preserving the
                        // user's shutdown callback (if any).
                        let _ = __EXT_PHP_RS_USER_SHUTDOWN.set(entry.module_shutdown_func);
                        entry.module_shutdown_func = Some(ext_php_rs_shutdown);
                        entry
                    },
                    Err(e) => panic!("Failed to build PHP module: {:?}", e),
                }
            })
        }

        #delegate

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

#[cfg(test)]
mod tests {
    use super::parser_impl;
    use syn::parse_quote;

    fn expand(crate_name: Option<&str>, static_ext: bool) -> String {
        let input: syn::ItemFn =
            parse_quote! { fn module(module: ModuleBuilder) -> ModuleBuilder { module } };
        parser_impl(input, crate_name, static_ext)
            .unwrap()
            .to_string()
            .replace(' ', "")
    }

    #[test]
    fn dynamic_build_exports_get_module_and_prefixed_delegate() {
        let out = expand(Some("my_ext"), false);
        assert!(out.contains(r#"#[unsafe(no_mangle)]extern"C"fnget_module("#));
        assert!(out.contains(r#"#[unsafe(no_mangle)]extern"C"fnmy_ext_get_module("#));
    }

    #[test]
    fn static_ext_suppresses_get_module_export_but_keeps_the_fn() {
        let out = expand(Some("my_ext"), true);
        assert!(!out.contains(r#"no_mangle)]extern"C"fnget_module("#));
        assert!(out.contains(r#"extern"C"fnget_module("#));
        assert!(out.contains(r#"#[unsafe(no_mangle)]extern"C"fnmy_ext_get_module("#));
    }

    #[test]
    fn missing_crate_name_skips_delegate_in_dynamic_build() {
        assert!(!expand(None, false).contains("_get_module"));
    }

    #[test]
    fn missing_crate_name_errors_in_static_build() {
        let input: syn::ItemFn =
            parse_quote! { fn module(module: ModuleBuilder) -> ModuleBuilder { module } };
        assert!(parser_impl(input, None, true).is_err());
    }
}
