use darling::FromAttributes;
use darling::util::{Flag, SpannedValue};
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::{HashMap, HashSet};
use syn::{Expr, Ident, ItemImpl};

use crate::constant::PhpConstAttribute;
use crate::function::{Args, CallType, Function, MethodReceiver};
use crate::helpers::get_docs;
use crate::parsing::{
    PhpNameContext, PhpRename, RenameRule, Visibility, ident_to_php_name, reject_php_attrs,
    validate_php_name,
};
use crate::prelude::*;

/// Method types.
#[derive(Debug)]
enum MethodTy {
    /// Regular PHP method.
    Normal,
    /// Constructor method.
    Constructor,
    /// Property getter method.
    Getter,
    /// Property setter method.
    Setter,
    /// Abstract method.
    Abstract,
}

#[derive(FromAttributes, Debug, Default)]
#[darling(attributes(php), default)]
pub struct PhpImpl {
    /// Rename methods to match the given rule.
    change_method_case: Option<RenameRule>,
    /// Rename constants to match the given rule.
    change_constant_case: Option<RenameRule>,
}

pub fn parser(mut input: ItemImpl) -> Result<TokenStream> {
    let args = PhpImpl::from_attributes(&input.attrs)?;
    input.attrs.retain(|attr| !attr.path().is_ident("php"));
    let path = match &*input.self_ty {
        syn::Type::Path(ty) => &ty.path,
        _ => {
            bail!(input.self_ty => "The `#[php_impl]` attribute is only valid for struct implementations.")
        }
    };

    let mut parsed = ParsedImpl::new(
        path,
        args.change_method_case.unwrap_or(RenameRule::Camel),
        args.change_constant_case
            .unwrap_or(RenameRule::ScreamingSnake),
    );
    parsed.parse(input.items.iter_mut())?;

    let php_class_impl = parsed.generate_php_class_impl();
    Ok(quote::quote! {
        #input
        #php_class_impl
    })
}

/// Arguments applied to methods.
#[derive(Debug)]
struct MethodArgs {
    /// Method name. Only applies to PHP (not the Rust method name).
    name: String,
    /// The first optional argument of the function signature.
    optional: Option<Ident>,
    /// Default values for optional arguments.
    defaults: HashMap<Ident, Expr>,
    /// Visibility of the method (public, protected, private).
    vis: Visibility,
    /// Span of the explicit `vis` option, when given.
    vis_span: Option<proc_macro2::Span>,
    /// Method type.
    ty: MethodTy,
    /// Whether this is a final method.
    is_final: bool,
}

#[derive(FromAttributes, Default, Debug)]
#[darling(default, attributes(php), forward_attrs(doc))]
pub struct PhpFunctionImplAttribute {
    #[darling(flatten)]
    rename: PhpRename,
    defaults: HashMap<Ident, Expr>,
    optional: Option<Ident>,
    vis: Option<SpannedValue<Visibility>>,
    attrs: Vec<syn::Attribute>,
    getter: Flag,
    setter: Flag,
    constructor: Flag,
    #[darling(rename = "abstract")]
    abstract_method: Flag,
    #[darling(rename = "final")]
    final_method: Flag,
}

impl MethodArgs {
    #[allow(clippy::similar_names)]
    fn new(name: String, attr: PhpFunctionImplAttribute) -> Result<Self> {
        let is_constructor = name == "__construct" || attr.constructor.is_present();
        let is_getter = attr.getter.is_present();
        let is_setter = attr.setter.is_present();
        let is_abstract = attr.abstract_method.is_present();
        let is_final = attr.final_method.is_present();

        let abstract_span = attr.abstract_method.span();
        let final_span = attr.final_method.span();
        if is_getter && is_setter {
            bail!(attr.setter.span() => "A method cannot be both a getter and a setter.");
        }
        if is_constructor {
            if is_abstract {
                bail!(abstract_span => "Constructors cannot be abstract.");
            }
            if is_final {
                bail!(final_span => "Constructors cannot be final.");
            }
        }
        if is_getter {
            if is_abstract {
                bail!(abstract_span => "Getters cannot be abstract.");
            }
            if is_final {
                bail!(final_span => "Getters cannot be final.");
            }
        }
        if is_setter {
            if is_abstract {
                bail!(abstract_span => "Setters cannot be abstract.");
            }
            if is_final {
                bail!(final_span => "Setters cannot be final.");
            }
        }
        if is_abstract {
            if is_final {
                bail!(final_span => "Methods cannot be both abstract and final.");
            }
            if let Some(vis) = attr.vis.filter(|vis| **vis == Visibility::Private) {
                bail!(vis.span() => "Abstract methods cannot be private.");
            }
        }

        let ty = if is_constructor {
            MethodTy::Constructor
        } else if is_getter {
            MethodTy::Getter
        } else if is_setter {
            MethodTy::Setter
        } else if is_abstract {
            MethodTy::Abstract
        } else {
            MethodTy::Normal
        };

        Ok(Self {
            name,
            optional: attr.optional,
            defaults: attr.defaults,
            vis_span: attr.vis.as_ref().map(SpannedValue::span),
            vis: attr.vis.map_or(Visibility::Public, |vis| *vis),
            ty,
            is_final,
        })
    }
}

