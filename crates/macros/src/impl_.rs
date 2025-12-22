use darling::FromAttributes;
use darling::util::Flag;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::{HashMap, HashSet};
use syn::{Expr, Ident, ItemImpl};

use crate::constant::PhpConstAttribute;
use crate::function::{Args, CallType, Function, MethodReceiver};
use crate::helpers::get_docs;
use crate::parsing::{PhpRename, RenameRule, Visibility};
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
    /// Method type.
    ty: MethodTy,
}

#[derive(FromAttributes, Default, Debug)]
#[darling(default, attributes(php), forward_attrs(doc))]
pub struct PhpFunctionImplAttribute {
    #[darling(flatten)]
    rename: PhpRename,
    defaults: HashMap<Ident, Expr>,
    optional: Option<Ident>,
    vis: Option<Visibility>,
    attrs: Vec<syn::Attribute>,
    getter: Flag,
    setter: Flag,
    constructor: Flag,
    abstract_method: Flag,
}

impl MethodArgs {
    fn new(name: String, attr: PhpFunctionImplAttribute) -> Self {
        let ty = if name == "__construct" || attr.constructor.is_present() {
            MethodTy::Constructor
        } else if attr.getter.is_present() {
            MethodTy::Getter
        } else if attr.setter.is_present() {
            MethodTy::Setter
        } else if attr.abstract_method.is_present() {
            MethodTy::Abstract
        } else {
            MethodTy::Normal
        };

        Self {
            name,
            optional: attr.optional,
            defaults: attr.defaults,
            vis: attr.vis.unwrap_or(Visibility::Public),
            ty,
        }
    }
}

#[derive(Debug)]
struct ParsedImpl<'a> {
    path: &'a syn::Path,
    change_method_case: RenameRule,
    change_constant_case: RenameRule,
    functions: Vec<FnBuilder>,
    constructor: Option<(Function<'a>, Option<Visibility>)>,
    constants: Vec<Constant<'a>>,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub enum MethodModifier {
    Abstract,
    Static,
}

impl quote::ToTokens for MethodModifier {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match *self {
            Self::Abstract => quote! { ::ext_php_rs::flags::MethodFlags::Abstract },
            Self::Static => quote! { ::ext_php_rs::flags::MethodFlags::Static },
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
        }
    }

    /// Parses an impl block from `items`, populating `self`.
    fn parse(&mut self, items: impl Iterator<Item = &'a mut syn::ImplItem>) -> Result<()> {
        for items in items {
            match items {
                syn::ImplItem::Const(c) => {
                    let attr = PhpConstAttribute::from_attributes(&c.attrs)?;
                    let name = attr
                        .rename
                        .rename(c.ident.to_string(), self.change_constant_case);
                    let docs = get_docs(&attr.attrs)?;
                    c.attrs.retain(|attr| !attr.path().is_ident("php"));

                    self.constants.push(Constant {
                        name,
                        ident: &c.ident,
                        docs,
                    });
                }
                syn::ImplItem::Fn(method) => {
                    let attr = PhpFunctionImplAttribute::from_attributes(&method.attrs)?;
                    let name = attr
                        .rename
                        .rename_method(method.sig.ident.to_string(), self.change_method_case);
                    let docs = get_docs(&attr.attrs)?;
                    method.attrs.retain(|attr| !attr.path().is_ident("php"));

                    let opts = MethodArgs::new(name, attr);
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
                        if matches!(opts.ty, MethodTy::Abstract) {
                            modifiers.insert(MethodModifier::Abstract);
                        }

                        let builder = func.function_builder(&call_type);

                        self.functions.push(FnBuilder {
                            builder,
                            vis: opts.vis,
                            modifiers,
                        });
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Generates an `impl PhpClassImpl<Self> for PhpClassImplCollector<Self>`
    /// block.
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
            quote! {
                (#name, &#path::#ident, &[#(#docs),*])
            }
        });

        quote! {
            impl ::ext_php_rs::internal::class::PhpClassImpl<#path>
                for ::ext_php_rs::internal::class::PhpClassImplCollector<#path>
            {
                fn get_methods(self) -> ::std::vec::Vec<
                    (::ext_php_rs::builders::FunctionBuilder<'static>, ::ext_php_rs::flags::MethodFlags)
                > {
                    vec![#(#functions),*]
                }

                fn get_method_props<'a>(self) -> ::std::collections::HashMap<&'static str, ::ext_php_rs::props::Property<'a, #path>> {
                    todo!()
                }

                fn get_constructor(self) -> ::std::option::Option<::ext_php_rs::class::ConstructorMeta<#path>> {
                    #constructor
                }

                fn get_constants(self) -> &'static [(&'static str, &'static dyn ::ext_php_rs::convert::IntoZvalDyn, &'static [&'static str])] {
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
