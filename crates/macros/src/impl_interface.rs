//! Implementation for the `#[php_impl_interface]` macro.
//!
//! This macro allows classes to implement PHP interfaces by implementing Rust
//! traits that are marked with `#[php_interface]`.
//!
//! Uses the `inventory` crate for cross-crate interface discovery.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ItemImpl;

use crate::prelude::*;

const INTERNAL_INTERFACE_NAME_PREFIX: &str = "PhpInterface";

/// Parses a trait impl block and generates the interface implementation
/// registration.
///
/// # Arguments
///
/// * `input` - The trait impl block (e.g., `impl SomeTrait for SomeStruct { ...
///   }`)
///
/// # Generated Code
///
/// The macro generates:
/// 1. The original trait impl block (passed through unchanged)
/// 2. An `inventory::submit!` call to register the interface implementation
pub fn parser(input: &ItemImpl) -> Result<TokenStream> {
    // Extract the trait being implemented
    let Some((_, trait_path, _)) = &input.trait_ else {
        bail!(input => "`#[php_impl_interface]` can only be used on trait implementations (e.g., `impl SomeTrait for SomeStruct`)");
    };

    // Get the last segment of the trait path (the trait name)
    let trait_ident = match trait_path.segments.last() {
        Some(segment) => &segment.ident,
        None => {
            bail!(trait_path => "Invalid trait path");
        }
    };

    // Get the struct type being implemented
    let struct_ty = &input.self_ty;

    // Generate the internal interface struct name (e.g., PhpInterfaceSomeTrait)
    let interface_struct_name = format_ident!("{}{}", INTERNAL_INTERFACE_NAME_PREFIX, trait_ident);

    Ok(quote! {
        // Pass through the original trait implementation
        #input

        // Register the interface implementation using inventory for cross-crate discovery
        ::ext_php_rs::inventory::submit! {
            ::ext_php_rs::internal::class::InterfaceRegistration {
                class_type_id: ::std::any::TypeId::of::<#struct_ty>(),
                interface_getter: || (
                    || <#interface_struct_name as ::ext_php_rs::class::RegisteredClass>::get_metadata().ce(),
                    <#interface_struct_name as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME
                ),
            }
        }
    })
}
