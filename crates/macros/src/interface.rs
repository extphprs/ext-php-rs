use std::collections::{BTreeSet, HashMap};

use crate::class::ClassEntryAttribute;
use crate::constant::PhpConstAttribute;
use crate::function::{Args, Function};
use crate::helpers::{CleanPhpAttr, get_docs};
use darling::FromAttributes;
use darling::util::Flag;
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{Expr, Ident, ItemTrait, Path, TraitItem, TraitItemConst, TraitItemFn, TypeParamBound};

use crate::impl_::{FnBuilder, MethodModifier};
use crate::parsing::{
    NameSet, PhpNameContext, PhpRename, RenameRule, Visibility, ident_to_php_name,
    reject_php_attrs, validate_php_name,
};
use crate::prelude::*;

const INTERNAL_INTERFACE_NAME_PREFIX: &str = "PhpInterface";

#[derive(FromAttributes, Debug, Default)]
#[darling(attributes(php), forward_attrs(doc), default)]
pub struct TraitAttributes {
    #[darling(flatten)]
    rename: PhpRename,
    /// Rename methods to match the given rule.
    change_method_case: Option<RenameRule>,
    /// Rename constants to match the given rule.
    change_constant_case: Option<RenameRule>,
    #[darling(multiple)]
    extends: Vec<ClassEntryAttribute>,
    attrs: Vec<syn::Attribute>,
}

pub fn parser(mut input: ItemTrait) -> Result<TokenStream> {
    let interface_data: InterfaceData = input.parse()?;
    let interface_tokens = quote! { #interface_data };

    Ok(quote! {
        #input

        #interface_tokens
    })
}

trait Parse<'a, T> {
    fn parse(&'a mut self) -> Result<T>;
}

/// Represents a supertrait that should be converted to an interface extension.
/// These are automatically detected from Rust trait bounds (e.g., `trait Foo:
/// Bar`).
struct SupertraitInterface {
    /// The full path to the supertrait's PHP interface struct (e.g.,
    /// `PhpInterfaceBar` or `other_module::PhpInterfaceBar`)
    interface_struct_path: Path,
}

impl ToTokens for SupertraitInterface {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let interface_struct_path = &self.interface_struct_path;
        quote! {
            (
                || <#interface_struct_path as ::ext_php_rs::class::RegisteredClass>::get_metadata().ce(),
                <#interface_struct_path as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME
            )
        }
        .to_tokens(tokens);
    }
}

struct InterfaceData<'a> {
    ident: &'a Ident,
    name: String,
    /// Extends from `#[php(extends(...))]` attributes
    extends: Vec<ClassEntryAttribute>,
    /// Extends from Rust trait bounds (supertraits)
    supertrait_extends: Vec<SupertraitInterface>,
    methods: Vec<FnBuilder>,
    method_names: Vec<(Ident, String)>,
    constants: Vec<Constant<'a>>,
    docs: Vec<String>,
}

/// Name of the associated constant on the interface struct that holds the PHP
/// name of the trait method `ident`. `#[php_impl_interface]` reads it, so both
/// sides register the method under one name.
pub fn method_name_const(ident: &Ident) -> Ident {
    format_ident!("__php_method_{}", ident_to_php_name(ident))
}

