//! Implementation for the `#[php_impl_interface]` macro.
//!
//! This macro allows classes to implement PHP interfaces by implementing Rust
//! traits that are marked with `#[php_interface]`.
//!
//! Uses the `inventory` crate for cross-crate interface discovery.
//! Method registration uses autoref specialization to avoid PHP symbol
//! resolution issues at binary load time.

use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ImplItem, ItemImpl, Pat, ReturnType};

use crate::parsing::{MethodRename, RenameRule, ident_to_php_name};
use crate::prelude::*;

/// Attributes for the `#[php_impl_interface]` macro.
#[derive(FromMeta, Default, Debug, Copy, Clone)]
#[darling(default)]
pub struct PhpImplInterfaceArgs {
    /// Rename methods to match the given rule. Should match the interface's
    /// `change_method_case` if specified.
    change_method_case: Option<RenameRule>,
}

const INTERNAL_INTERFACE_NAME_PREFIX: &str = "PhpInterface";

/// Parses a trait impl block and generates the interface implementation
/// registration.
///
/// # Arguments
///
/// * `args` - The macro arguments (e.g., `change_method_case = "snake_case"`)
/// * `input` - The trait impl block (e.g., `impl SomeTrait for SomeStruct { ...
///   }`)
///
/// # Generated Code
///
/// The macro generates:
/// 1. The original trait impl block (passed through unchanged)
/// 2. An `inventory::submit!` call to register the interface implementation
/// 3. An `InterfaceMethodsProvider` trait implementation for method
///    registration
///
/// # Path Resolution
///
/// The macro preserves the full module path of the trait, so
/// `impl other::MyTrait for Foo` will correctly reference
/// `other::PhpInterfaceMyTrait`.
pub fn parser(args: PhpImplInterfaceArgs, input: &ItemImpl) -> Result<TokenStream> {
    let change_method_case = args.change_method_case.unwrap_or(RenameRule::Camel);
    // Extract the trait being implemented
    let Some((_, trait_path, _)) = &input.trait_ else {
        bail!(input => "`#[php_impl_interface]` can only be used on trait implementations (e.g., `impl SomeTrait for SomeStruct`)");
    };

    // Clone the trait path and modify the last segment to add PhpInterface prefix
    let mut interface_struct_path = trait_path.clone();
    match interface_struct_path.segments.last_mut() {
        Some(segment) => {
            segment.ident = format_ident!("{}{}", INTERNAL_INTERFACE_NAME_PREFIX, segment.ident);
        }
        None => {
            bail!(trait_path => "Invalid trait path");
        }
    }

    // Get the struct type being implemented
    let struct_ty = &input.self_ty;

    // Generate method builders for each trait method
    let mut method_builders = Vec::new();

    for item in &input.items {
        let ImplItem::Fn(method) = item else {
            continue;
        };

        let method_ident = &method.sig.ident;
        let php_name = ident_to_php_name(method_ident);
        let php_name = php_name.rename_method(change_method_case);

        // Check if this is a static method (no self receiver)
        let has_self = method
            .sig
            .inputs
            .iter()
            .any(|arg| matches!(arg, FnArg::Receiver(_)));
        let is_static = !has_self;

        // Generate the method builder
        let builder = generate_method_builder(
            &php_name,
            struct_ty,
            method_ident,
            &method.sig.inputs,
            &method.sig.output,
            is_static,
        );
        method_builders.push(builder);
    }

    Ok(quote! {
        // Pass through the original trait implementation
        #input

        // Register the interface implementation using inventory for cross-crate discovery
        ::ext_php_rs::inventory::submit! {
            ::ext_php_rs::internal::class::InterfaceRegistration {
                class_type_id: ::std::any::TypeId::of::<#struct_ty>(),
                interface_getter: || (
                    || <#interface_struct_path as ::ext_php_rs::class::RegisteredClass>::get_metadata().ce(),
                    <#interface_struct_path as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME
                ),
            }
        }

        // Implement InterfaceMethodsProvider for the class (direct impl, not on reference)
        // This uses autoref specialization - the direct impl takes precedence over the
        // default reference impl.
        impl ::ext_php_rs::internal::class::InterfaceMethodsProvider<#struct_ty>
            for ::ext_php_rs::internal::class::PhpClassImplCollector<#struct_ty>
        {
            fn get_interface_methods(self) -> ::std::vec::Vec<(
                ::ext_php_rs::builders::FunctionBuilder<'static>,
                ::ext_php_rs::flags::MethodFlags,
            )> {
                vec![
                    #(#method_builders),*
                ]
            }
        }
    })
}

