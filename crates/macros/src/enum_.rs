use std::convert::TryFrom;

use darling::{FromAttributes, util::Flag};
use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Fields, Ident, ItemEnum, Lit};

use crate::{
    helpers::get_docs,
    parsing::{
        PhpNameContext, PhpRename, RenameRule, Visibility, ident_to_php_name, validate_php_name,
    },
    prelude::*,
};

#[derive(FromAttributes, Default, Debug)]
#[darling(default, attributes(php), forward_attrs(doc))]
struct PhpEnumAttribute {
    #[darling(flatten)]
    rename: PhpRename,
    #[darling(default)]
    allow_native_discriminants: Flag,
    rename_cases: Option<RenameRule>,
    // TODO: Implement visibility support
    vis: Option<Visibility>,
    attrs: Vec<syn::Attribute>,
}

#[derive(FromAttributes, Default, Debug)]
#[darling(default, attributes(php), forward_attrs(doc))]
struct PhpEnumVariantAttribute {
    #[darling(flatten)]
    rename: PhpRename,
    #[darling(rename = "value")]
    discriminant: Option<Lit>,
    attrs: Vec<syn::Attribute>,
}

pub fn parser(mut input: ItemEnum) -> Result<TokenStream> {
    let php_attr = PhpEnumAttribute::from_attributes(&input.attrs)?;
    input.attrs.retain(|attr| !attr.path().is_ident("php"));

    let docs = get_docs(&php_attr.attrs)?;
    let mut cases = vec![];
    let mut discriminant_type = DiscriminantType::None;

    for variant in &mut input.variants {
        if variant.fields != Fields::Unit {
            bail!("Enum cases must be unit variants, found: {:?}", variant);
        }
        if !php_attr.allow_native_discriminants.is_present() && variant.discriminant.is_some() {
            bail!(variant => "Native discriminants are currently not exported to PHP. To set a discriminant, use the `#[php(allow_native_discriminants)]` attribute on the enum. To export discriminants, set the #[php(value = ...)] attribute on the enum case.");
        }

        let variant_attr = PhpEnumVariantAttribute::from_attributes(&variant.attrs)?;
        variant.attrs.retain(|attr| !attr.path().is_ident("php"));
        let docs = get_docs(&variant_attr.attrs)?;
        let discriminant = variant_attr
            .discriminant
            .as_ref()
            .map(TryInto::try_into)
            .transpose()?;

        if let Some(d) = &discriminant {
            match d {
                Discriminant::String(_) => {
                    if discriminant_type == DiscriminantType::Integer {
                        bail!(variant => "Mixed discriminants are not allowed in enums, found string and integer discriminants");
                    }

                    discriminant_type = DiscriminantType::String;
                }
                Discriminant::Integer(_) => {
                    if discriminant_type == DiscriminantType::String {
                        bail!(variant => "Mixed discriminants are not allowed in enums, found string and integer discriminants");
                    }

                    discriminant_type = DiscriminantType::Integer;
                }
            }
        } else if discriminant_type != DiscriminantType::None {
            bail!(variant => "Discriminant must be specified for all enum cases, found: {:?}", variant);
        }

        let case_name = variant_attr.rename.rename(
            ident_to_php_name(&variant.ident),
            php_attr.rename_cases.unwrap_or(RenameRule::Pascal),
        );
        validate_php_name(&case_name, PhpNameContext::EnumCase, variant.ident.span())?;

        cases.push(EnumCase {
            ident: variant.ident.clone(),
            name: case_name,
            attrs: variant_attr,
            discriminant,
            docs,
        });

        if !cases
            .iter()
            .filter_map(|case| case.discriminant.as_ref())
            .all_unique()
        {
            bail!(variant => "Enum cases must have unique discriminants, found duplicates in: {:?}", cases);
        }
    }

    let enum_props = Enum::new(
        &input.ident,
        &php_attr,
        docs,
        cases,
        None, // TODO: Implement flags support
        discriminant_type,
    )?;

    Ok(quote! {
        #[allow(dead_code)]
        #input

        #enum_props
    })
}

