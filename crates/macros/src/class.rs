use darling::util::Flag;
use darling::{FromAttributes, FromMeta, ToTokens};
use proc_macro2::TokenStream;
use quote::{TokenStreamExt, quote};
use syn::{Attribute, Expr, Fields, ItemStruct};

use crate::helpers::get_docs;
use crate::parsing::{PhpNameContext, PhpRename, RenameRule, ident_to_php_name, validate_php_name};
use crate::prelude::*;

#[derive(FromAttributes, Debug, Default)]
#[darling(attributes(php), forward_attrs(doc), default)]
pub struct StructAttributes {
    /// The name of the PHP class. Defaults to the same name as the struct.
    #[darling(flatten)]
    rename: PhpRename,
    /// A modifier function which should accept one argument, a `ClassBuilder`,
    /// and return the same object. Allows the user to modify the class before
    /// it is built.
    modifier: Option<syn::Ident>,
    /// An expression of `ClassFlags` to be applied to the class.
    flags: Option<syn::Expr>,
    extends: Option<ClassEntryAttribute>,
    #[darling(multiple)]
    implements: Vec<ClassEntryAttribute>,
    attrs: Vec<Attribute>,
}

#[derive(FromMeta, Debug)]
pub struct ClassEntryAttribute {
    pub ce: syn::Expr,
    pub stub: String,
}

impl ToTokens for ClassEntryAttribute {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let ce = &self.ce;
        let stub = &self.stub;
        let token = quote! { (#ce, #stub) };
        tokens.append_all(token);
    }
}

pub fn parser(mut input: ItemStruct) -> Result<TokenStream> {
    let attr = StructAttributes::from_attributes(&input.attrs)?;
    let ident = &input.ident;
    let name = attr
        .rename
        .rename(ident_to_php_name(ident), RenameRule::Pascal);
    validate_php_name(&name, PhpNameContext::Class, ident.span())?;
    let docs = get_docs(&attr.attrs)?;
    input.attrs.retain(|attr| !attr.path().is_ident("php"));

    let fields = match &mut input.fields {
        Fields::Named(fields) => parse_fields(fields.named.iter_mut())?,
        _ => vec![],
    };

    let class_impl = generate_registered_class_impl(
        ident,
        &name,
        attr.modifier.as_ref(),
        attr.extends.as_ref(),
        &attr.implements,
        &fields,
        attr.flags.as_ref(),
        &docs,
    );

    Ok(quote! {
        #input
        #class_impl

        ::ext_php_rs::class_derives!(#ident);
    })
}

#[derive(FromAttributes, Debug, Default)]
#[darling(attributes(php), forward_attrs(doc), default)]
struct PropAttributes {
    prop: Flag,
    #[darling(rename = "static")]
    static_: Flag,
    #[darling(flatten)]
    rename: PhpRename,
    flags: Option<Expr>,
    default: Option<Expr>,
    attrs: Vec<Attribute>,
}

fn parse_fields<'a>(fields: impl Iterator<Item = &'a mut syn::Field>) -> Result<Vec<Property<'a>>> {
    let mut result = vec![];
    for field in fields {
        let attr = PropAttributes::from_attributes(&field.attrs)?;
        if attr.prop.is_present() {
            let ident = field
                .ident
                .as_ref()
                .ok_or_else(|| err!("Only named fields can be properties."))?;
            let docs = get_docs(&attr.attrs)?;
            field.attrs.retain(|attr| !attr.path().is_ident("php"));

            let name = attr
                .rename
                .rename(ident_to_php_name(ident), RenameRule::Camel);
            validate_php_name(&name, PhpNameContext::Property, ident.span())?;

            result.push(Property {
                ident,
                name,
                attr,
                docs,
            });
        }
    }

    Ok(result)
}

#[derive(Debug)]
struct Property<'a> {
    pub ident: &'a syn::Ident,
    pub name: String,
    pub attr: PropAttributes,
    pub docs: Vec<String>,
}

impl Property<'_> {
    pub fn is_static(&self) -> bool {
        self.attr.static_.is_present()
    }
}