/// A PHP property backed by getter and/or setter methods, in declaration order.
#[derive(Debug)]
struct PropGroup<'a> {
    /// Property name in PHP (e.g., "name" for `get_name`/`set_name`).
    name: String,
    getter: Option<&'a syn::Ident>,
    setter: Option<&'a syn::Ident>,
    /// Visibility of the property.
    vis: Visibility,
    /// Documentation comments for the property.
    docs: Vec<String>,
    value_ty: Option<&'a syn::Type>,
}

#[derive(Debug)]
struct ParsedImpl<'a> {
    path: &'a syn::Path,
    change_method_case: RenameRule,
    change_constant_case: RenameRule,
    functions: Vec<FnBuilder>,
    constructor: Option<(Function<'a>, Option<Visibility>)>,
    constants: Vec<Constant<'a>>,
    has_abstract_methods: bool,
    /// Properties backed by getter/setter methods, in declaration order.
    properties: Vec<PropGroup<'a>>,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum MethodModifier {
    Abstract,
    Static,
    Final,
}

impl quote::ToTokens for MethodModifier {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match *self {
            Self::Abstract => quote! { ::ext_php_rs::flags::MethodFlags::Abstract },
            Self::Static => quote! { ::ext_php_rs::flags::MethodFlags::Static },
            Self::Final => quote! { ::ext_php_rs::flags::MethodFlags::Final },
        }
        .to_tokens(tokens);
    }
}

#[derive(Debug)]
pub struct FnBuilder {
    /// Tokens which represent the `FunctionBuilder` for this function.
    pub builder: TokenStream,
    /// The visibility of this method.
    pub vis: Visibility,
    /// Whether this method is abstract.
    pub modifiers: HashSet<MethodModifier>,
}

#[derive(Debug)]
pub struct Constant<'a> {
    /// Name of the constant in PHP land.
    pub name: String,
    /// Identifier of the constant in Rust land.
    pub ident: &'a syn::Ident,
    /// Documentation for the constant.
    pub docs: Vec<String>,
    /// PHP visibility of the constant.
    pub vis: Visibility,
}

impl<'a> ParsedImpl<'a> {
    /// Create a new, empty parsed impl block.
    ///
    /// # Parameters
    ///
    /// * `path` - Path of the type the `impl` block is for.
    /// * `rename_methods` - Rule to rename methods with.
    /// * `rename_constants` - Rule to rename constants with.
    fn new(path: &'a syn::Path, rename_methods: RenameRule, rename_constants: RenameRule) -> Self {
        Self {
            path,
            change_method_case: rename_methods,
            change_constant_case: rename_constants,
            functions: Vec::default(),
            constructor: Option::default(),
            constants: Vec::default(),
            has_abstract_methods: false,
            properties: Vec::default(),
        }
    }