#[derive(Debug)]
pub struct Enum<'a> {
    ident: &'a Ident,
    name: String,
    discriminant_type: DiscriminantType,
    docs: Vec<String>,
    cases: Vec<EnumCase>,
    flags: Option<String>,
}

impl<'a> Enum<'a> {
    fn new(
        ident: &'a Ident,
        attrs: &PhpEnumAttribute,
        docs: Vec<String>,
        cases: Vec<EnumCase>,
        flags: Option<String>,
        discriminant_type: DiscriminantType,
    ) -> Result<Self> {
        let name = attrs
            .rename
            .rename(ident_to_php_name(ident), RenameRule::Pascal);
        validate_php_name(&name, PhpNameContext::Enum, ident.span())?;

        Ok(Self {
            ident,
            name,
            discriminant_type,
            docs,
            cases,
            flags,
        })
    }

    fn registered_class(&self) -> TokenStream {
        let ident = &self.ident;
        let name = &self.name;
        let flags = self
            .flags
            .as_ref()
            .map(|f| quote! { | #f })
            .unwrap_or_default();
        let flags = quote! { ::ext_php_rs::flags::ClassFlags::Enum #flags };
        let docs = &self.docs;

        quote! {
            impl ::ext_php_rs::class::RegisteredClass for #ident {
                const CLASS_NAME: &'static str = #name;
                const BUILDER_MODIFIER: ::std::option::Option<
                    fn(::ext_php_rs::builders::ClassBuilder) -> ::ext_php_rs::builders::ClassBuilder
                > = None;
                const EXTENDS: ::std::option::Option<
                    ::ext_php_rs::class::ClassEntryInfo
                > = None;
                const IMPLEMENTS: &'static [::ext_php_rs::class::ClassEntryInfo] = &[];
                const FLAGS: ::ext_php_rs::flags::ClassFlags = #flags;
                const DOC_COMMENTS: &'static [&'static str] = &[
                    #(#docs,)*
                ];

                fn get_metadata() -> &'static ::ext_php_rs::class::ClassMetadata<Self> {
                    static METADATA: ::ext_php_rs::class::ClassMetadata<#ident> =
                        ::ext_php_rs::class::ClassMetadata::new();
                    &METADATA
                }

                #[inline]
                fn get_properties<'a>() -> ::std::collections::HashMap<
                    &'static str, ::ext_php_rs::internal::property::PropertyInfo<'a, Self>
                > {
                    ::std::collections::HashMap::new()
                }

                #[inline]
                fn method_builders() -> ::std::vec::Vec<
                    (::ext_php_rs::builders::FunctionBuilder<'static>, ::ext_php_rs::flags::MethodFlags)
                > {
                    use ::ext_php_rs::internal::class::PhpClassImpl;
                    ::ext_php_rs::internal::class::PhpClassImplCollector::<Self>::default().get_methods()
                }

                #[inline]
                fn constructor() -> ::std::option::Option<::ext_php_rs::class::ConstructorMeta<Self>> {
                    None
                }

                #[inline]
                fn constants() -> &'static [(&'static str, &'static dyn ::ext_php_rs::convert::IntoZvalDyn, &'static [&'static str])] {
                    use ::ext_php_rs::internal::class::PhpClassImpl;
                    ::ext_php_rs::internal::class::PhpClassImplCollector::<Self>::default().get_constants()
                }
            }
        }
    }

    fn registered_enum(&self) -> TokenStream {
        let ident = &self.ident;
        let cases = &self.cases;
        let case_from_names = self.cases.iter().map(|case| {
            let ident = &case.ident;
            let name = &case.name;
            quote! {
                #name => Ok(Self::#ident)
            }
        });
        let case_to_names = self.cases.iter().map(|case| {
            let ident = &case.ident;
            let name = &case.name;
            quote! {
                Self::#ident => #name
            }
        });

        quote! {
            impl ::ext_php_rs::enum_::RegisteredEnum for #ident {
                const CASES: &'static [::ext_php_rs::enum_::EnumCase] = &[
                    #(#cases,)*
                ];

                fn from_name(name: &str) -> ::ext_php_rs::error::Result<Self> {
                    match name {
                        #(#case_from_names,)*
                        _ => Err(::ext_php_rs::error::Error::InvalidProperty),
                    }
                }

                fn to_name(&self) -> &'static str {
                    match self {
                        #(#case_to_names,)*
                    }
                }
            }
        }
    }

    pub fn impl_try_from(&self) -> TokenStream {
        if self.discriminant_type == DiscriminantType::None {
            return quote! {};
        }
        let discriminant_type = match self.discriminant_type {
            DiscriminantType::Integer => quote! { i64 },
            DiscriminantType::String => quote! { &str },
            DiscriminantType::None => unreachable!("Discriminant type should not be None here"),
        };
        let ident = &self.ident;
        let cases = self.cases.iter().map(|case| {
            let ident = &case.ident;
            match case
                .discriminant
                .as_ref()
                .expect("Discriminant should be set")
            {
                Discriminant::String(s) => quote! { #s => Ok(Self::#ident) },
                Discriminant::Integer(i) => quote! { #i => Ok(Self::#ident) },
            }
        });

        quote! {
            impl TryFrom<#discriminant_type> for #ident {
                type Error = ::ext_php_rs::error::Error;

                fn try_from(value: #discriminant_type) -> ::ext_php_rs::error::Result<Self> {
                    match value {
                        #(
                            #cases,
                        )*
                        _ => Err(::ext_php_rs::error::Error::InvalidProperty),
                    }
                }
            }
        }
    }

    pub fn impl_into(&self) -> TokenStream {
        if self.discriminant_type == DiscriminantType::None {
            return quote! {};
        }
        let discriminant_type = match self.discriminant_type {
            DiscriminantType::Integer => quote! { i64 },
            DiscriminantType::String => quote! { &'static str },
            DiscriminantType::None => unreachable!("Discriminant type should not be None here"),
        };
        let ident = &self.ident;
        let cases = self.cases.iter().map(|case| {
            let ident = &case.ident;
            match case
                .discriminant
                .as_ref()
                .expect("Discriminant should be set")
            {
                Discriminant::String(s) => quote! { Self::#ident => #s },
                Discriminant::Integer(i) => quote! { Self::#ident => #i },
            }
        });

        quote! {
            impl Into<#discriminant_type> for #ident {
                fn into(self) -> #discriminant_type {
                    match self {
                        #(
                            #cases,
                        )*
                    }
                }
            }
        }
    }
}

