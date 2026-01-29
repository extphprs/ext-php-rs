use std::collections::HashMap;

use darling::{FromAttributes, ToTokens};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{Expr, FnArg, GenericArgument, ItemFn, PatType, PathArguments, Type, TypePath};

use crate::helpers::get_docs;
use crate::parsing::{
    PhpNameContext, PhpRename, RenameRule, Visibility, ident_to_php_name, validate_php_name,
};
use crate::prelude::*;
use crate::syn_ext::DropLifetimes;

/// Checks if the return type is a reference to Self (`&Self` or `&mut Self`).
/// This is used to detect methods that return `$this` in PHP.
fn returns_self_ref(output: Option<&Type>) -> bool {
    let Some(ty) = output else {
        return false;
    };
    if let Type::Reference(ref_) = ty
        && let Type::Path(path) = &*ref_.elem
        && path.path.segments.len() == 1
        && let Some(segment) = path.path.segments.last()
    {
        return segment.ident == "Self";
    }
    false
}

/// Checks if the return type is `Self` (not a reference).
/// This is used to detect methods that return a new instance of the same class.
fn returns_self(output: Option<&Type>) -> bool {
    let Some(ty) = output else {
        return false;
    };
    if let Type::Path(path) = ty
        && path.path.segments.len() == 1
        && let Some(segment) = path.path.segments.last()
    {
        return segment.ident == "Self";
    }
    false
}

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

    let func_name = php_attr
        .rename
        .rename(ident_to_php_name(&input.sig.ident), RenameRule::Snake);
    validate_php_name(&func_name, PhpNameContext::Function, input.sig.ident.span())?;
    let func = Function::new(&input.sig, func_name, args, php_attr.optional, docs);
    let function_impl = func.php_function_impl();

    // Strip #[php(...)] attributes from function parameters before emitting output
    // (must be done after function_impl is generated since func borrows from input)
    for arg in &mut input.sig.inputs {
        if let FnArg::Typed(pat_type) = arg {
            pat_type.attrs.retain(|attr| !attr.path().is_ident("php"));
        }
    }

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

        let returns = self.build_returns(None);
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
    pub fn function_builder(&self, call_type: &CallType) -> TokenStream {
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

        let returns = self.build_returns(Some(call_type));
        let result = self.build_result(call_type, required, not_required);
        let docs = if self.docs.is_empty() {
            quote! {}
        } else {
            let docs = &self.docs;
            quote! {
                .docs(&[#(#docs),*])
            }
        };

        // Static methods cannot return &Self or &mut Self
        if returns_self_ref(self.output)
            && let CallType::Method {
                receiver: MethodReceiver::Static,
                ..
            } = call_type
            && let Some(output) = self.output
        {
            return quote_spanned! { output.span() =>
                compile_error!(
                    "Static methods cannot return `&Self` or `&mut Self`. \
                     Only instance methods can use fluent interface pattern returning `$this`."
                )
            };
        }

        // Check if this method returns &Self or &mut Self
        // In that case, we need to return `this` (the ZendClassObject) directly
        let returns_this = returns_self_ref(self.output)
            && matches!(
                call_type,
                CallType::Method {
                    receiver: MethodReceiver::Class | MethodReceiver::ZendClassObject,
                    ..
                }
            );

        let handler_body = if returns_this {
            quote! {
                use ::ext_php_rs::convert::IntoZval;

                #(#arg_declarations)*
                #result

                // The method returns &Self or &mut Self, use `this` directly
                if let Err(e) = this.set_zval(retval, false) {
                    let e: ::ext_php_rs::exception::PhpException = e.into();
                    e.throw().expect("Failed to throw PHP exception.");
                }
            }
        } else {
            quote! {
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
        };

        quote! {
            ::ext_php_rs::builders::FunctionBuilder::new(#name, {
                ::ext_php_rs::zend_fastcall! {
                    extern fn handler(
                        ex: &mut ::ext_php_rs::zend::ExecuteData,
                        retval: &mut ::ext_php_rs::types::Zval,
                    ) {
                        use ::ext_php_rs::zend::try_catch;
                        use ::std::panic::AssertUnwindSafe;

                        // Wrap the handler body with try_catch to ensure Rust destructors
                        // are called if a bailout occurs (issue #537)
                        let catch_result = try_catch(AssertUnwindSafe(|| {
                            #handler_body
                        }));

                        // If there was a bailout, run BailoutGuard cleanups and re-trigger
                        if catch_result.is_err() {
                            ::ext_php_rs::zend::run_bailout_cleanups();
                            unsafe { ::ext_php_rs::zend::bailout(); }
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

    fn build_returns(&self, call_type: Option<&CallType>) -> TokenStream {
        let Some(output) = self.output.cloned() else {
            // PHP magic methods __destruct and __clone cannot have return types
            // (only applies to class methods, not standalone functions)
            if matches!(call_type, Some(CallType::Method { .. }))
                && (self.name == "__destruct" || self.name == "__clone")
            {
                return quote! {};
            }
            // No return type means void in PHP
            return quote! {
                .returns(::ext_php_rs::flags::DataType::Void, false, false)
            };
        };

        let mut output = output;
        output.drop_lifetimes();

        // If returning &Self or &mut Self from a method, use the class type
        // for return type information since we return `this` (ZendClassObject)
        if returns_self_ref(self.output)
            && let Some(CallType::Method { class, .. }) = call_type
        {
            return quote! {
                .returns(
                    <&mut ::ext_php_rs::types::ZendClassObject<#class> as ::ext_php_rs::convert::IntoZval>::TYPE,
                    false,
                    <&mut ::ext_php_rs::types::ZendClassObject<#class> as ::ext_php_rs::convert::IntoZval>::NULLABLE,
                )
            };
        }

        // If returning Self (new instance) from a method, replace Self with
        // the actual class type since Self won't resolve in generated code
        if returns_self(self.output)
            && let Some(CallType::Method { class, .. }) = call_type
        {
            return quote! {
                .returns(
                    <#class as ::ext_php_rs::convert::IntoZval>::TYPE,
                    false,
                    <#class as ::ext_php_rs::convert::IntoZval>::NULLABLE,
                )
            };
        }

        quote! {
            .returns(
                <#output as ::ext_php_rs::convert::IntoZval>::TYPE,
                false,
                <#output as ::ext_php_rs::convert::IntoZval>::NULLABLE,
            )
        }
    }

    fn build_result(
        &self,
        call_type: &CallType,
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

        // Check if this method returns &Self or &mut Self
        let returns_this = returns_self_ref(self.output);

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

                // When returning &Self or &mut Self, discard the return value
                // (we'll use `this` directly in the handler)
                let call = match (receiver, returns_this) {
                    (MethodReceiver::Static, _) => {
                        quote! { #class::#ident(#({#arg_accessors}),*) }
                    }
                    (MethodReceiver::Class, true) => {
                        quote! { let _ = this.#ident(#({#arg_accessors}),*); }
                    }
                    (MethodReceiver::Class, false) => {
                        quote! { this.#ident(#({#arg_accessors}),*) }
                    }
                    (MethodReceiver::ZendClassObject, true) => {
                        // Explicit scope helps with mutable borrow lifetime when
                        // the method returns `&mut Self`
                        quote! {
                            {
                                let _ = #class::#ident(this, #({#arg_accessors}),*);
                            }
                        }
                    }
                    (MethodReceiver::ZendClassObject, false) => {
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
        let builder = self.function_builder(&CallType::Function);

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
                        use ::ext_php_rs::zend::try_catch;
                        use ::std::panic::AssertUnwindSafe;

                        // Wrap the constructor body with try_catch to ensure Rust destructors
                        // are called if a bailout occurs (issue #537)
                        let catch_result = try_catch(AssertUnwindSafe(|| {
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
                        }));

                        // If there was a bailout, run BailoutGuard cleanups and re-trigger
                        match catch_result {
                            Ok(result) => result,
                            Err(_) => {
                                ::ext_php_rs::zend::run_bailout_cleanups();
                                unsafe { ::ext_php_rs::zend::bailout() }
                            }
                        }
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

/// Represents a single element in a DNF type - either a simple class or an
/// intersection group.
#[derive(Debug, Clone)]
pub enum TypeGroup {
    /// A single class/interface type: `ArrayAccess`
    Single(String),
    /// An intersection of class/interface types: `Countable&Traversable`
    Intersection(Vec<String>),
}

/// Represents a complex PHP type declaration parsed from `#[php(type =
/// "...")]`.
#[derive(Debug, Clone)]
pub enum PhpTypeDecl {
    /// Union of primitive types: int|string|null
    PrimitiveUnion(Vec<TokenStream>),
    /// Intersection of class/interface types: Countable&Traversable
    Intersection(Vec<String>),
    /// Union of class types: Foo|Bar
    ClassUnion(Vec<String>),
    /// DNF (Disjunctive Normal Form) type: `(A&B)|C|D` or `(A&B)|(C&D)`
    /// e.g., `(A&B)|C` becomes `vec![Intersection(["A", "B"]), Single("C")]`
    Dnf(Vec<TypeGroup>),
    /// A Rust enum type that implements `PhpUnion` trait.
    /// The union types are determined at runtime via `PhpUnion::union_types()`.
    UnionEnum,
}

#[derive(Debug)]
pub struct TypedArg<'a> {
    pub name: &'a Ident,
    pub ty: Type,
    pub nullable: bool,
    pub default: Option<Expr>,
    pub as_ref: bool,
    pub variadic: bool,
    /// PHP type declaration from `#[php(type = "...")]` or `#[php(union =
    /// "...")]`
    pub php_type: Option<PhpTypeDecl>,
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
                FnArg::Typed(PatType { pat, ty, attrs, .. }) => {
                    let syn::Pat::Ident(syn::PatIdent { ident, .. }) = &**pat else {
                        bail!(pat => "Unsupported argument.");
                    };

                    // Parse #[php(type = "...")] or #[php(union = "...")] attribute if present
                    let php_type = Self::parse_type_attr(attrs)?;

                    // If the variable is `&[&Zval]` treat it as the variadic argument.
                    let default = defaults.remove(ident);
                    let nullable = type_is_nullable(ty.as_ref())?;
                    let (variadic, as_ref, ty) = Self::parse_typed(ty);
                    result.typed.push(TypedArg {
                        name: ident,
                        ty,
                        nullable,
                        default,
                        as_ref,
                        variadic,
                        php_type,
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
    ///   arguments will be from the first optional argument (nullable or has
    ///   default) after the last required argument to the end of the arguments.
    pub fn split_args(&self, optional: Option<&Ident>) -> (&[TypedArg<'a>], &[TypedArg<'a>]) {
        let mut mid = None;
        for (i, arg) in self.typed.iter().enumerate() {
            // An argument is optional if it's nullable (Option<T>) or has a default value.
            let is_optional = arg.nullable || arg.default.is_some();
            if let Some(optional) = optional {
                if optional == arg.name {
                    mid.replace(i);
                }
            } else if mid.is_none() && is_optional {
                mid.replace(i);
            } else if !is_optional {
                mid.take();
            }
        }
        match mid {
            Some(mid) => (&self.typed[..mid], &self.typed[mid..]),
            None => (&self.typed[..], &self.typed[0..0]),
        }
    }

    /// Parses `#[php(types = "...")]`, `#[php(union = "...")]`, or
    /// `#[php(union_enum)]` attribute from parameter attributes.
    /// Returns the parsed PHP type declaration if found.
    ///
    /// Supports:
    /// - `#[php(types = "int|string")]` - union of primitives
    /// - `#[php(types = "Countable&Traversable")]` - intersection of classes
    /// - `#[php(types = "Foo|Bar")]` - union of classes
    /// - `#[php(union = "int|string")]` - backwards compatible union syntax
    /// - `#[php(union_enum)]` - use `PhpUnion::union_types()` for Rust enum
    ///   types
    fn parse_type_attr(attrs: &[syn::Attribute]) -> Result<Option<PhpTypeDecl>> {
        for attr in attrs {
            if !attr.path().is_ident("php") {
                continue;
            }

            // Parse #[php(types = "...")], #[php(union = "...")], or #[php(union_enum)]
            let nested = attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            )?;

            for meta in nested {
                // Check for #[php(union_enum)] - a path without value
                if let syn::Meta::Path(path) = &meta
                    && path.is_ident("union_enum")
                {
                    return Ok(Some(PhpTypeDecl::UnionEnum));
                }

                // Check for #[php(types = "...")] or #[php(union = "...")]
                if let syn::Meta::NameValue(nv) = meta
                    && (nv.path.is_ident("types") || nv.path.is_ident("union"))
                    && let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit_str),
                        ..
                    }) = &nv.value
                {
                    let type_str = lit_str.value();
                    return Ok(Some(parse_php_type_string(&type_str)?));
                }
            }
        }
        Ok(None)
    }
}

/// Converts a PHP type name string to a `DataType` token stream.
/// Returns `None` if the type name is not recognized.
fn php_type_name_to_data_type(type_name: &str) -> Option<TokenStream> {
    let trimmed = type_name.trim();

    // Emit deprecation warnings for aliases deprecated in PHP 8.5
    // See: https://php.watch/versions/8.5/boolean-double-integer-binary-casts-deprecated
    let deprecated_warning = match trimmed {
        "boolean" => Some(("boolean", "bool")),
        "integer" => Some(("integer", "int")),
        "double" => Some(("double", "float")),
        "binary" => Some(("binary", "string")),
        _ => None,
    };

    if let Some((old, new)) = deprecated_warning {
        // Emit a compile-time warning for deprecated type aliases.
        // This generates a #[deprecated] item that triggers a warning.
        let warning_fn = syn::Ident::new(
            &format!("__ext_php_rs_deprecated_{old}"),
            proc_macro2::Span::call_site(),
        );
        let msg = format!("The type alias '{old}' is deprecated in PHP 8.5+. Use '{new}' instead.");
        // We return the tokens that include a deprecated function call to trigger
        // warning
        let data_type = match trimmed {
            "boolean" => quote! { ::ext_php_rs::flags::DataType::Bool },
            "integer" => quote! { ::ext_php_rs::flags::DataType::Long },
            "double" => quote! { ::ext_php_rs::flags::DataType::Double },
            "binary" => quote! { ::ext_php_rs::flags::DataType::String },
            _ => unreachable!(),
        };
        return Some(quote! {{
            #[deprecated(note = #msg)]
            #[allow(non_snake_case)]
            const fn #warning_fn() {}
            #[cfg(php85)]
            { let _ = #warning_fn(); }
            #data_type
        }});
    }

    let tokens = match trimmed {
        "int" | "long" => quote! { ::ext_php_rs::flags::DataType::Long },
        "string" => quote! { ::ext_php_rs::flags::DataType::String },
        "bool" => quote! { ::ext_php_rs::flags::DataType::Bool },
        "float" => quote! { ::ext_php_rs::flags::DataType::Double },
        "array" => quote! { ::ext_php_rs::flags::DataType::Array },
        "null" => quote! { ::ext_php_rs::flags::DataType::Null },
        "object" => quote! { ::ext_php_rs::flags::DataType::Object(None) },
        "resource" => quote! { ::ext_php_rs::flags::DataType::Resource },
        "callable" => quote! { ::ext_php_rs::flags::DataType::Callable },
        "iterable" => quote! { ::ext_php_rs::flags::DataType::Iterable },
        "mixed" => quote! { ::ext_php_rs::flags::DataType::Mixed },
        "void" => quote! { ::ext_php_rs::flags::DataType::Void },
        "false" => quote! { ::ext_php_rs::flags::DataType::False },
        "true" => quote! { ::ext_php_rs::flags::DataType::True },
        "never" => quote! { ::ext_php_rs::flags::DataType::Never },
        _ => return None,
    };
    Some(tokens)
}

/// Parses a PHP type string and determines if it's a union, intersection, DNF,
/// or class union.
///
/// Supports:
/// - `"int|string"` - union of primitives
/// - `"Countable&Traversable"` - intersection of classes/interfaces
/// - `"Foo|Bar"` - union of classes (when types start with uppercase)
/// - `"(A&B)|C"` - DNF (Disjunctive Normal Form) type (PHP 8.2+)
fn parse_php_type_string(type_str: &str) -> Result<PhpTypeDecl> {
    let type_str = type_str.trim();

    // Check if it's a DNF type (contains parentheses with intersection)
    if type_str.contains('(') && type_str.contains('&') {
        return parse_dnf_type(type_str);
    }

    // Check if it's an intersection type (contains & but no |)
    if type_str.contains('&') {
        if type_str.contains('|') {
            // Has both & and | but no parentheses - invalid syntax
            return Err(syn::Error::new(
                Span::call_site(),
                "DNF types require parentheses around intersection groups. Use '(A&B)|C' instead of 'A&B|C'.",
            ));
        }

        let class_names: Vec<String> = type_str.split('&').map(|s| s.trim().to_string()).collect();

        if class_names.len() < 2 {
            return Err(syn::Error::new(
                Span::call_site(),
                "Intersection type must contain at least 2 types",
            ));
        }

        // Validate that all intersection members look like class names (start with
        // uppercase)
        for name in &class_names {
            if name.is_empty() {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "Empty type name in intersection",
                ));
            }
            if !name.chars().next().unwrap().is_uppercase() && name != "self" {
                return Err(syn::Error::new(
                    Span::call_site(),
                    format!(
                        "Intersection types can only contain class/interface names. '{name}' looks like a primitive type.",
                    ),
                ));
            }
        }

        return Ok(PhpTypeDecl::Intersection(class_names));
    }

    // It's a union type (contains |)
    let parts: Vec<&str> = type_str.split('|').map(str::trim).collect();

    if parts.len() < 2 {
        return Err(syn::Error::new(
            Span::call_site(),
            "Type declaration must contain at least 2 types (e.g., 'int|string' or 'Foo&Bar')",
        ));
    }

    // Check if all parts are primitive types
    let primitive_types: Vec<Option<TokenStream>> = parts
        .iter()
        .map(|p| php_type_name_to_data_type(p))
        .collect();

    if primitive_types.iter().all(Option::is_some) {
        // All are primitives - it's a primitive union
        let tokens: Vec<TokenStream> = primitive_types.into_iter().map(Option::unwrap).collect();
        return Ok(PhpTypeDecl::PrimitiveUnion(tokens));
    }

    // Check if all parts look like class names (start with uppercase or are 'null')
    let all_classes = parts.iter().all(|p| {
        let p = p.trim();
        p == "null" || p.chars().next().is_some_and(char::is_uppercase) || p == "self"
    });

    if all_classes {
        // Filter out 'null' from class names - it's handled via allow_null
        let class_names: Vec<String> = parts
            .iter()
            .filter(|&&p| p != "null")
            .map(|&p| p.to_string())
            .collect();

        if class_names.is_empty() {
            return Err(syn::Error::new(
                Span::call_site(),
                "Class union must contain at least one class name",
            ));
        }

        return Ok(PhpTypeDecl::ClassUnion(class_names));
    }

    // Mixed primitives and classes in union - treat unknown ones as class names
    // Actually, for simplicity, if we have a mix, report an error
    Err(syn::Error::new(
        Span::call_site(),
        format!(
            "Cannot mix primitive types and class names in a union. \
             For primitive unions use: 'int|string|null'. \
             For class unions use: 'Foo|Bar'. Got: '{type_str}'",
        ),
    ))
}

/// Parses a DNF (Disjunctive Normal Form) type string like "(A&B)|C" or
/// "(A&B)|(C&D)".
///
/// Returns a `PhpTypeDecl::Dnf` with explicit `TypeGroup` variants:
/// - `TypeGroup::Single` for simple class names
/// - `TypeGroup::Intersection` for intersection groups
fn parse_dnf_type(type_str: &str) -> Result<PhpTypeDecl> {
    let mut groups: Vec<TypeGroup> = Vec::new();
    let mut current_pos = 0;
    let chars: Vec<char> = type_str.chars().collect();

    while current_pos < chars.len() {
        // Skip whitespace
        while current_pos < chars.len() && chars[current_pos].is_whitespace() {
            current_pos += 1;
        }

        if current_pos >= chars.len() {
            break;
        }

        // Skip | separator
        if chars[current_pos] == '|' {
            current_pos += 1;
            continue;
        }

        if chars[current_pos] == '(' {
            // Parse intersection group: (A&B&C)
            current_pos += 1; // Skip '('
            let start = current_pos;

            // Find closing parenthesis
            while current_pos < chars.len() && chars[current_pos] != ')' {
                current_pos += 1;
            }

            if current_pos >= chars.len() {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "Unclosed parenthesis in DNF type",
                ));
            }

            let group_str: String = chars[start..current_pos].iter().collect();
            current_pos += 1; // Skip ')'

            // Parse the intersection group
            let class_names: Vec<String> = group_str
                .split('&')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            if class_names.len() < 2 {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "Intersection group in DNF type must contain at least 2 types",
                ));
            }

            // Validate class names
            for name in &class_names {
                if !name.chars().next().unwrap().is_uppercase() && name != "self" {
                    return Err(syn::Error::new(
                        Span::call_site(),
                        format!(
                            "Intersection types can only contain class/interface names. '{name}' looks like a primitive type.",
                        ),
                    ));
                }
            }

            groups.push(TypeGroup::Intersection(class_names));
        } else {
            // Parse simple type name (until | or end)
            let start = current_pos;
            while current_pos < chars.len()
                && chars[current_pos] != '|'
                && !chars[current_pos].is_whitespace()
            {
                current_pos += 1;
            }

            let type_name: String = chars[start..current_pos].iter().collect();
            let type_name = type_name.trim();

            if !type_name.is_empty() {
                // Validate it's a class name
                if !type_name.chars().next().unwrap().is_uppercase()
                    && type_name != "self"
                    && type_name != "null"
                {
                    return Err(syn::Error::new(
                        Span::call_site(),
                        format!(
                            "DNF types can only contain class/interface names. '{type_name}' looks like a primitive type.",
                        ),
                    ));
                }

                groups.push(TypeGroup::Single(type_name.to_string()));
            }
        }
    }

    if groups.len() < 2 {
        return Err(syn::Error::new(
            Span::call_site(),
            "DNF type must contain at least 2 type groups",
        ));
    }

    Ok(PhpTypeDecl::Dnf(groups))
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
    #[allow(clippy::too_many_lines)]
    fn arg_builder(&self) -> TokenStream {
        let name = ident_to_php_name(self.name);
        let ty = self.clean_ty();
        let null = if self.nullable {
            Some(quote! { .allow_null() })
        } else {
            None
        };
        let default = self.default.as_ref().map(|val| {
            let val = expr_to_php_stub(val);
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

        // Check if we have a PHP type declaration override
        if let Some(php_type) = &self.php_type {
            return match php_type {
                PhpTypeDecl::PrimitiveUnion(data_types) => {
                    let data_types = data_types.clone();
                    quote! {
                        ::ext_php_rs::args::Arg::new_union(#name, vec![#(#data_types),*])
                            #default
                            #as_ref
                            #variadic
                    }
                }
                PhpTypeDecl::Intersection(class_names) => {
                    quote! {
                        ::ext_php_rs::args::Arg::new_intersection(
                            #name,
                            vec![#(#class_names.to_string()),*]
                        )
                            #default
                            #as_ref
                            #variadic
                    }
                }
                PhpTypeDecl::ClassUnion(class_names) => {
                    // Check if original type string included null for allow_null
                    quote! {
                        ::ext_php_rs::args::Arg::new_union_classes(
                            #name,
                            vec![#(#class_names.to_string()),*]
                        )
                            #null
                            #default
                            #as_ref
                            #variadic
                    }
                }
                PhpTypeDecl::Dnf(groups) => {
                    // Generate TypeGroup variants for DNF type
                    let group_tokens: Vec<_> = groups
                        .iter()
                        .map(|group| match group {
                            TypeGroup::Single(name) => {
                                quote! {
                                    ::ext_php_rs::args::TypeGroup::Single(#name.to_string())
                                }
                            }
                            TypeGroup::Intersection(names) => {
                                quote! {
                                    ::ext_php_rs::args::TypeGroup::Intersection(vec![#(#names.to_string()),*])
                                }
                            }
                        })
                        .collect();
                    quote! {
                        ::ext_php_rs::args::Arg::new_dnf(
                            #name,
                            vec![#(#group_tokens),*]
                        )
                            #null
                            #default
                            #as_ref
                            #variadic
                    }
                }
                PhpTypeDecl::UnionEnum => {
                    // Use PhpUnion::union_types() to get union types from the Rust enum.
                    // The result can be either Simple (Vec<DataType>) or Dnf (Vec<TypeGroup>).
                    quote! {
                        {
                            let union_types = <#ty as ::ext_php_rs::convert::PhpUnion>::union_types();
                            match union_types {
                                ::ext_php_rs::convert::PhpUnionTypes::Simple(types) => {
                                    ::ext_php_rs::args::Arg::new_union(#name, types)
                                        #null
                                        #default
                                        #as_ref
                                        #variadic
                                }
                                ::ext_php_rs::convert::PhpUnionTypes::Dnf(groups) => {
                                    ::ext_php_rs::args::Arg::new_dnf(#name, groups)
                                        #null
                                        #default
                                        #as_ref
                                        #variadic
                                }
                            }
                        }
                    }
                }
            };
        }

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
            if self.nullable {
                // For nullable types with defaults, null is acceptable
                quote! {
                    #name.val().unwrap_or(#default.into())
                }
            } else {
                // For non-nullable types with defaults:
                // - If argument was omitted: use default
                // - If null was explicitly passed: throw TypeError
                // - If a value was passed: try to convert it
                let bail_null = bail_fn(quote! {
                    ::ext_php_rs::exception::PhpException::new(
                        concat!("Argument `$", stringify!(#name), "` must not be null").into(),
                        0,
                        ::ext_php_rs::zend::ce::type_error(),
                    )
                });
                let bail_invalid = bail_fn(quote! {
                    ::ext_php_rs::exception::PhpException::default(
                        concat!("Invalid value given for argument `", stringify!(#name), "`.").into()
                    )
                });
                quote! {
                    match #name.zval() {
                        Some(zval) if zval.is_null() => {
                            // Null was explicitly passed to a non-nullable parameter
                            #bail_null
                        }
                        Some(_) => {
                            // A value was passed, try to convert it
                            match #name.val() {
                                Some(val) => val,
                                None => {
                                    #bail_invalid
                                }
                            }
                        }
                        None => {
                            // Argument was omitted, use default
                            #default.into()
                        }
                    }
                }
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

/// Converts a Rust expression to a PHP stub-compatible default value string.
///
/// This function handles common Rust patterns and converts them to valid PHP
/// syntax for use in generated stub files:
///
/// - `None` → `"null"`
/// - `Some(expr)` → converts the inner expression
/// - `42`, `3.14` → numeric literals as-is
/// - `true`/`false` → as-is
/// - `"string"` → `"string"`
/// - `"string".to_string()` or `String::from("string")` → `"string"`
fn expr_to_php_stub(expr: &Expr) -> String {
    match expr {
        // Handle None -> null
        Expr::Path(path) => {
            let path_str = path.path.to_token_stream().to_string();
            if path_str == "None" {
                "null".to_string()
            } else if path_str == "true" || path_str == "false" {
                path_str
            } else {
                // For other paths (constants, etc.), use the raw representation
                path_str
            }
        }

        // Handle Some(expr) -> convert inner expression
        Expr::Call(call) => {
            if let Expr::Path(func_path) = &*call.func {
                let func_name = func_path.path.to_token_stream().to_string();

                // Some(value) -> convert inner value
                if func_name == "Some"
                    && let Some(arg) = call.args.first()
                {
                    return expr_to_php_stub(arg);
                }

                // String::from("...") -> "..."
                if (func_name == "String :: from" || func_name == "String::from")
                    && let Some(arg) = call.args.first()
                {
                    return expr_to_php_stub(arg);
                }
            }

            // Default: use raw representation
            expr.to_token_stream().to_string()
        }

        // Handle method calls like "string".to_string()
        Expr::MethodCall(method_call) => {
            let method_name = method_call.method.to_string();

            // "...".to_string() or "...".to_owned() or "...".into() -> "..."
            if method_name == "to_string" || method_name == "to_owned" || method_name == "into" {
                return expr_to_php_stub(&method_call.receiver);
            }

            // Default: use raw representation
            expr.to_token_stream().to_string()
        }

        // String literals -> keep as-is (already valid PHP)
        Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Str(s) => format!(
                "\"{}\"",
                s.value().replace('\\', "\\\\").replace('"', "\\\"")
            ),
            syn::Lit::Int(i) => i.to_string(),
            syn::Lit::Float(f) => f.to_string(),
            syn::Lit::Bool(b) => if b.value { "true" } else { "false" }.to_string(),
            syn::Lit::Char(c) => format!("\"{}\"", c.value()),
            _ => expr.to_token_stream().to_string(),
        },

        // Handle arrays: [] or vec![]
        Expr::Array(arr) => {
            if arr.elems.is_empty() {
                "[]".to_string()
            } else {
                let elems: Vec<String> = arr.elems.iter().map(expr_to_php_stub).collect();
                format!("[{}]", elems.join(", "))
            }
        }

        // Handle vec![] macro
        Expr::Macro(m) => {
            let macro_name = m.mac.path.to_token_stream().to_string();
            if macro_name == "vec" {
                let tokens = m.mac.tokens.to_string();
                if tokens.trim().is_empty() {
                    return "[]".to_string();
                }
            }
            // Default: use raw representation
            expr.to_token_stream().to_string()
        }

        // Handle unary expressions like -42
        Expr::Unary(unary) => {
            let inner = expr_to_php_stub(&unary.expr);
            format!("{}{}", unary.op.to_token_stream(), inner)
        }

        // Default: use raw representation
        _ => expr.to_token_stream().to_string(),
    }
}

/// Returns true if the given type is nullable in PHP (i.e., it's an
/// `Option<T>`).
///
/// Note: Having a default value does NOT make a type nullable. A parameter with
/// a default value is optional (can be omitted), but passing `null` explicitly
/// should still be rejected unless the type is `Option<T>`.
// TODO(david): Eventually move to compile-time constants for this (similar to
// FromZval::NULLABLE).
pub fn type_is_nullable(ty: &Type) -> Result<bool> {
    Ok(match ty {
        Type::Path(path) => path
            .path
            .segments
            .iter()
            .next_back()
            .is_some_and(|seg| seg.ident == "Option"),
        Type::Reference(_) => false, /* Reference cannot be nullable unless */
        // wrapped in `Option` (in that case it'd be a Path).
        _ => bail!(ty => "Unsupported argument type."),
    })
}
