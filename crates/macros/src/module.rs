use darling::FromAttributes;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{ItemFn, Signature};

use crate::parsing::reject_php_attrs;
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
    for arg in &input.sig.inputs {
        if let syn::FnArg::Typed(arg) = arg {
            reject_php_attrs(&arg.attrs, "`#[php_module]` parameters")?;
        }
    }
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
                let b = ::ext_php_rs::internal::startup_guard(|| {
                    // ext_php_rs_startup is idempotent (Closure::build early-returns once
                    // built), so it runs whether or not this is the first startup.
                    ::ext_php_rs::internal::ext_php_rs_startup();
                    match __EXT_PHP_RS_MODULE_STARTUP.lock().take() {
                        Some(startup) => startup.startup(ty, mod_num),
                        None => Ok(()),
                    }
                });
                a | b
            }

            static __EXT_PHP_RS_BUILD_ERROR: ::std::sync::OnceLock<::std::string::String> =
                ::std::sync::OnceLock::new();

            extern "C" fn ext_php_rs_failed_startup(_ty: i32, _mod_num: i32) -> i32 {
                ::ext_php_rs::internal::failed_module_startup(
                    __EXT_PHP_RS_BUILD_ERROR.get().map_or("unknown error", ::std::string::String::as_str),
                )
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
                    Ok((entry, startup, owned)) => {
                        __EXT_PHP_RS_MODULE_STARTUP.lock().replace(startup);
                        (entry, owned)
                    },
                    Err(e) => {
                        // get_module cannot report failure to the engine (dl() dereferences the
                        // returned entry unchecked), so hand back a placeholder entry whose
                        // MINIT logs the build error and fails.
                        let _ = __EXT_PHP_RS_BUILD_ERROR.set(e.to_string());
                        let (entry, _, owned) = ::ext_php_rs::builders::ModuleBuilder::new(
                            env!("CARGO_PKG_NAME"),
                            env!("CARGO_PKG_VERSION"),
                        )
                        .startup_function(ext_php_rs_failed_startup)
                        .try_into()
                        .unwrap_or_else(|_| ::std::unreachable!("a module with only env! strings always builds"));
                        (entry, owned)
                    }
                }
            })
        }

        #delegate

        #[cfg(debug_assertions)]
        #[unsafe(no_mangle)]
        pub extern "C" fn ext_php_rs_describe_module_v2() -> *mut ::ext_php_rs::describe::Description {
            use ::ext_php_rs::describe::*;

            #[inline]
            fn internal(#inputs) #output {
                #(#stmts)*
            }

            let builder = internal(::ext_php_rs::builders::ModuleBuilder::new(
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ));

            ::std::boxed::Box::into_raw(::std::boxed::Box::new(Description::new(builder.into())))
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
    fn build_failure_yields_a_placeholder_entry_whose_minit_fails() {
        let out = expand(Some("my_ext"), false);
        assert!(!out.contains("panic!"));
        assert!(out.contains(r#"extern"C"fnext_php_rs_failed_startup("#));
        assert!(out.contains("::ext_php_rs::internal::failed_module_startup("));
        assert!(out.contains(".startup_function(ext_php_rs_failed_startup)"));
    }

    #[test]
    fn minit_runs_registration_under_startup_guard() {
        let out = expand(Some("my_ext"), false);
        assert!(out.contains("::ext_php_rs::internal::startup_guard(||"));
        assert!(!out.contains("unwrap_or(1)"));
    }

    #[test]
    fn missing_crate_name_skips_delegate_in_dynamic_build() {
        assert!(!expand(None, false).contains("_get_module"));
    }

    #[test]
    fn shutdown_function_is_left_to_the_user() {
        let out = expand(Some("my_ext"), false);
        assert!(!out.contains("module_shutdown_func"));
        assert!(!out.contains("cleanup_module_allocations"));
    }

    #[test]
    fn missing_crate_name_errors_in_static_build() {
        let input: syn::ItemFn =
            parse_quote! { fn module(module: ModuleBuilder) -> ModuleBuilder { module } };
        assert!(parser_impl(input, None, true).is_err());
    }
}
