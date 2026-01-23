//! Implementation of the `#[derive(PhpUnion)]` macro for representing PHP union
//! types as Rust enums.

use darling::ToTokens;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    DataEnum, DeriveInput, GenericParam, Generics, Ident, Lifetime, LifetimeParam, Type,
    WhereClause, punctuated::Punctuated, token::Where,
};

use crate::prelude::*;

/// Information about a variant's PHP type override from attributes.
#[derive(Debug, Clone)]
enum PhpTypeOverride {
    /// Use the default `FromZval::TYPE`
    Default,
    /// A PHP class name: `#[php(class = "MyClass")]`
    Class(String),
    /// A PHP interface name: `#[php(interface = "Iterator")]`
    Interface(String),
    /// An intersection of interfaces: `#[php(intersection = ["Countable",
    /// "Traversable"])]`
    Intersection(Vec<String>),
}

impl PhpTypeOverride {
    /// Returns true if this override creates an intersection group (requires
    /// DNF).
    fn is_intersection(&self) -> bool {
        matches!(self, PhpTypeOverride::Intersection(_))
    }
}

/// Parses `#[php(...)]` attributes from variant attributes.
///
/// Supports:
/// - `#[php(class = "MyClass")]` - single class
/// - `#[php(interface = "Iterator")]` - single interface
/// - `#[php(intersection = ["Countable", "Traversable"])]` - intersection group
fn parse_variant_php_attr(attrs: &[syn::Attribute]) -> Result<PhpTypeOverride> {
    for attr in attrs {
        if !attr.path().is_ident("php") {
            continue;
        }

        let nested = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        )?;

        for meta in nested {
            if let syn::Meta::NameValue(nv) = &meta {
                // Handle string value attributes: class = "...", interface = "..."
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit_str),
                    ..
                }) = &nv.value
                {
                    let value = lit_str.value();
                    if nv.path.is_ident("class") {
                        return Ok(PhpTypeOverride::Class(value));
                    } else if nv.path.is_ident("interface") {
                        return Ok(PhpTypeOverride::Interface(value));
                    }
                }

                // Handle array value: intersection = ["A", "B"]
                if nv.path.is_ident("intersection") {
                    if let syn::Expr::Array(arr) = &nv.value {
                        let names: Result<Vec<String>> = arr
                            .elems
                            .iter()
                            .map(|elem| {
                                if let syn::Expr::Lit(syn::ExprLit {
                                    lit: syn::Lit::Str(s),
                                    ..
                                }) = elem
                                {
                                    Ok(s.value())
                                } else {
                                    bail!(elem => "intersection array elements must be string literals")
                                }
                            })
                            .collect();
                        let names = names?;
                        if names.len() < 2 {
                            bail!(arr => "intersection requires at least 2 interfaces");
                        }
                        return Ok(PhpTypeOverride::Intersection(names));
                    }
                    bail!(nv.value => "intersection must be an array, e.g., intersection = [\"A\", \"B\"]");
                }
            }
        }
    }
    Ok(PhpTypeOverride::Default)
}