/// Generates an implementation of `RegisteredClass` for struct `ident`.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn generate_registered_class_impl(
    ident: &syn::Ident,
    class_name: &str,
    modifier: Option<&syn::Ident>,
    extends: Option<&ClassEntryAttribute>,
    implements: &[ClassEntryAttribute],
    fields: &[Property],
    flags: Option<&syn::Expr>,
    docs: &[String],
) -> TokenStream {
    let modifier = modifier.option_tokens();

    // Separate instance properties from static properties
    let (instance_props, static_props): (Vec<_>, Vec<_>) =
        fields.iter().partition(|prop| !prop.is_static());

    // Generate instance properties (with Rust handlers)
    let instance_fields = instance_props.iter().map(|prop| {
        let name = &prop.name;
        let field_ident = prop.ident;
        let flags = prop
            .attr
            .flags
            .as_ref()
            .map(ToTokens::to_token_stream)
            .unwrap_or(quote! { ::ext_php_rs::flags::PropertyFlags::Public });
        let docs = &prop.docs;

        quote! {
            (#name, ::ext_php_rs::internal::property::PropertyInfo {
                prop: ::ext_php_rs::props::Property::field(|this: &mut Self| &mut this.#field_ident),
                flags: #flags,
                docs: &[#(#docs,)*]
            })
        }
    });

    // Generate static properties (PHP-managed, no Rust handlers)
    // We combine the base flags with Static flag using from_bits_retain which is
    // const
    let static_fields = static_props.iter().map(|prop| {
        let name = &prop.name;
        let base_flags = prop
            .attr
            .flags
            .as_ref()
            .map(ToTokens::to_token_stream)
            .unwrap_or(quote! { ::ext_php_rs::flags::PropertyFlags::Public });
        let docs = &prop.docs;

        // Handle default value - if provided, wrap in Some(&value), otherwise None
        let default_value = if let Some(expr) = &prop.attr.default {
            quote! { ::std::option::Option::Some(&#expr as &'static (dyn ::ext_php_rs::convert::IntoZvalDyn + Sync)) }
        } else {
            quote! { ::std::option::Option::None }
        };

        // Use from_bits_retain to combine flags in a const context
        quote! {
            (#name, ::ext_php_rs::flags::PropertyFlags::from_bits_retain(
                (#base_flags).bits() | ::ext_php_rs::flags::PropertyFlags::Static.bits()
            ), #default_value, &[#(#docs,)*] as &[&str])
        }
    });

    let flags = match flags {
        Some(flags) => flags.to_token_stream(),
        None => quote! { ::ext_php_rs::flags::ClassFlags::empty() }.to_token_stream(),
    };

    let docs = quote! {
        #(#docs,)*
    };

    let extends = if let Some(extends) = extends {
        quote! {
            Some(#extends)
        }
    } else {
        quote! { None }
    };

    let implements = implements.iter().map(|imp| {
        let ce = &imp.ce;
        let stub = &imp.stub;
        quote! {
            (#ce, #stub)
        }
    });

    quote! {
        impl ::ext_php_rs::class::RegisteredClass for #ident {
            const CLASS_NAME: &'static str = #class_name;
            const BUILDER_MODIFIER: ::std::option::Option<
                fn(::ext_php_rs::builders::ClassBuilder) -> ::ext_php_rs::builders::ClassBuilder
            > = #modifier;
            const EXTENDS: ::std::option::Option<
                ::ext_php_rs::class::ClassEntryInfo
            > = #extends;
            const IMPLEMENTS: &'static [::ext_php_rs::class::ClassEntryInfo] = &[
                #(#implements,)*
            ];
            const FLAGS: ::ext_php_rs::flags::ClassFlags = #flags;
            const DOC_COMMENTS: &'static [&'static str] = &[
                #docs
            ];

            #[inline]
            fn get_metadata() -> &'static ::ext_php_rs::class::ClassMetadata<Self> {
                static METADATA: ::ext_php_rs::class::ClassMetadata<#ident> =
                    ::ext_php_rs::class::ClassMetadata::new();
                &METADATA
            }

            fn get_properties<'a>() -> ::std::collections::HashMap<
                &'static str, ::ext_php_rs::internal::property::PropertyInfo<'a, Self>
            > {
                use ::std::iter::FromIterator;
                ::std::collections::HashMap::from_iter([
                    #(#instance_fields,)*
                ])
            }

            #[must_use]
            fn static_properties() -> &'static [(&'static str, ::ext_php_rs::flags::PropertyFlags, ::std::option::Option<&'static (dyn ::ext_php_rs::convert::IntoZvalDyn + Sync)>, &'static [&'static str])] {
                static STATIC_PROPS: &[(&str, ::ext_php_rs::flags::PropertyFlags, ::std::option::Option<&'static (dyn ::ext_php_rs::convert::IntoZvalDyn + Sync)>, &[&str])] = &[#(#static_fields,)*];
                STATIC_PROPS
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
                use ::ext_php_rs::internal::class::PhpClassImpl;
                ::ext_php_rs::internal::class::PhpClassImplCollector::<Self>::default().get_constructor()
            }

            #[inline]
            fn constants() -> &'static [(&'static str, &'static dyn ::ext_php_rs::convert::IntoZvalDyn, &'static [&'static str])] {
                use ::ext_php_rs::internal::class::PhpClassImpl;
                ::ext_php_rs::internal::class::PhpClassImplCollector::<Self>::default().get_constants()
            }
        }
    }
}