impl ToTokens for InterfaceData<'_> {
    #[allow(clippy::too_many_lines)]
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let interface_name = format_ident!("{INTERNAL_INTERFACE_NAME_PREFIX}{}", self.ident);
        let name = &self.name;
        let implements = &self.extends;
        let supertrait_implements = &self.supertrait_extends;
        let methods_sig = &self.methods;
        let constants = &self.constants;
        let docs = &self.docs;
        let name_consts = self.method_names.iter().map(|(ident, name)| {
            let const_ident = method_name_const(ident);
            quote! {
                #[doc(hidden)]
                #[allow(non_upper_case_globals)]
                pub const #const_ident: &'static str = #name;
            }
        });

        quote! {
            pub struct #interface_name;

            impl #interface_name {
                #(#name_consts)*
            }

            impl ::ext_php_rs::class::RegisteredClass for #interface_name {
                const CLASS_NAME: &'static str = #name;

                const BUILDER_MODIFIER: Option<
                fn(::ext_php_rs::builders::ClassBuilder) -> ::ext_php_rs::builders::ClassBuilder,
                > = None;

                const EXTENDS: Option<::ext_php_rs::class::ClassEntryInfo> = None;

                const FLAGS: ::ext_php_rs::flags::ClassFlags = ::ext_php_rs::flags::ClassFlags::Interface;

                // Interface inheritance from both explicit #[php(extends(...))] and Rust trait bounds
                const IMPLEMENTS: &'static [::ext_php_rs::class::ClassEntryInfo] = &[
                    #(#implements,)*
                    #(#supertrait_implements,)*
                ];

                const DOC_COMMENTS: &'static [&'static str] = &[
                    #(#docs,)*
                ];

                fn get_metadata() -> &'static ::ext_php_rs::class::ClassMetadata<Self> {
                    static METADATA: ::ext_php_rs::class::ClassMetadata<#interface_name> =
                    ::ext_php_rs::class::ClassMetadata::new(&[]);

                    &METADATA
                }

                fn method_builders() -> Vec<(
                    ::ext_php_rs::builders::FunctionBuilder<'static>,
                    ::ext_php_rs::flags::MethodFlags,
                )> {
                    vec![#(#methods_sig),*]
                }

                fn constructor() -> Option<::ext_php_rs::class::ConstructorMeta<Self>> {
                    None
                }

                fn constants() -> &'static [(
                    &'static str,
                    &'static dyn ext_php_rs::convert::IntoZvalDyn,
                    ext_php_rs::describe::DocComments,
                    ext_php_rs::flags::ConstantFlags,
                )] {
                    &[#(#constants),*]
                }

            }

            impl<'a> ::ext_php_rs::convert::FromZendObject<'a> for &'a #interface_name {
                #[inline]
                fn from_zend_object(
                    obj: &'a ::ext_php_rs::types::ZendObject,
                ) -> ::ext_php_rs::error::Result<Self> {
                    let obj = ::ext_php_rs::types::ZendClassObject::<#interface_name>::from_zend_obj(obj)
                        .ok_or(::ext_php_rs::error::Error::InvalidScope)?;
                    Ok(&**obj)
                }
            }
            impl<'a> ::ext_php_rs::convert::FromZendObjectMut<'a> for &'a mut #interface_name {
                #[inline]
                fn from_zend_object_mut(
                    obj: &'a mut ::ext_php_rs::types::ZendObject,
                ) -> ::ext_php_rs::error::Result<Self> {
                    let obj = ::ext_php_rs::types::ZendClassObject::<#interface_name>::from_zend_obj_mut(obj)
                        .ok_or(::ext_php_rs::error::Error::InvalidScope)?;
                    Ok(&mut **obj)
                }
            }
            impl<'a> ::ext_php_rs::convert::FromZval<'a> for &'a #interface_name {
                const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::object(<#interface_name as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME);
                #[inline]
                fn from_zval(zval: &'a ::ext_php_rs::types::Zval) -> ::std::option::Option<Self> {
                    <Self as ::ext_php_rs::convert::FromZendObject>::from_zend_object(zval.object()?).ok()
                }
            }
            impl<'a> ::ext_php_rs::convert::FromZvalMut<'a> for &'a mut #interface_name {
                const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::object(<#interface_name as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME);
                #[inline]
                fn from_zval_mut(zval: &'a mut ::ext_php_rs::types::Zval) -> ::std::option::Option<Self> {
                    <Self as ::ext_php_rs::convert::FromZendObjectMut>::from_zend_object_mut(zval.object_mut()?)
                        .ok()
                }
            }
            impl ::ext_php_rs::convert::IntoZendObject for #interface_name {
                #[inline]
                fn into_zend_object(
                    self,
                ) -> ::ext_php_rs::error::Result<::ext_php_rs::boxed::ZBox<::ext_php_rs::types::ZendObject>>
                {
                    Ok(::ext_php_rs::types::ZendClassObject::new(self).into())
                }
            }
            impl ::ext_php_rs::convert::IntoZval for #interface_name {
                const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::object(<#interface_name as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME);
                const NULLABLE: bool = false;
                #[inline]
                fn set_zval(
                    self,
                    zv: &mut ::ext_php_rs::types::Zval,
                    persistent: bool,
                ) -> ::ext_php_rs::error::Result<()> {
                    use ::ext_php_rs::convert::IntoZendObject;
                    self.into_zend_object()?.set_zval(zv, persistent)
                }
            }
        }.to_tokens(tokens);
    }
}

