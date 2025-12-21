use std::collections::HashMap;

use darling::{FromAttributes, ToTokens};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote};
use syn::spanned::Spanned as _;
use syn::{Expr, FnArg, GenericArgument, ItemFn, PatType, PathArguments, Type, TypePath};

use crate::helpers::get_docs;
use crate::parsing::{PhpRename, RenameRule, Visibility};
use crate::prelude::*;
use crate::syn_ext::DropLifetimes;

pub fn wrap(input: &syn::Path) -> Result<TokenStream> {
    let Some(func_name) = input.get_ident() else {
        bail!(input => "Pass a PHP function name into `wrap_function!()`.");
    };
    let builder_func = format_ident!("_internal_{func_name}");

    Ok(quote! {{
        (<#builder_func as ::ext_php_rs::internal::function::PhpFunction>::FUNCTION_ENTRY)()
    }})
}

#[derive(FromAttributes, Default, Debug)]
#[darling(default, attributes(php), forward_attrs(doc))]
struct PhpFunctionAttribute {
    #[darling(flatten)]
    rename: PhpRename,
    defaults: HashMap<Ident, Expr>,
    optional: Option<Ident>,
    vis: Option<Visibility>,
    attrs: Vec<syn::Attribute>,
}

pub fn parser(mut input: ItemFn) -> Result<TokenStream> {
    let php_attr = PhpFunctionAttribute::from_attributes(&input.attrs)?;
    input.attrs.retain(|attr| !attr.path().is_ident("php"));

    let args = Args::parse_from_fnargs(input.sig.inputs.iter(), php_attr.defaults)?;
    if let Some(ReceiverArg { span, .. }) = args.receiver {
        bail!(span => "Receiver arguments are invalid on PHP functions. See `#[php_impl]`.");
    }

    let docs = get_docs(&php_attr.attrs)?;

    let func = Function::new(
        &input.sig,
        php_attr
            .rename
            .rename(input.sig.ident.to_string(), RenameRule::Snake),
        args,
        php_attr.optional,
        docs,
    );
    let function_impl = func.php_function_impl();

    Ok(quote! {
        #input
        #function_impl
    })
}

#[derive(Debug)]
pub struct Function<'a> {
    /// Identifier of the Rust function associated with the function.
    pub ident: &'a Ident,
    /// Name of the function in PHP.
    pub name: String,
    /// Function arguments.
    pub args: Args<'a>,
    /// Function outputs.
    pub output: Option<&'a Type>,
    /// The first optional argument of the function.
    pub optional: Option<Ident>,
    /// Doc comments for the function.
    pub docs: Vec<String>,
}

#[derive(Debug)]
pub enum CallType<'a> {
    Function,
    Method {
        class: &'a syn::Path,
        receiver: MethodReceiver,
    },
}

/// Type of receiver on the method.
#[derive(Debug)]
pub enum MethodReceiver {
    /// Static method - has no receiver.
    Static,
    /// Class method, takes `&self` or `&mut self`.
    Class,
    /// Class method, takes `&mut ZendClassObject<Self>`.
    ZendClassObject,
}

impl<'a> Function<'a> {
    /// Parse a function.
    ///
    /// # Parameters
    ///
    /// * `sig` - Function signature.
    /// * `name` - Function name in PHP land.
    /// * `args` - Function arguments.
    /// * `optional` - The ident of the first optional argument.
    pub fn new(
        sig: &'a syn::Signature,
        name: String,
        args: Args<'a>,
        optional: Option<Ident>,
        docs: Vec<String>,
    ) -> Self {
        Self {
            ident: &sig.ident,
            name,
            args,
            output: match &sig.output {
                syn::ReturnType::Default => None,
                syn::ReturnType::Type(_, ty) => Some(&**ty),
            },
            optional,
            docs,
        }
    }

    /// Generates an internal identifier for the function.
    pub fn internal_ident(&self) -> Ident {
        format_ident!("_internal_{}", &self.ident)
    }