pub fn parser(input: DeriveInput) -> Result<TokenStream> {
    let DeriveInput {
        generics, ident, ..
    } = input;

    let (into_impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut into_where_clause = where_clause.cloned().unwrap_or_else(|| WhereClause {
        where_token: Where {
            span: Span::call_site(),
        },
        predicates: Punctuated::default(),
    });
    let mut from_where_clause = into_where_clause.clone();

    // Add lifetime for FromZval implementation
    let from_impl_generics = {
        let tokens = into_impl_generics.to_token_stream();
        let mut parsed: Generics = syn::parse2(tokens).expect("couldn't reparse generics");
        parsed
            .params
            .push(GenericParam::Lifetime(LifetimeParam::new(Lifetime::new(
                "'_zval",
                Span::call_site(),
            ))));
        parsed
    };

    // Add trait bounds for generic types
    for generic in &generics.params {
        match generic {
            GenericParam::Type(ty) => {
                let type_ident = &ty.ident;
                into_where_clause.predicates.push(
                    syn::parse2(quote! {
                        #type_ident: ::ext_php_rs::convert::IntoZval
                    })
                    .expect("couldn't parse where predicate"),
                );
                from_where_clause.predicates.push(
                    syn::parse2(quote! {
                        #type_ident: ::ext_php_rs::convert::FromZval<'_zval>
                    })
                    .expect("couldn't parse where predicate"),
                );
            }
            GenericParam::Lifetime(lt) => from_where_clause.predicates.push(
                syn::parse2(quote! {
                    '_zval: #lt
                })
                .expect("couldn't parse where predicate"),
            ),
            GenericParam::Const(_) => {}
        }
    }

    match input.data {
        syn::Data::Enum(data) => parse_enum(
            &data,
            &ident,
            &into_impl_generics,
            &from_impl_generics,
            &into_where_clause,
            &from_where_clause,
            &ty_generics,
        ),
        syn::Data::Struct(_) => {
            bail!(ident.span() => "Only enums are supported by the `#[derive(PhpUnion)]` macro. For structs, use `#[derive(ZvalConvert)]` instead.")
        }
        syn::Data::Union(_) => {
            bail!(ident.span() => "Only enums are supported by the `#[derive(PhpUnion)]` macro.")
        }
    }
}

/// Information collected from parsing a variant.
struct VariantInfo {
    /// The Rust type of the variant's field
    field_ty: Type,
    /// PHP type override from attributes (class/interface name)
    php_override: PhpTypeOverride,
}

#[allow(clippy::too_many_lines)]
fn parse_enum(
    data: &DataEnum,
    ident: &Ident,
    into_impl_generics: &syn::ImplGenerics,
    from_impl_generics: &Generics,
    into_where_clause: &WhereClause,
    from_where_clause: &WhereClause,
    ty_generics: &syn::TypeGenerics,
) -> Result<TokenStream> {
    if data.variants.is_empty() {
        bail!(ident.span() => "PhpUnion enum must have at least one variant.");
    }

    // Collect variant info for code generation
    let mut variant_infos = Vec::new();
    let mut into_variants = Vec::new();
    let mut from_variants = Vec::new();

    for variant in &data.variants {
        let variant_ident = &variant.ident;
        let fields = &variant.fields;

        // Parse PHP type override from variant attributes
        let php_override = parse_variant_php_attr(&variant.attrs)?;

        // PhpUnion only supports single-field tuple variants (no unit or named
        // variants)
        match fields {
            syn::Fields::Unnamed(unnamed) => {
                if unnamed.unnamed.len() != 1 {
                    bail!(
                        unnamed =>
                        "PhpUnion enum variants must have exactly one field. For example: `Int(i64)` or `Str(String)`."
                    );
                }

                let field_ty = &unnamed.unnamed.first().unwrap().ty;
                variant_infos.push(VariantInfo {
                    field_ty: field_ty.clone(),
                    php_override,
                });

                into_variants.push(quote! {
                    #ident::#variant_ident(val) => val.set_zval(zv, persistent)
                });

                from_variants.push(quote! {
                    if let ::std::option::Option::Some(value) = <#field_ty>::from_zval(zval) {
                        return ::std::option::Option::Some(Self::#variant_ident(value));
                    }
                });
            }
            syn::Fields::Unit => {
                bail!(
                    variant =>
                    "PhpUnion enum variants cannot be unit variants. Each variant must wrap a type, e.g., `Int(i64)`."
                );
            }
            syn::Fields::Named(_) => {
                bail!(
                    variant =>
                    "PhpUnion enum variants must use tuple syntax, e.g., `Int(i64)`, not named fields."
                );
            }
        }
    }

    // Check if any variant uses intersection (requires DNF output)
    let has_intersection = variant_infos
        .iter()
        .any(|info| info.php_override.is_intersection());

    // Generate the union_types() method body
    // If any variant uses intersection, we generate DNF (Vec<TypeGroup>)
    // Otherwise, we generate simple union (Vec<DataType>)
    let union_types_body = if has_intersection {
        // DNF mode: generate Vec<TypeGroup>
        let type_group_tokens: Vec<_> = variant_infos
            .iter()
            .map(|info| {
                match &info.php_override {
                    PhpTypeOverride::Default => {
                        // For default types, we need to convert DataType to a class name
                        // This only works for Object types; primitives in DNF are not supported
                        let ty = &info.field_ty;
                        quote! {
                            {
                                // Get the type and convert to TypeGroup::Single if it's an object
                                let dt = <#ty as ::ext_php_rs::convert::FromZval>::TYPE;
                                match dt {
                                    ::ext_php_rs::flags::DataType::Object(Some(name)) => {
                                        ::ext_php_rs::args::TypeGroup::Single(name.to_string())
                                    }
                                    _ => panic!("DNF types only support class/interface types, not primitives. Use #[php(class/interface = \"...\")] or #[php(intersection = [...])] on all variants.")
                                }
                            }
                        }
                    }
                    PhpTypeOverride::Class(name) | PhpTypeOverride::Interface(name) => {
                        quote! {
                            ::ext_php_rs::args::TypeGroup::Single(#name.to_string())
                        }
                    }
                    PhpTypeOverride::Intersection(names) => {
                        quote! {
                            ::ext_php_rs::args::TypeGroup::Intersection(
                                ::std::vec![#(#names.to_string()),*]
                            )
                        }
                    }
                }
            })
            .collect();

        quote! {
            ::ext_php_rs::convert::PhpUnionTypes::Dnf(
                ::std::vec![#(#type_group_tokens),*]
            )
        }
    } else {
        // Simple mode: generate Vec<DataType>
        let type_tokens: Vec<_> = variant_infos
            .iter()
            .map(|info| match &info.php_override {
                PhpTypeOverride::Default => {
                    let ty = &info.field_ty;
                    quote! {
                        <#ty as ::ext_php_rs::convert::FromZval>::TYPE
                    }
                }
                PhpTypeOverride::Class(name) | PhpTypeOverride::Interface(name) => {
                    quote! {
                        ::ext_php_rs::flags::DataType::Object(::std::option::Option::Some(#name))
                    }
                }
                PhpTypeOverride::Intersection(_) => {
                    unreachable!("intersection should trigger DNF mode")
                }
            })
            .collect();

        quote! {
            ::ext_php_rs::convert::PhpUnionTypes::Simple(
                ::std::vec![#(#type_tokens),*]
            )
        }
    };

    Ok(quote! {
        impl #into_impl_generics ::ext_php_rs::convert::IntoZval for #ident #ty_generics #into_where_clause {
            const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::Mixed;
            const NULLABLE: bool = false;

            fn set_zval(
                self,
                zv: &mut ::ext_php_rs::types::Zval,
                persistent: bool,
            ) -> ::ext_php_rs::error::Result<()> {
                use ::ext_php_rs::convert::IntoZval;

                match self {
                    #(#into_variants,)*
                }
            }
        }

        impl #from_impl_generics ::ext_php_rs::convert::FromZval<'_zval> for #ident #ty_generics #from_where_clause {
            const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::Mixed;

            fn from_zval(zval: &'_zval ::ext_php_rs::types::Zval) -> ::std::option::Option<Self> {
                use ::ext_php_rs::convert::FromZval;

                #(#from_variants)*

                ::std::option::Option::None
            }
        }

        impl #from_impl_generics ::ext_php_rs::convert::PhpUnion<'_zval> for #ident #ty_generics #from_where_clause {
            fn union_types() -> ::ext_php_rs::convert::PhpUnionTypes {
                use ::ext_php_rs::convert::FromZval;

                #union_types_body
            }
        }
    })
}