impl<'a> Parse<'a, InterfaceData<'a>> for ItemTrait {
    fn parse(&'a mut self) -> Result<InterfaceData<'a>> {
        let attrs = TraitAttributes::from_attributes(&self.attrs)?;
        let ident = &self.ident;
        let name = attrs
            .rename
            .rename(ident_to_php_name(ident), RenameRule::Pascal);
        validate_php_name(&name, PhpNameContext::Interface, ident.span())?;
        let docs = get_docs(&attrs.attrs)?;
        self.attrs.clean_php();
        // Parse supertraits to automatically generate interface inheritance
        let supertrait_extends = parse_supertraits(&self.supertraits);

        let mut data = InterfaceData {
            ident,
            name,
            extends: attrs.extends,
            supertrait_extends,
            methods: Vec::default(),
            method_names: Vec::default(),
            constants: Vec::default(),
            docs,
        };

        let mut method_names = NameSet::case_insensitive();
        let mut constant_names = NameSet::case_sensitive();
        for item in &mut self.items {
            match item {
                TraitItem::Fn(f) => {
                    let (method, name) = parse_trait_item_fn(f, attrs.change_method_case)?;
                    if let Err(earlier) = method_names.insert(&name) {
                        bail!(f.sig.ident => "PHP method `{name}` is already declared as `{earlier}` in this interface. PHP method names ignore case. Rename one of them with `#[php(name = \"...\")]`.");
                    }
                    data.methods.push(method);
                    data.method_names.push((f.sig.ident.clone(), name));
                }
                TraitItem::Const(c) => {
                    let span = c.ident.span();
                    let constant = parse_trait_item_const(c, attrs.change_constant_case)?;
                    if constant_names.insert(&constant.name).is_err() {
                        bail!(span => "PHP constant `{}` is already declared in this interface. Rename one of them with `#[php(name = \"...\")]`.", constant.name);
                    }
                    data.constants.push(constant);
                }
                TraitItem::Type(t) => reject_php_attrs(&t.attrs, "associated types")?,
                _ => {}
            }
        }

        Ok(data)
    }
}

/// Parses the supertraits of a trait definition and converts them to interface
/// extensions. For a trait like `trait Foo: Bar + Baz`, this will generate
/// references to `PhpInterfaceBar` and `PhpInterfaceBaz`.
///
/// This function preserves the full module path, so `trait Foo: other::Bar`
/// will generate `other::PhpInterfaceBar`.
fn parse_supertraits(
    supertraits: &syn::punctuated::Punctuated<TypeParamBound, syn::token::Plus>,
) -> Vec<SupertraitInterface> {
    supertraits
        .iter()
        .filter_map(|bound| {
            if let TypeParamBound::Trait(trait_bound) = bound {
                // Clone the path and modify the last segment to add
                // PhpInterface prefix
                let mut path = trait_bound.path.clone();
                let last_segment = path.segments.last_mut()?;
                last_segment.ident =
                    format_ident!("{}{}", INTERNAL_INTERFACE_NAME_PREFIX, last_segment.ident);
                Some(SupertraitInterface {
                    interface_struct_path: path,
                })
            } else {
                None
            }
        })
        .collect()
}

