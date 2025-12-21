use std::collections::{HashMap, HashSet};

use crate::class::ClassEntryAttribute;
use crate::constant::PhpConstAttribute;
use crate::function::{Args, Function};
use crate::helpers::{CleanPhpAttr, get_docs};
use darling::FromAttributes;
use darling::util::Flag;
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{Expr, Ident, ItemTrait, Path, TraitItem, TraitItemConst, TraitItemFn};

use crate::impl_::{FnBuilder, MethodModifier};
use crate::parsing::{
    PhpNameContext, PhpRename, RenameRule, Visibility, ident_to_php_name, validate_php_name,
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

struct InterfaceData<'a> {
    ident: &'a Ident,
    name: String,
    path: Path,
    extends: Vec<ClassEntryAttribute>,
    constructor: Option<Function<'a>>,
    methods: Vec<FnBuilder>,
    constants: Vec<Constant<'a>>,
    docs: Vec<String>,
}

impl ToTokens for InterfaceData<'_> {
    #[allow(clippy::too_many_lines)]
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let interface_name = format_ident!("{INTERNAL_INTERFACE_NAME_PREFIX}{}", self.ident);
        let name = &self.name;
        let implements = &self.extends;
        let methods_sig = &self.methods;
        let constants = &self.constants;
        let docs = &self.docs;

        let _constructor = self
            .constructor
            .as_ref()
            .map(|func| func.constructor_meta(&self.path, Some(&Visibility::Public)))
            .option_tokens();

        quote! {
            pub struct #interface_name;

            impl ::ext_php_rs::class::RegisteredClass for #interface_name {
                const CLASS_NAME: &'static str = #name;

                const BUILDER_MODIFIER: Option<
                fn(::ext_php_rs::builders::ClassBuilder) -> ::ext_php_rs::builders::ClassBuilder,
                > = None;

                const EXTENDS: Option<::ext_php_rs::class::ClassEntryInfo> = None;

                const FLAGS: ::ext_php_rs::flags::ClassFlags = ::ext_php_rs::flags::ClassFlags::Interface;

                const IMPLEMENTS: &'static [::ext_php_rs::class::ClassEntryInfo] = &[
                    #(#implements,)*
                ];

                const DOC_COMMENTS: &'static [&'static str] = &[
                    #(#docs,)*
                ];

                fn get_metadata() -> &'static ::ext_php_rs::class::ClassMetadata<Self> {
                    static METADATA: ::ext_php_rs::class::ClassMetadata<#interface_name> =
                    ::ext_php_rs::class::ClassMetadata::new();

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
                )] {
                    &[#(#constants),*]
                }

                fn get_properties<'a>() -> ::std::collections::HashMap<&'static str, ::ext_php_rs::internal::property::PropertyInfo<'a, Self>> {
                    panic!("Not supported for Interface");
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
                const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::Object(Some(
                    <#interface_name as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
                ));
                #[inline]
                fn from_zval(zval: &'a ::ext_php_rs::types::Zval) -> ::std::option::Option<Self> {
                    <Self as ::ext_php_rs::convert::FromZendObject>::from_zend_object(zval.object()?).ok()
                }
            }
            impl<'a> ::ext_php_rs::convert::FromZvalMut<'a> for &'a mut #interface_name {
                const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::Object(Some(
                    <#interface_name as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
                ));
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
                const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::Object(Some(
                    <#interface_name as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
                ));
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
        let interface_name = format_ident!("{INTERNAL_INTERFACE_NAME_PREFIX}{ident}");
        let ts = quote! { #interface_name };
        let path: Path = syn::parse2(ts)?;
        let mut data = InterfaceData {
            ident,
            name,
            path,
            extends: attrs.extends,
            constructor: None,
            methods: Vec::default(),
            constants: Vec::default(),
            docs,
        };

        for item in &mut self.items {
            match item {
                TraitItem::Fn(f) => match parse_trait_item_fn(f, attrs.change_method_case)? {
                    MethodKind::Method(builder) => data.methods.push(builder),
                    MethodKind::Constructor(builder) => {
                        if data.constructor.replace(builder).is_some() {
                            bail!("Only one constructor can be provided per class.");
                        }
                    }
                },
                TraitItem::Const(c) => data
                    .constants
                    .push(parse_trait_item_const(c, attrs.change_constant_case)?),
                _ => {}
            }
        }

        Ok(data)
    }
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

enum MethodKind<'a> {
    Method(FnBuilder),
    Constructor(Function<'a>),
}

fn parse_trait_item_fn(
    fn_item: &mut TraitItemFn,
    change_case: Option<RenameRule>,
) -> Result<MethodKind<'_>> {
    if fn_item.default.is_some() {
        bail!(fn_item => "Interface an not have default impl");
    }

    let php_attr = PhpFunctionInterfaceAttribute::from_attributes(&fn_item.attrs)?;
    fn_item.attrs.clean_php();

    let mut args = Args::parse_from_fnargs(fn_item.sig.inputs.iter(), php_attr.defaults)?;

    let docs = get_docs(&php_attr.attrs)?;

    let mut modifiers: HashSet<MethodModifier> = HashSet::new();
    modifiers.insert(MethodModifier::Abstract);

    if args.typed.first().is_some_and(|arg| arg.name == "self_") {
        args.typed.pop();
    } else if args.receiver.is_none() {
        modifiers.insert(MethodModifier::Static);
    }

    let method_name = php_attr.rename.rename(
        ident_to_php_name(&fn_item.sig.ident),
        change_case.unwrap_or(RenameRule::Camel),
    );
    validate_php_name(
        &method_name,
        PhpNameContext::Method,
        fn_item.sig.ident.span(),
    )?;
    let f = Function::new(&fn_item.sig, method_name, args, php_attr.optional, docs);

    if php_attr.constructor.is_present() {
        Ok(MethodKind::Constructor(f))
    } else {
        let builder = FnBuilder {
            builder: f.abstract_function_builder(),
            vis: php_attr.vis.unwrap_or(Visibility::Public),
            modifiers,
        };

        Ok(MethodKind::Method(builder))
    }
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
            (#name, &#expr, &[#(#docs),*])
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