    fn parse_property_method(
        &mut self,
        method: &'a syn::ImplItemFn,
        opts: &MethodArgs,
        rename: &PhpRename,
        docs: Vec<String>,
    ) -> Result<()> {
        let is_getter = matches!(opts.ty, MethodTy::Getter);
        let method_name = ident_to_php_name(&method.sig.ident);
        let prefix = if is_getter { "get_" } else { "set_" };
        let prop_name = rename.rename(
            method_name.strip_prefix(prefix).unwrap_or(&method_name),
            RenameRule::Camel,
        );
        validate_php_name(
            &prop_name,
            PhpNameContext::Property,
            method.sig.ident.span(),
        )?;

        let value_ty = match (is_getter, &method.sig.output) {
            (true, syn::ReturnType::Type(_, ty)) => Some(ty.as_ref()),
            (false, _) => method.sig.inputs.iter().nth(1).and_then(|arg| match arg {
                syn::FnArg::Typed(pat) => Some(pat.ty.as_ref()),
                syn::FnArg::Receiver(_) => None,
            }),
            _ => None,
        };

        let idx = self
            .properties
            .iter()
            .position(|g| g.name == prop_name)
            .unwrap_or_else(|| {
                self.properties.push(PropGroup {
                    name: prop_name,
                    getter: None,
                    setter: None,
                    vis: opts.vis,
                    docs: Vec::new(),
                    value_ty: None,
                });
                self.properties.len() - 1
            });
        let group = &mut self.properties[idx];
        let kind = if is_getter { "getter" } else { "setter" };
        let slot = if is_getter {
            &mut group.getter
        } else {
            &mut group.setter
        };
        if slot.is_some() {
            bail!(method.sig.ident => "Property `{}` already has a {kind}.", group.name);
        }
        if group.vis != opts.vis {
            let span = opts.vis_span.unwrap_or_else(|| method.sig.ident.span());
            bail!(span => "The getter and setter of property `{}` must have the same visibility.", group.name);
        }
        *slot = Some(&method.sig.ident);
        if (is_getter && value_ty.is_some()) || group.value_ty.is_none() {
            group.value_ty = value_ty;
        }
        if (is_getter && !docs.is_empty()) || group.docs.is_empty() {
            group.docs = docs;
        }
        Ok(())
    }