#[derive(FromAttributes, Default, Debug)]
#[darling(default, attributes(php), forward_attrs(doc))]
pub struct PhpFunctionInterfaceAttribute {
    #[darling(flatten)]
    rename: PhpRename,
    defaults: HashMap<Ident, Expr>,
    optional: Option<Ident>,
    vis: Option<Visibility>,
    attrs: Vec<syn::Attribute>,
    getter: Flag,
    setter: Flag,
    constructor: Flag,
}

fn parse_trait_item_fn(
    fn_item: &mut TraitItemFn,
    change_case: Option<RenameRule>,
) -> Result<(FnBuilder, String)> {
    if fn_item.default.is_some() {
        bail!(fn_item => "Interface methods cannot have a default implementation.");
    }

    let php_attr = PhpFunctionInterfaceAttribute::from_attributes(&fn_item.attrs)?;
    for flag in [&php_attr.getter, &php_attr.setter] {
        if flag.is_present() {
            bail!(flag.span() => "Interfaces cannot declare property accessors; declare a plain method instead.");
        }
    }
    if php_attr.constructor.is_present() {
        bail!(php_attr.constructor.span() => "Constructors are not supported on interfaces.");
    }
    fn_item.attrs.clean_php();

    let mut args = Args::parse_from_fnargs(fn_item.sig.inputs.iter(), php_attr.defaults)?;

    let docs = get_docs(&php_attr.attrs)?;

    let mut modifiers: BTreeSet<MethodModifier> = BTreeSet::new();
    modifiers.insert(MethodModifier::Abstract);

    if !args.take_self_object()? && args.receiver.is_none() {
        modifiers.insert(MethodModifier::Static);
    }

    let method_name = php_attr.rename.rename_method(
        ident_to_php_name(&fn_item.sig.ident),
        change_case.unwrap_or(RenameRule::Camel),
    );
    validate_php_name(
        &method_name,
        PhpNameContext::Method,
        fn_item.sig.ident.span(),
    )?;
    let f = Function::new(
        &fn_item.sig,
        method_name.clone(),
        args,
        php_attr.optional,
        docs,
    )?;

    Ok((
        FnBuilder {
            builder: f.abstract_function_builder(),
            vis: php_attr.vis.unwrap_or(Visibility::Public),
            modifiers,
        },
        method_name,
    ))
}

#[derive(Debug)]
struct Constant<'a> {
    name: String,
    expr: &'a Expr,
    docs: Vec<String>,
}

impl ToTokens for Constant<'_> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let expr = &self.expr;
        let docs = &self.docs;
        quote! {
            (#name, &#expr, &[#(#docs),*], ::ext_php_rs::flags::ConstantFlags::Public)
        }
        .to_tokens(tokens);
    }
}

impl<'a> Constant<'a> {
    fn new(name: String, expr: &'a Expr, docs: Vec<String>) -> Self {
        Self { name, expr, docs }
    }
}

fn parse_trait_item_const(
    const_item: &mut TraitItemConst,
    change_case: Option<RenameRule>,
) -> Result<Constant<'_>> {
    if const_item.default.is_none() {
        bail!(const_item => "PHP Interface const can not be empty");
    }

    let attr = PhpConstAttribute::from_attributes(&const_item.attrs)?;
    if let Some(vis) = &attr.vis {
        bail!(vis.span() => "Interface constants are always public; remove `vis`.");
    }
    let name = attr.rename.rename(
        ident_to_php_name(&const_item.ident),
        change_case.unwrap_or(RenameRule::ScreamingSnake),
    );
    validate_php_name(&name, PhpNameContext::Constant, const_item.ident.span())?;
    let docs = get_docs(&attr.attrs)?;
    const_item.attrs.clean_php();

    let (_, expr) = const_item.default.as_ref().unwrap();
    Ok(Constant::new(name, expr, docs))
}

#[cfg(test)]
mod tests {
    use super::method_name_const;

    #[test]
    fn method_name_const_strips_raw_prefix() {
        assert_eq!(
            method_name_const(&syn::parse_quote!(r#type)).to_string(),
            "__php_method_type"
        );
        assert_eq!(
            method_name_const(&syn::parse_quote!(my_method)).to_string(),
            "__php_method_my_method"
        );
    }
}