/// Generates a method builder expression (`FunctionBuilder`, `MethodFlags`).
/// The handler is defined inside the `FunctionBuilder::new()` call, so it's
/// only instantiated when `get_interface_methods()` is called at runtime.
#[allow(clippy::too_many_lines)]
fn generate_method_builder(
    php_name: &str,
    struct_ty: &syn::Type,
    method_ident: &syn::Ident,
    inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
    output: &ReturnType,
    is_static: bool,
) -> TokenStream {
    // Helper to check if a type is Option<T>
    fn is_option_type(ty: &syn::Type) -> bool {
        if let syn::Type::Path(type_path) = ty
            && let Some(segment) = type_path.path.segments.last()
        {
            return segment.ident == "Option";
        }
        false
    }

    // Collect non-self arguments in order
    let args: Vec<_> = inputs
        .iter()
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg
                && let Pat::Ident(pat_ident) = &*pat_type.pat
            {
                return Some((&pat_ident.ident, &pat_type.ty));
            }
            None
        })
        .collect();

    // Find the optional boundary: the first index where all remaining args are Option<T>.
    // Args before this boundary are required (even if Option<T>), args at/after are optional.
    // This handles cases like `fn foo(opt: Option<i64>, required: i64)` correctly -
    // `opt` is effectively required because a required param follows it.
    let optional_boundary = {
        let mut boundary = args.len();
        for i in (0..args.len()).rev() {
            if is_option_type(args[i].1) {
                boundary = i;
            } else {
                break;
            }
        }
        boundary
    };

    let required_args: Vec<_> = args[..optional_boundary].to_vec();
    let optional_args: Vec<_> = args[optional_boundary..].to_vec();

    // Create Arg declarations for each parameter (in order)
    let required_arg_declarations: Vec<_> = required_args
        .iter()
        .map(|(name, ty)| {
            let php_name = ident_to_php_name(name);
            // Even Option<T> args in required position don't get allow_null() in parser
            quote! {
                let mut #name = ::ext_php_rs::args::Arg::new(#php_name, <#ty as ::ext_php_rs::convert::FromZvalMut>::TYPE);
            }
        })
        .collect();

    let optional_arg_declarations: Vec<_> = optional_args
        .iter()
        .map(|(name, ty)| {
            let php_name = ident_to_php_name(name);
            quote! {
                let mut #name = ::ext_php_rs::args::Arg::new(#php_name, <#ty as ::ext_php_rs::convert::FromZvalMut>::TYPE).allow_null();
            }
        })
        .collect();

    let required_arg_names: Vec<_> = required_args.iter().map(|(name, _)| *name).collect();
    let optional_arg_names: Vec<_> = optional_args.iter().map(|(name, _)| *name).collect();
    let arg_names: Vec<_> = args.iter().map(|(name, _)| name).collect();

    // Value accessors that extract values from the Arg objects after parsing
    let required_arg_value_accessors: Vec<_> = required_args
        .iter()
        .map(|(name, ty)| {
            let php_name = ident_to_php_name(name);
            if is_option_type(ty) {
                // Option<T> in required position - still use .val() which returns Option<T>
                quote! {
                    let #name: #ty = #name.val();
                }
            } else {
                quote! {
                    let #name: #ty = match #name.val() {
                        Some(v) => v,
                        None => {
                            let msg = format!("Invalid value for argument `{}`", #php_name);
                            ::ext_php_rs::exception::PhpException::default(msg.into())
                                .throw()
                                .expect("Failed to throw PHP exception.");
                            return;
                        }
                    };
                }
            }
        })
        .collect();

    let optional_arg_value_accessors: Vec<_> = optional_args
        .iter()
        .map(|(name, ty)| {
            // For Option<T>, .val() returns Option<T>, which is what we want
            quote! {
                let #name: #ty = #name.val();
            }
        })
        .collect();

    // Generate .arg() calls for PHP reflection metadata (for FunctionBuilder)
    let required_arg_builders: Vec<_> = required_args
        .iter()
        .map(|(name, ty)| {
            let php_name = ident_to_php_name(name);
            quote! {
                .arg(::ext_php_rs::args::Arg::new(#php_name, <#ty as ::ext_php_rs::convert::FromZvalMut>::TYPE))
            }
        })
        .collect();

    let optional_arg_builders: Vec<_> = optional_args
        .iter()
        .map(|(name, ty)| {
            let php_name = ident_to_php_name(name);
            quote! {
                .arg(::ext_php_rs::args::Arg::new(#php_name, <#ty as ::ext_php_rs::convert::FromZvalMut>::TYPE).allow_null())
            }
        })
        .collect();

    let flags = if is_static {
        quote! { ::ext_php_rs::flags::MethodFlags::Public | ::ext_php_rs::flags::MethodFlags::Static }
    } else {
        quote! { ::ext_php_rs::flags::MethodFlags::Public }
    };

    // Generate the .returns() call - void for no return type, or the actual type
    let returns_call = match output {
        ReturnType::Default => quote! {
            .returns(::ext_php_rs::flags::DataType::Void, false, false)
        },
        ReturnType::Type(_, ty) => {
            quote! {
                .returns(
                    <#ty as ::ext_php_rs::convert::IntoZval>::TYPE,
                    false,
                    <#ty as ::ext_php_rs::convert::IntoZval>::NULLABLE,
                )
            }
        }
    };

    // Check if return type is void
    let is_void = matches!(output, ReturnType::Default);

    // Generate the method call and result handling based on whether it's void
    let result_handling = if is_void {
        quote! {
            // Void method - just call it, don't try to set return value
        }
    } else {
        quote! {
            if let Err(e) = result.set_zval(retval, false) {
                let e: ::ext_php_rs::exception::PhpException = e.into();
                e.throw().expect("Failed to throw PHP exception.");
            }
        }
    };

    let handler_body = if is_static {
        if is_void {
            quote! {
                #(#required_arg_declarations)*
                #(#optional_arg_declarations)*
                let parse = ex.parser()
                    #(.arg(&mut #required_arg_names))*
                    .not_required()
                    #(.arg(&mut #optional_arg_names))*
                    .parse();
                if parse.is_err() {
                    return;
                }
                #(#required_arg_value_accessors)*
                #(#optional_arg_value_accessors)*
                <#struct_ty>::#method_ident(#(#arg_names),*);
            }
        } else {
            quote! {
                #(#required_arg_declarations)*
                #(#optional_arg_declarations)*
                let parse = ex.parser()
                    #(.arg(&mut #required_arg_names))*
                    .not_required()
                    #(.arg(&mut #optional_arg_names))*
                    .parse();
                if parse.is_err() {
                    return;
                }
                #(#required_arg_value_accessors)*
                #(#optional_arg_value_accessors)*
                let result = <#struct_ty>::#method_ident(#(#arg_names),*);
                #result_handling
            }
        }
    } else if is_void {
        quote! {
            let (parse, this) = ex.parser_method::<#struct_ty>();
            let this = match this {
                Some(this) => this,
                None => {
                    ::ext_php_rs::exception::PhpException::default("Failed to get $this".into())
                        .throw()
                        .expect("Failed to throw PHP exception.");
                    return;
                }
            };
            #(#required_arg_declarations)*
            #(#optional_arg_declarations)*
            let parse_result = parse
                #(.arg(&mut #required_arg_names))*
                .not_required()
                #(.arg(&mut #optional_arg_names))*
                .parse();
            if parse_result.is_err() {
                return;
            }
            #(#required_arg_value_accessors)*
            #(#optional_arg_value_accessors)*
            this.#method_ident(#(#arg_names),*);
        }
    } else {
        quote! {
            let (parse, this) = ex.parser_method::<#struct_ty>();
            let this = match this {
                Some(this) => this,
                None => {
                    ::ext_php_rs::exception::PhpException::default("Failed to get $this".into())
                        .throw()
                        .expect("Failed to throw PHP exception.");
                    return;
                }
            };
            #(#required_arg_declarations)*
            #(#optional_arg_declarations)*
            let parse_result = parse
                #(.arg(&mut #required_arg_names))*
                .not_required()
                #(.arg(&mut #optional_arg_names))*
                .parse();
            if parse_result.is_err() {
                return;
            }
            #(#required_arg_value_accessors)*
            #(#optional_arg_value_accessors)*
            let result = this.#method_ident(#(#arg_names),*);
            #result_handling
        }
    };

    quote! {
        (
            ::ext_php_rs::builders::FunctionBuilder::new(#php_name, {
                ::ext_php_rs::zend_fastcall! {
                    extern fn handler(
                        ex: &mut ::ext_php_rs::zend::ExecuteData,
                        retval: &mut ::ext_php_rs::types::Zval,
                    ) {
                        use ::ext_php_rs::convert::IntoZval;
                        use ::ext_php_rs::zend::try_catch;
                        use ::std::panic::AssertUnwindSafe;

                        let catch_result = try_catch(AssertUnwindSafe(|| {
                            #handler_body
                        }));

                        if catch_result.is_err() {
                            ::ext_php_rs::zend::run_bailout_cleanups();
                            unsafe {
                                ::ext_php_rs::zend::bailout();
                            }
                        }
                    }
                }
                handler
            })
            #(#required_arg_builders)*
            .not_required()
            #(#optional_arg_builders)*
            #returns_call,
            #flags
        )
    }
}