    /// Parses an impl block from `items`, populating `self`.
    fn parse(&mut self, items: impl Iterator<Item = &'a mut syn::ImplItem>) -> Result<()> {
        for items in items {
            match items {
                syn::ImplItem::Const(c) => {
                    let attr = PhpConstAttribute::from_attributes(&c.attrs)?;
                    let name = attr
                        .rename
                        .rename(ident_to_php_name(&c.ident), self.change_constant_case);
                    validate_php_name(&name, PhpNameContext::Constant, c.ident.span())?;
                    let docs = get_docs(&attr.attrs)?;
                    c.attrs.retain(|attr| !attr.path().is_ident("php"));

                    self.constants.push(Constant {
                        name,
                        ident: &c.ident,
                        docs,
                        vis: attr.vis.map_or(Visibility::Public, |vis| *vis),
                    });
                }
                syn::ImplItem::Fn(method) => {
                    let mut attr = PhpFunctionImplAttribute::from_attributes(&method.attrs)?;
                    let name = attr.rename.rename_method(
                        ident_to_php_name(&method.sig.ident),
                        self.change_method_case,
                    );
                    validate_php_name(&name, PhpNameContext::Method, method.sig.ident.span())?;
                    let docs = get_docs(&attr.attrs)?;
                    method.attrs.retain(|attr| !attr.path().is_ident("php"));

                    let rename = std::mem::take(&mut attr.rename);
                    let opts = MethodArgs::new(name, attr)?;

                    // Handle getter/setter methods
                    if matches!(opts.ty, MethodTy::Getter | MethodTy::Setter) {
                        self.parse_property_method(method, &opts, &rename, docs)?;
                        continue;
                    }

                    let args = Args::parse_from_fnargs(method.sig.inputs.iter(), opts.defaults)?;
                    let mut func = Function::new(&method.sig, opts.name, args, opts.optional, docs);

                    let mut modifiers: HashSet<MethodModifier> = HashSet::new();

                    if matches!(opts.ty, MethodTy::Constructor) {
                        if self.constructor.replace((func, opts.vis.into())).is_some() {
                            bail!(method => "Only one constructor can be provided per class.");
                        }
                    } else {
                        let call_type = CallType::Method {
                            class: self.path,
                            receiver: if func.args.receiver.is_some() {
                                // `&self` or `&mut self`
                                MethodReceiver::Class
                            } else if func
                                .args
                                .typed
                                .first()
                                .is_some_and(|arg| arg.name == "self_")
                            {
                                // `self_: &[mut] ZendClassObject<Self>`
                                // Need to remove arg from argument list
                                func.args.typed.remove(0);
                                MethodReceiver::ZendClassObject
                            } else {
                                modifiers.insert(MethodModifier::Static);
                                // Static method
                                MethodReceiver::Static
                            },
                        };
                        let is_abstract = matches!(opts.ty, MethodTy::Abstract);
                        if is_abstract {
                            modifiers.insert(MethodModifier::Abstract);
                            self.has_abstract_methods = true;
                        }
                        if opts.is_final {
                            modifiers.insert(MethodModifier::Final);
                        }

                        // Abstract methods use a different builder that doesn't
                        // generate a handler
                        let builder = if is_abstract {
                            func.abstract_function_builder()
                        } else {
                            func.function_builder(&call_type)
                        };

                        self.functions.push(FnBuilder {
                            builder,
                            vis: opts.vis,
                            modifiers,
                        });
                    }
                }
                syn::ImplItem::Type(t) => {
                    reject_php_attrs(&t.attrs, "associated types")?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Generates an `impl PhpClassImpl<Self> for PhpClassImplCollector<Self>`
    /// block.
    #[allow(clippy::too_many_lines)]
    fn generate_php_class_impl(&self) -> TokenStream {
        let path = &self.path;
        let functions = &self.functions;
        let constructor = self
            .constructor
            .as_ref()
            .map(|(func, vis)| func.constructor_meta(self.path, vis.as_ref()))
            .option_tokens();
        let constants = self.constants.iter().map(|c| {
            let name = &c.name;
            let ident = c.ident;
            let docs = &c.docs;
            let flags = c
                .vis
                .flag_tokens(&quote! { ::ext_php_rs::flags::ConstantFlags });
            quote! {
                (#name, &#path::#ident, &[#(#docs),*], #flags)
            }
        });

        // Compile-time check: abstract methods can only be in abstract classes
        let abstract_check = if self.has_abstract_methods {
            quote! {
                const _: () = assert!(
                    <#path as ::ext_php_rs::class::RegisteredClass>::FLAGS
                        .contains(::ext_php_rs::flags::ClassFlags::Abstract),
                    "Abstract methods can only be defined in abstract classes. \
                     Add `#[php(flags = ClassFlags::Abstract)]` to the class definition."
                );
            }
        } else {
            quote! {}
        };

        // Generate static PropertyDescriptor entries for method properties.
        // Each getter/setter pair gets wrapper fn pointers and a descriptor.
        let method_prop_data: Vec<(Vec<TokenStream>, TokenStream)> = self
            .properties
            .iter()
            .enumerate()
            .map(|(i, group)| {
                let prop_name = &group.name;
                let flags = match group.vis {
                    Visibility::Public => quote! { ::ext_php_rs::flags::PropertyFlags::Public },
                    Visibility::Protected => {
                        quote! { ::ext_php_rs::flags::PropertyFlags::Protected }
                    }
                    Visibility::Private => quote! { ::ext_php_rs::flags::PropertyFlags::Private },
                };
                let docs = &group.docs;
                let readonly = group.getter.is_some() && group.setter.is_none();
                let type_tokens = group.value_ty.map_or_else(
                    || {
                        quote! {
                            ty: ::ext_php_rs::flags::DataType::Mixed,
                            nullable: false,
                        }
                    },
                    |vty| {
                        quote! {
                            ty: <#vty as ::ext_php_rs::convert::IntoZval>::TYPE,
                            nullable: <#vty as ::ext_php_rs::convert::IntoZval>::NULLABLE,
                        }
                    },
                );

                let mut fn_defs = Vec::new();

                // Generate getter wrapper
                let getter_ref = if let Some(getter_ident) = group.getter {
                    let getter_fn_name =
                        syn::Ident::new(&format!("__method_get_{i}"), getter_ident.span());
                    fn_defs.push(quote! {
                        fn #getter_fn_name(
                            this: &#path,
                            __zv: &mut ::ext_php_rs::types::Zval,
                        ) -> ::ext_php_rs::exception::PhpResult {
                            use ::ext_php_rs::convert::IntoZval as _;
                            let value = #path::#getter_ident(this);
                            value.set_zval(__zv, false)
                                .map_err(|e| format!("Failed to return property value: {e:?}"))?;
                            Ok(())
                        }
                    });
                    quote! { ::std::option::Option::Some(#getter_fn_name) }
                } else {
                    quote! { ::std::option::Option::None }
                };

                // Generate setter wrapper
                let setter_ref = if let Some(setter_ident) = group.setter {
                    let setter_fn_name =
                        syn::Ident::new(&format!("__method_set_{i}"), setter_ident.span());
                    if let Some(vty) = group.value_ty {
                        fn_defs.push(quote! {
                            fn #setter_fn_name(
                                this: &mut #path,
                                __zv: &::ext_php_rs::types::Zval,
                            ) -> ::ext_php_rs::exception::PhpResult {
                                use ::ext_php_rs::convert::FromZval as _;
                                let val = <#vty as ::ext_php_rs::convert::FromZval>::from_zval(__zv)
                                    .ok_or("Unable to convert property value into required type.")?;
                                #path::#setter_ident(this, val);
                                Ok(())
                            }
                        });
                    } else {
                        fn_defs.push(quote! {
                            fn #setter_fn_name(
                                this: &mut #path,
                                __zv: &::ext_php_rs::types::Zval,
                            ) -> ::ext_php_rs::exception::PhpResult {
                                use ::ext_php_rs::convert::FromZval as _;
                                let val = ::ext_php_rs::convert::FromZval::from_zval(__zv)
                                    .ok_or("Unable to convert property value into required type.")?;
                                #path::#setter_ident(this, val);
                                Ok(())
                            }
                        });
                    }
                    quote! { ::std::option::Option::Some(#setter_fn_name) }
                } else {
                    quote! { ::std::option::Option::None }
                };

                let descriptor = quote! {
                    ::ext_php_rs::internal::property::PropertyDescriptor {
                        name: #prop_name,
                        get: #getter_ref,
                        set: #setter_ref,
                        flags: #flags,
                        docs: &[#(#docs),*],
                        #type_tokens
                        readonly: #readonly,
                    }
                };

                (fn_defs, descriptor)
            })
            .collect();

        let method_prop_fn_defs: Vec<&TokenStream> = method_prop_data
            .iter()
            .flat_map(|(fns, _)| fns.iter())
            .collect();
        let method_prop_descriptors: Vec<&TokenStream> =
            method_prop_data.iter().map(|(_, d)| d).collect();
        let method_prop_count = method_prop_descriptors.len();

        quote! {
            #abstract_check

            impl ::ext_php_rs::internal::class::PhpClassImpl<#path>
                for ::ext_php_rs::internal::class::PhpClassImplCollector<#path>
            {
                fn get_methods(self) -> ::std::vec::Vec<
                    (::ext_php_rs::builders::FunctionBuilder<'static>, ::ext_php_rs::flags::MethodFlags)
                > {
                    vec![#(#functions),*]
                }

                fn get_method_props(self) -> &'static [::ext_php_rs::internal::property::PropertyDescriptor<#path>] {
                    #(#method_prop_fn_defs)*

                    static METHOD_PROPS: [
                        ::ext_php_rs::internal::property::PropertyDescriptor<#path>; #method_prop_count
                    ] = [
                        #(#method_prop_descriptors,)*
                    ];
                    &METHOD_PROPS
                }

                fn get_constructor(self) -> ::std::option::Option<::ext_php_rs::class::ConstructorMeta<#path>> {
                    #constructor
                }

                fn get_constants(self) -> &'static [(&'static str, &'static dyn ::ext_php_rs::convert::IntoZvalDyn, &'static [&'static str], ::ext_php_rs::flags::ConstantFlags)] {
                    &[#(#constants),*]
                }
            }
        }
    }
}

impl quote::ToTokens for FnBuilder {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let builder = &self.builder;
        // TODO(cole_d): allow more flags via attributes
        let mut flags = vec![];
        let vis = &self.vis;
        flags.push(quote! { #vis });
        for flag in &self.modifiers {
            flags.push(quote! { #flag });
        }

        quote! {
            (#builder, #(#flags)|*)
        }
        .to_tokens(tokens);
    }
}