    pub fn abstract_function_builder(&self) -> TokenStream {
        let name = &self.name;
        let (required, not_required) = self.args.split_args(self.optional.as_ref());

        // `entry` impl
        let required_args = required
            .iter()
            .map(TypedArg::arg_builder)
            .collect::<Vec<_>>();
        let not_required_args = not_required
            .iter()
            .map(TypedArg::arg_builder)
            .collect::<Vec<_>>();

        let returns = self.build_returns();
        let docs = if self.docs.is_empty() {
            quote! {}
        } else {
            let docs = &self.docs;
            quote! {
                .docs(&[#(#docs),*])
            }
        };

        quote! {
            ::ext_php_rs::builders::FunctionBuilder::new_abstract(#name)
            #(.arg(#required_args))*
            .not_required()
            #(.arg(#not_required_args))*
            #returns
            #docs
        }
    }

    /// Generates the function builder for the function.
    pub fn function_builder(&self, call_type: CallType) -> TokenStream {
        let name = &self.name;
        let (required, not_required) = self.args.split_args(self.optional.as_ref());

        // `handler` impl
        let arg_declarations = self
            .args
            .typed
            .iter()
            .map(TypedArg::arg_declaration)
            .collect::<Vec<_>>();

        // `entry` impl
        let required_args = required
            .iter()
            .map(TypedArg::arg_builder)
            .collect::<Vec<_>>();
        let not_required_args = not_required
            .iter()
            .map(TypedArg::arg_builder)
            .collect::<Vec<_>>();

        let returns = self.build_returns();
        let result = self.build_result(call_type, required, not_required);
        let docs = if self.docs.is_empty() {
            quote! {}
        } else {
            let docs = &self.docs;
            quote! {
                .docs(&[#(#docs),*])
            }
        };

        quote! {
            ::ext_php_rs::builders::FunctionBuilder::new(#name, {
                ::ext_php_rs::zend_fastcall! {
                    extern fn handler(
                        ex: &mut ::ext_php_rs::zend::ExecuteData,
                        retval: &mut ::ext_php_rs::types::Zval,
                    ) {
                        use ::ext_php_rs::convert::IntoZval;

                        #(#arg_declarations)*
                        let result = {
                            #result
                        };

                        if let Err(e) = result.set_zval(retval, false) {
                            let e: ::ext_php_rs::exception::PhpException = e.into();
                            e.throw().expect("Failed to throw PHP exception.");
                        }
                    }
                }
                handler
            })
            #(.arg(#required_args))*
            .not_required()
            #(.arg(#not_required_args))*
            #returns
            #docs
        }
    }

    fn build_returns(&self) -> Option<TokenStream> {
        self.output.cloned().map(|mut output| {
            output.drop_lifetimes();
            quote! {
                .returns(
                    <#output as ::ext_php_rs::convert::IntoZval>::TYPE,
                    false,
                    <#output as ::ext_php_rs::convert::IntoZval>::NULLABLE,
                )
            }
        })
    }

    fn build_result(
        &self,
        call_type: CallType,
        required: &[TypedArg<'_>],
        not_required: &[TypedArg<'_>],
    ) -> TokenStream {
        let ident = self.ident;
        let required_arg_names: Vec<_> = required.iter().map(|arg| arg.name).collect();
        let not_required_arg_names: Vec<_> = not_required.iter().map(|arg| arg.name).collect();

        let variadic_bindings = self.args.typed.iter().filter_map(|arg| {
            if arg.variadic {
                let name = arg.name;
                let variadic_name = format_ident!("__variadic_{}", name);
                let clean_ty = arg.clean_ty();
                Some(quote! {
                    let #variadic_name = #name.variadic_vals::<#clean_ty>();
                })
            } else {
                None
            }
        });

        let arg_accessors = self.args.typed.iter().map(|arg| {
            arg.accessor(|e| {
                quote! {
                    #e.throw().expect("Failed to throw PHP exception.");
                    return;
                }
            })
        });

        match call_type {
            CallType::Function => quote! {
                let parse = ex.parser()
                    #(.arg(&mut #required_arg_names))*
                    .not_required()
                    #(.arg(&mut #not_required_arg_names))*
                    .parse();
                if parse.is_err() {
                    return;
                }
                #(#variadic_bindings)*

                #ident(#({#arg_accessors}),*)
            },
            CallType::Method { class, receiver } => {
                let this = match receiver {
                    MethodReceiver::Static => quote! {
                        let parse = ex.parser();
                    },
                    MethodReceiver::ZendClassObject | MethodReceiver::Class => quote! {
                        let (parse, this) = ex.parser_method::<#class>();
                        let this = match this {
                            Some(this) => this,
                            None => {
                                ::ext_php_rs::exception::PhpException::default("Failed to retrieve reference to `$this`".into())
                                    .throw()
                                    .unwrap();
                                return;
                            }
                        };
                    },
                };
                let call = match receiver {
                    MethodReceiver::Static => {
                        quote! { #class::#ident(#({#arg_accessors}),*) }
                    }
                    MethodReceiver::Class => quote! { this.#ident(#({#arg_accessors}),*) },
                    MethodReceiver::ZendClassObject => {
                        quote! { #class::#ident(this, #({#arg_accessors}),*) }
                    }
                };
                quote! {
                    #this
                    let parse_result = parse
                        #(.arg(&mut #required_arg_names))*
                        .not_required()
                        #(.arg(&mut #not_required_arg_names))*
                        .parse();
                    if parse_result.is_err() {
                        return;
                    }
                    #(#variadic_bindings)*

                    #call
                }
            }
        }
    }

    /// Generates a struct and impl for the `PhpFunction` trait.
    pub fn php_function_impl(&self) -> TokenStream {
        let internal_ident = self.internal_ident();
        let builder = self.function_builder(CallType::Function);

        quote! {
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            struct #internal_ident;

            impl ::ext_php_rs::internal::function::PhpFunction for #internal_ident {
                const FUNCTION_ENTRY: fn() -> ::ext_php_rs::builders::FunctionBuilder<'static> = {
                    fn entry() -> ::ext_php_rs::builders::FunctionBuilder<'static>
                    {
                        #builder
                    }
                    entry
                };
            }
        }
    }

    /// Returns a constructor metadata object for this function. This doesn't
    /// check if the function is a constructor, however.
    pub fn constructor_meta(
        &self,
        class: &syn::Path,
        visibility: Option<&Visibility>,
    ) -> TokenStream {
        let ident = self.ident;
        let (required, not_required) = self.args.split_args(self.optional.as_ref());
        let required_args = required
            .iter()
            .map(TypedArg::arg_builder)
            .collect::<Vec<_>>();
        let not_required_args = not_required
            .iter()
            .map(TypedArg::arg_builder)
            .collect::<Vec<_>>();

        let required_arg_names: Vec<_> = required.iter().map(|arg| arg.name).collect();
        let not_required_arg_names: Vec<_> = not_required.iter().map(|arg| arg.name).collect();
        let arg_declarations = self
            .args
            .typed
            .iter()
            .map(TypedArg::arg_declaration)
            .collect::<Vec<_>>();
        let variadic_bindings = self.args.typed.iter().filter_map(|arg| {
            if arg.variadic {
                let name = arg.name;
                let variadic_name = format_ident!("__variadic_{}", name);
                let clean_ty = arg.clean_ty();
                Some(quote! {
                    let #variadic_name = #name.variadic_vals::<#clean_ty>();
                })
            } else {
                None
            }
        });
        let arg_accessors = self.args.typed.iter().map(|arg| {
            arg.accessor(
                |e| quote! { return ::ext_php_rs::class::ConstructorResult::Exception(#e); },
            )
        });
        let variadic = self.args.typed.iter().any(|arg| arg.variadic).then(|| {
            quote! {
                .variadic()
            }
        });
        let docs = &self.docs;
        let flags = visibility.option_tokens();

        quote! {
            ::ext_php_rs::class::ConstructorMeta {
                constructor: {
                    fn inner(ex: &mut ::ext_php_rs::zend::ExecuteData) -> ::ext_php_rs::class::ConstructorResult<#class> {
                        #(#arg_declarations)*
                        let parse = ex.parser()
                            #(.arg(&mut #required_arg_names))*
                            .not_required()
                            #(.arg(&mut #not_required_arg_names))*
                            #variadic
                            .parse();
                        if parse.is_err() {
                            return ::ext_php_rs::class::ConstructorResult::ArgError;
                        }
                        #(#variadic_bindings)*
                        #class::#ident(#({#arg_accessors}),*).into()
                    }
                    inner
                },
                build_fn: {
                    fn inner(func: ::ext_php_rs::builders::FunctionBuilder) -> ::ext_php_rs::builders::FunctionBuilder {
                        func
                            .docs(&[#(#docs),*])
                            #(.arg(#required_args))*
                            .not_required()
                            #(.arg(#not_required_args))*
                            #variadic
                    }
                    inner
                },
                flags: #flags
            }
        }
    }
}

#[derive(Debug)]
pub struct ReceiverArg {
    pub _mutable: bool,
    pub span: Span,
}

#[derive(Debug)]
pub struct TypedArg<'a> {
    pub name: &'a Ident,
    pub ty: Type,
    pub nullable: bool,
    pub default: Option<Expr>,
    pub as_ref: bool,
    pub variadic: bool,
}

#[derive(Debug)]
pub struct Args<'a> {
    pub receiver: Option<ReceiverArg>,
    pub typed: Vec<TypedArg<'a>>,
}

impl<'a> Args<'a> {
    pub fn parse_from_fnargs(
        args: impl Iterator<Item = &'a FnArg>,
        mut defaults: HashMap<Ident, Expr>,
    ) -> Result<Self> {
        let mut result = Self {
            receiver: None,
            typed: vec![],
        };
        for arg in args {
            match arg {
                FnArg::Receiver(receiver) => {
                    if receiver.reference.is_none() {
                        bail!(receiver => "PHP objects are heap-allocated and cannot be passed by value. Try using `&self` or `&mut self`.");
                    } else if result.receiver.is_some() {
                        bail!(receiver => "Too many receivers specified.")
                    }
                    result.receiver.replace(ReceiverArg {
                        _mutable: receiver.mutability.is_some(),
                        span: receiver.span(),
                    });
                }
                FnArg::Typed(PatType { pat, ty, .. }) => {
                    let syn::Pat::Ident(syn::PatIdent { ident, .. }) = &**pat else {
                        bail!(pat => "Unsupported argument.");
                    };

                    // If the variable is `&[&Zval]` treat it as the variadic argument.
                    let default = defaults.remove(ident);
                    let nullable = type_is_nullable(ty.as_ref(), default.is_some())?;
                    let (variadic, as_ref, ty) = Self::parse_typed(ty);
                    result.typed.push(TypedArg {
                        name: ident,
                        ty,
                        nullable,
                        default,
                        as_ref,
                        variadic,
                    });
                }
            }
        }
        Ok(result)
    }

    fn parse_typed(ty: &Type) -> (bool, bool, Type) {
        match ty {
            Type::Reference(ref_) => {
                let as_ref = ref_.mutability.is_some();
                match ref_.elem.as_ref() {
                    Type::Slice(slice) => (
                        // TODO: Allow specifying the variadic type.
                        slice.elem.to_token_stream().to_string() == "& Zval",
                        as_ref,
                        ty.clone(),
                    ),
                    _ => (false, as_ref, ty.clone()),
                }
            }
            Type::Path(TypePath { path, .. }) => {
                let mut as_ref = false;

                // For for types that are `Option<&mut T>` to turn them into
                // `Option<&T>`, marking the Arg as as "passed by reference".
                let ty = path
                    .segments
                    .last()
                    .filter(|seg| seg.ident == "Option")
                    .and_then(|seg| {
                        if let PathArguments::AngleBracketed(args) = &seg.arguments {
                            args.args
                                .iter()
                                .find(|arg| matches!(arg, GenericArgument::Type(_)))
                                .and_then(|ga| match ga {
                                    GenericArgument::Type(ty) => Some(match ty {
                                        Type::Reference(r) => {
                                            // Only mark as_ref for mutable references
                                            // (Option<&mut T>), not immutable ones (Option<&T>)
                                            as_ref = r.mutability.is_some();
                                            let mut new_ref = r.clone();
                                            new_ref.mutability = None;
                                            Type::Reference(new_ref)
                                        }
                                        _ => ty.clone(),
                                    }),
                                    _ => None,
                                })
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| ty.clone());
                (false, as_ref, ty.clone())
            }
            _ => (false, false, ty.clone()),
        }
    }

    /// Splits the typed arguments into two slices:
    ///
    /// 1. Required arguments.
    /// 2. Non-required arguments.
    ///
    /// # Parameters
    ///
    /// * `optional` - The first optional argument. If [`None`], the optional
    ///   arguments will be from the first nullable argument after the last
    ///   non-nullable argument to the end of the arguments.
    pub fn split_args(&self, optional: Option<&Ident>) -> (&[TypedArg<'a>], &[TypedArg<'a>]) {
        let mut mid = None;
        for (i, arg) in self.typed.iter().enumerate() {
            if let Some(optional) = optional {
                if optional == arg.name {
                    mid.replace(i);
                }
            } else if mid.is_none() && arg.nullable {
                mid.replace(i);
            } else if !arg.nullable {
                mid.take();
            }
        }
        match mid {
            Some(mid) => (&self.typed[..mid], &self.typed[mid..]),
            None => (&self.typed[..], &self.typed[0..0]),
        }
    }
}

impl TypedArg<'_> {
    /// Returns a 'clean type' with the lifetimes removed. This allows the type
    /// to be used outside of the original function context.
    fn clean_ty(&self) -> Type {
        let mut ty = self.ty.clone();
        ty.drop_lifetimes();

        // Variadic arguments are passed as &[&Zval], so we need to extract the
        // inner type.
        if self.variadic {
            let Type::Reference(reference) = &ty else {
                return ty;
            };

            if let Type::Slice(inner) = &*reference.elem {
                return *inner.elem.clone();
            }
        }

        ty
    }

    /// Returns a token stream containing an argument declaration, where the
    /// name of the variable holding the arg is the name of the argument.
    fn arg_declaration(&self) -> TokenStream {
        let name = self.name;
        let val = self.arg_builder();
        quote! {
            let mut #name = #val;
        }
    }

    /// Returns a token stream containing the `Arg` definition to be passed to
    /// `ext-php-rs`.
    fn arg_builder(&self) -> TokenStream {
        let name = self.name.to_string();
        let ty = self.clean_ty();
        let null = if self.nullable {
            Some(quote! { .allow_null() })
        } else {
            None
        };
        let default = self.default.as_ref().map(|val| {
            let val = val.to_token_stream().to_string();
            quote! {
                .default(#val)
            }
        });
        let as_ref = if self.as_ref {
            Some(quote! { .as_ref() })
        } else {
            None
        };
        let variadic = self.variadic.then(|| quote! { .is_variadic() });
        quote! {
            ::ext_php_rs::args::Arg::new(#name, <#ty as ::ext_php_rs::convert::FromZvalMut>::TYPE)
                #null
                #default
                #as_ref
                #variadic
        }
    }

    /// Get the accessor used to access the value of the argument.
    fn accessor(&self, bail_fn: impl Fn(TokenStream) -> TokenStream) -> TokenStream {
        let name = self.name;
        if let Some(default) = &self.default {
            quote! {
                #name.val().unwrap_or(#default.into())
            }
        } else if self.variadic {
            let variadic_name = format_ident!("__variadic_{}", name);
            quote! {
                #variadic_name.as_slice()
            }
        } else if self.nullable {
            // Originally I thought we could just use the below case for `null` options, as
            // `val()` will return `Option<Option<T>>`, however, this isn't the case when
            // the argument isn't given, as the underlying zval is null.
            quote! {
                #name.val()
            }
        } else {
            let bail = bail_fn(quote! {
                ::ext_php_rs::exception::PhpException::default(
                    concat!("Invalid value given for argument `", stringify!(#name), "`.").into()
                )
            });
            quote! {
                match #name.val() {
                    Some(val) => val,
                    None => {
                        #bail;
                    }
                }
            }
        }
    }
}

/// Returns true of the given type is nullable in PHP.
// TODO(david): Eventually move to compile-time constants for this (similar to
// FromZval::NULLABLE).
pub fn type_is_nullable(ty: &Type, has_default: bool) -> Result<bool> {
    Ok(match ty {
        syn::Type::Path(path) => {
            has_default
                || path
                    .path
                    .segments
                    .iter()
                    .next_back()
                    .is_some_and(|seg| seg.ident == "Option")
        }
        syn::Type::Reference(_) => false, /* Reference cannot be nullable unless */
        // wrapped in `Option` (in that case it'd be a Path).
        _ => bail!(ty => "Unsupported argument type."),
    })
}