impl ToTokens for Enum<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let class = self.registered_class();
        let enum_impl = self.registered_enum();
        let impl_try_from = self.impl_try_from();
        let impl_into = self.impl_into();

        tokens.extend(quote! {
            #class
            #enum_impl
            #impl_try_from
            #impl_into
        });
    }
}

#[derive(Debug)]
struct EnumCase {
    ident: Ident,
    name: String,
    #[allow(dead_code)]
    attrs: PhpEnumVariantAttribute,
    discriminant: Option<Discriminant>,
    docs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Discriminant {
    String(String),
    Integer(i64),
}

impl TryFrom<&Lit> for Discriminant {
    type Error = syn::Error;

    fn try_from(lit: &Lit) -> Result<Self> {
        match lit {
            Lit::Str(s) => Ok(Discriminant::String(s.value())),
            Lit::Int(i) => i
                .base10_parse::<i64>()
                .map(Discriminant::Integer)
                .map_err(|_| err!(lit => "Invalid integer literal for enum case: {:?}", lit)),
            _ => bail!(lit => "Unsupported discriminant type: {:?}", lit),
        }
    }
}

impl ToTokens for Discriminant {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Discriminant::String(s) => {
                quote! { ::ext_php_rs::enum_::Discriminant::String(#s) }
            }
            Discriminant::Integer(i) => {
                quote! { ::ext_php_rs::enum_::Discriminant::Int(#i) }
            }
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DiscriminantType {
    None,
    String,
    Integer,
}

impl ToTokens for EnumCase {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ident = &self.name;
        let discriminant = self
            .discriminant
            .as_ref()
            .map_or_else(|| quote! { None }, |v| quote! { Some(#v) });
        let docs = &self.docs;

        tokens.extend(quote! {
            ::ext_php_rs::enum_::EnumCase {
                name: #ident,
                discriminant: #discriminant,
                docs: &[#(#docs,)*],
            }
        });
    }
}
