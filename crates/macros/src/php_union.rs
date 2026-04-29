use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned as _;
use syn::{DeriveInput, Type};

use crate::prelude::*;

pub fn parser(input: DeriveInput) -> Result<TokenStream> {
    let DeriveInput {
        ident,
        data,
        generics,
        ..
    } = input;

    if !generics.params.is_empty() {
        bail!(generics.span() => "`#[derive(PhpUnion)]` does not support generics yet; remove type or lifetime parameters from the enum");
    }

    let data = match data {
        syn::Data::Enum(data) => data,
        syn::Data::Struct(_) => {
            bail!(ident.span() => "`#[derive(PhpUnion)]` requires an enum; structs map to objects via `#[derive(ZvalConvert)]`")
        }
        syn::Data::Union(_) => {
            bail!(ident.span() => "`#[derive(PhpUnion)]` requires an enum")
        }
    };

    if data.variants.is_empty() {
        bail!(ident.span() => "`#[derive(PhpUnion)]` requires at least one variant");
    }

    let mut variants: Vec<(syn::Ident, Type)> = Vec::with_capacity(data.variants.len());
    for variant in &data.variants {
        let v_ident = variant.ident.clone();
        match &variant.fields {
            syn::Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let ty = fields.unnamed.first().unwrap().ty.clone();
                variants.push((v_ident, ty));
            }
            syn::Fields::Unnamed(fields) => {
                bail!(variant.span() => "`#[derive(PhpUnion)]` variant `{}` must wrap exactly one field; found {}", v_ident, fields.unnamed.len());
            }
            syn::Fields::Named(_) => {
                bail!(variant.span() => "`#[derive(PhpUnion)]` variant `{}` cannot have named fields; rewrite as `{}(T)`", v_ident, v_ident);
            }
            syn::Fields::Unit => {
                bail!(variant.span() => "`#[derive(PhpUnion)]` variant `{}` must wrap a value; unit variants are not supported", v_ident);
            }
        }
    }

    let variant_types: Vec<&Type> = variants.iter().map(|(_, ty)| ty).collect();

    let into_arms = variants.iter().map(|(v_ident, _)| {
        quote! {
            Self::#v_ident(val) => val.set_zval(zv, persistent)
        }
    });

    let from_arms = variants.iter().map(|(v_ident, ty)| {
        quote! {
            if let ::std::option::Option::Some(value) =
                <#ty as ::ext_php_rs::convert::FromZval>::from_zval(zval)
            {
                return ::std::option::Option::Some(Self::#v_ident(value));
            }
        }
    });

    Ok(quote! {
        impl ::ext_php_rs::types::PhpUnion for #ident {
            fn union_types() -> ::ext_php_rs::types::PhpType {
                ::ext_php_rs::types::PhpType::Union(::std::vec![
                    #(<#variant_types as ::ext_php_rs::convert::IntoZval>::TYPE),*
                ])
            }
        }

        impl ::ext_php_rs::convert::IntoZval for #ident {
            const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::Mixed;
            const NULLABLE: bool = false;

            fn php_type() -> ::ext_php_rs::types::PhpType {
                <Self as ::ext_php_rs::types::PhpUnion>::union_types()
            }

            fn set_zval(
                self,
                zv: &mut ::ext_php_rs::types::Zval,
                persistent: bool,
            ) -> ::ext_php_rs::error::Result<()> {
                use ::ext_php_rs::convert::IntoZval;
                match self {
                    #(#into_arms,)*
                }
            }
        }

        impl<'_zval> ::ext_php_rs::convert::FromZval<'_zval> for #ident {
            const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::Mixed;

            fn php_type() -> ::ext_php_rs::types::PhpType {
                <Self as ::ext_php_rs::types::PhpUnion>::union_types()
            }

            fn from_zval(zval: &'_zval ::ext_php_rs::types::Zval) -> ::std::option::Option<Self> {
                #(#from_arms)*
                ::std::option::Option::None
            }
        }
    })
}
