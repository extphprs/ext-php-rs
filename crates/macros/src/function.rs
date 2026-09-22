use std::collections::HashMap;

use darling::{FromAttributes, util::SpannedValue};
use proc_macro2::{Ident, Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned as _;
use syn::{Expr, FnArg, ItemFn, PatType, Type};

use crate::helpers::get_docs;
use crate::parsing::{
    PhpNameContext, PhpRename, RenameRule, Visibility, ident_to_php_name, reject_php_attrs,
    validate_php_name,
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
    vis: Option<SpannedValue<Visibility>>,
    attrs: Vec<syn::Attribute>,
}

pub fn parser(mut input: ItemFn) -> Result<TokenStream> {
    let php_attr = PhpFunctionAttribute::from_attributes(&input.attrs)?;
    if let Some(vis) = &php_attr.vis {
        bail!(vis.span() => "`vis` has no effect on a PHP function; visibility applies to methods and class constants.");
    }
    input.attrs.retain(|attr| !attr.path().is_ident("php"));

    let args = Args::parse_from_fnargs(input.sig.inputs.iter(), php_attr.defaults)?;
    if let Some(ReceiverArg { span }) = args.receiver {
        bail!(span => "Receiver arguments are invalid on PHP functions. See `#[php_impl]`.");
    }

    let docs = get_docs(&php_attr.attrs)?;

    let func_name = php_attr
        .rename
        .rename(ident_to_php_name(&input.sig.ident), RenameRule::Snake);
    validate_php_name(&func_name, PhpNameContext::Function, input.sig.ident.span())?;
    let func = Function::new(&input.sig, func_name, args, php_attr.optional, docs);
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
        let required = self.args.required_count(self.optional.as_ref());
        let args = self
            .args
            .typed
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

        quote! {{
            #required
            ::ext_php_rs::builders::FunctionBuilder::new_abstract(#name)
            #(.arg(#args))*
            .required_args(__REQUIRED)
            #returns
            #docs
        }}
    }

    /// Generates the function builder for the function.
    pub fn function_builder(&self, call_type: &CallType) -> TokenStream {
        let name = &self.name;
        let required = self.args.required_count(self.optional.as_ref());

        // `handler` impl
        let arg_declarations = self
            .args
            .typed
            .iter()
            .map(TypedArg::arg_declaration)
            .collect::<Vec<_>>();

        // `entry` impl
        let args = self
            .args
            .typed
            .iter()
            .map(TypedArg::arg_builder)
            .collect::<Vec<_>>();

        let returns = self.build_returns(Some(call_type));
        let result = self.build_result(call_type);
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

        let handler_body = if self.is_fast_path_eligible(call_type) {
            self.build_fast_handler_body(call_type)
        } else if returns_this {
            quote! {
                use ::ext_php_rs::convert::IntoZval;

                #(#arg_declarations)*
                #result

                // The method returns &Self or &mut Self, use `this` directly
                if let Err(e) = this.set_zval(retval, false) {
                    let e: ::ext_php_rs::exception::PhpException = e.into();
                    e.throw();
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
                    e.throw();
                }
            }
        };

        quote! {{
            #required
            ::ext_php_rs::builders::FunctionBuilder::new(#name, {
                ::ext_php_rs::zend_fastcall! {
                    #[allow(clippy::used_underscore_binding)]
                    extern fn handler(
                        ex: &mut ::ext_php_rs::zend::ExecuteData,
                        retval: &mut ::ext_php_rs::types::Zval,
                    ) {
                        ::ext_php_rs::zend::run_handler(::std::panic::AssertUnwindSafe(|| {
                            #handler_body
                        }));
                    }
                }
                handler
            })
            #(.arg(#args))*
            .required_args(__REQUIRED)
            #returns
            #docs
        }}
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

    fn build_result(&self, call_type: &CallType) -> TokenStream {
        let ident = self.ident;
        let arg_names: Vec<_> = self.args.typed.iter().map(|arg| arg.name).collect();

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
                    #e.throw();
                    return;
                }
            })
        });

        // Check if this method returns &Self or &mut Self
        let returns_this = returns_self_ref(self.output);

        match call_type {
            CallType::Function => quote! {
                let parse = ex.parser()
                    #(.arg(&mut #arg_names))*
                    .required_args(__REQUIRED)
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
                                ::ext_php_rs::exception::PhpException::from_message("Failed to retrieve reference to `$this`".into())
                                    .throw();
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
                        // Explicit scope helps with mutable borrow lifetime
                        // when the method returns `&mut
                        // Self`
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
                        #(.arg(&mut #arg_names))*
                        .required_args(__REQUIRED)
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

    /// Whether this function is eligible for the zero-alloc fast path.
    /// Requires: no variadic parameters.
    fn is_fast_path_eligible(&self, call_type: &CallType) -> bool {
        let no_variadic = !self.args.typed.iter().any(|arg| arg.variadic);
        let supported_call_type = matches!(
            call_type,
            CallType::Function
                | CallType::Method {
                    receiver: MethodReceiver::Static
                        | MethodReceiver::Class
                        | MethodReceiver::ZendClassObject,
                    ..
                }
        );
        no_variadic && supported_call_type
    }

    /// Generates a zero-alloc fast path handler body.
    ///
    /// Instead of building `ArgParser` with `Vec`/`String` heap allocations,
    /// reads zvals directly from the call frame via pointer arithmetic
    /// and converts with `FromZvalMut` inline. Matches the pattern used by
    /// PHP's `ZEND_PARSE_PARAMETERS_START`/`END` C macros.
    fn build_fast_arg_binding(i: usize, arg: &TypedArg<'_>) -> TokenStream {
        let name = arg.name;
        let ty = arg.clean_ty();
        let zval_ident = format_ident!("__zval_{}", i);

        let read_zval = quote! {
            let #zval_ident = unsafe { ex.zend_call_arg(#i) };
            let Some(#zval_ident) = #zval_ident else { return; };
        };

        let convert = quote! {
            <#ty as ::ext_php_rs::convert::FromZvalMut>::from_zval_mut(
                #zval_ident.dereference_mut()
            )
        };

        let throw_invalid = quote! {
            ::ext_php_rs::exception::PhpException::from_message(
                concat!("Invalid value given for argument `", stringify!(#name), "`.").into()
            ).throw();
            return;
        };

        // An explicit null on a non-nullable parameter that has a default is a
        // TypeError, the same as PHP reports for `int $x = 5` called with null.
        let reject_null = arg.default.as_ref().map(|_| {
            quote! {
                if !<#ty as ::ext_php_rs::convert::FromZvalMut>::NULLABLE && #zval_ident.is_null() {
                    ::ext_php_rs::exception::PhpException::new(
                        concat!("Argument `$", stringify!(#name), "` must not be null").into(),
                        0,
                        ::ext_php_rs::zend::ce::type_error(),
                    ).throw();
                    return;
                }
            }
        });

        let missing = arg.missing_value(&ty);

        quote! {
            let #name: #ty = {
                let __value = if #i < __REQUIRED || __num_args > #i {
                    #read_zval
                    #reject_null
                    #convert
                } else {
                    #missing
                };
                match __value {
                    Some(value) => value,
                    None => { #throw_invalid }
                }
            };
        }
    }

    fn build_fast_count_check(max_num_args: usize) -> TokenStream {
        let max_u32 = u32::try_from(max_num_args).expect("too many args");

        quote! {
            let __num_args = unsafe { ex.This.u2.num_args } as usize;
            if !(__REQUIRED..=#max_num_args).contains(&__num_args) {
                unsafe {
                    ::ext_php_rs::ffi::zend_wrong_parameters_count_error(
                        __REQUIRED.try_into().unwrap_or(u32::MAX),
                        #max_u32,
                    );
                };
                return;
            }
        }
    }

    fn build_fast_handler_body(&self, call_type: &CallType) -> TokenStream {
        let ident = self.ident;
        let max_num_args = self.args.typed.len();

        // Arg count validation (matches zend_wrong_parameters_count_error)
        let count_check = Self::build_fast_count_check(max_num_args);

        let arg_bindings: Vec<TokenStream> = self
            .args
            .typed
            .iter()
            .enumerate()
            .map(|(i, arg)| Self::build_fast_arg_binding(i, arg))
            .collect();

        let arg_names: Vec<_> = self.args.typed.iter().map(|arg| arg.name).collect();

        let this_error = quote! {
            ::ext_php_rs::exception::PhpException::from_message(
                "Failed to retrieve reference to `$this`".into()
            ).throw();
            return;
        };

        let returns_this = returns_self_ref(self.output);

        let (this_binding, call) = match call_type {
            CallType::Function => (quote! {}, quote! { #ident(#(#arg_names),*) }),
            CallType::Method {
                class,
                receiver: MethodReceiver::Static,
                ..
            } => (quote! {}, quote! { #class::#ident(#(#arg_names),*) }),
            CallType::Method {
                class,
                receiver: MethodReceiver::Class,
                ..
            } => (
                quote! {
                    let __this = match ex.get_object::<#class>() {
                        Some(v) => v,
                        None => { #this_error }
                    };
                },
                if returns_this {
                    quote! { let _ = __this.#ident(#(#arg_names),*); }
                } else {
                    quote! { __this.#ident(#(#arg_names),*) }
                },
            ),
            CallType::Method {
                class,
                receiver: MethodReceiver::ZendClassObject,
                ..
            } => (
                quote! {
                    let __this = match ex.get_object::<#class>() {
                        Some(v) => v,
                        None => { #this_error }
                    };
                },
                if returns_this {
                    quote! { { let _ = #class::#ident(__this, #(#arg_names),*); } }
                } else {
                    quote! { #class::#ident(__this, #(#arg_names),*) }
                },
            ),
        };

        if returns_this {
            quote! {
                use ::ext_php_rs::convert::{FromZvalMut, IntoZval};

                #count_check
                #(#arg_bindings)*
                #this_binding
                #call

                if let Err(e) = __this.set_zval(retval, false) {
                    let e: ::ext_php_rs::exception::PhpException = e.into();
                    e.throw();
                }
            }
        } else {
            quote! {
                use ::ext_php_rs::convert::{FromZvalMut, IntoZval};

                #count_check
                #(#arg_bindings)*
                #this_binding
                let __result = { #call };

                if let Err(e) = __result.set_zval(retval, false) {
                    let e: ::ext_php_rs::exception::PhpException = e.into();
                    e.throw();
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
        let required = self.args.required_count(self.optional.as_ref());
        let args = self
            .args
            .typed
            .iter()
            .map(TypedArg::arg_builder)
            .collect::<Vec<_>>();
        let arg_names: Vec<_> = self.args.typed.iter().map(|arg| arg.name).collect();
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
        let docs = &self.docs;
        let flags = visibility.option_tokens();

        quote! {{
            #required
            ::ext_php_rs::class::ConstructorMeta {
                constructor: {
                    fn inner(ex: &mut ::ext_php_rs::zend::ExecuteData) -> ::ext_php_rs::class::ConstructorResult<#class> {
                        #(#arg_declarations)*
                        let parse = ex.parser()
                            #(.arg(&mut #arg_names))*
                            .required_args(__REQUIRED)
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
                            #(.arg(#args))*
                            .required_args(__REQUIRED)
                    }
                    inner
                },
                flags: #flags
            }
        }}
    }
}

#[derive(Debug)]
pub struct ReceiverArg {
    pub span: Span,
}

#[derive(Debug)]
pub struct TypedArg<'a> {
    pub name: &'a Ident,
    pub ty: Type,
    pub default: Option<Expr>,
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
                    let syn::ReceiverKind::Reference(..) = &receiver.kind else {
                        bail!(receiver => "PHP objects are heap-allocated and cannot be passed by value. Try using `&self` or `&mut self`.");
                    };
                    if result.receiver.is_some() {
                        bail!(receiver => "Too many receivers specified.")
                    }
                    result.receiver.replace(ReceiverArg {
                        span: receiver.span(),
                    });
                }
                FnArg::Typed(PatType { attrs, pat, ty, .. }) => {
                    reject_php_attrs(
                        attrs,
                        "function parameters; use `defaults` and `optional` on the function",
                    )?;
                    let syn::Pat::Ident(syn::PatIdent { ident, .. }) = &**pat else {
                        bail!(pat => "Unsupported argument.");
                    };
                    if result.typed.last().is_some_and(|arg| arg.variadic) {
                        bail!(pat => "A variadic parameter (`&[T]`) must be the last parameter.");
                    }

                    result.typed.push(TypedArg {
                        name: ident,
                        ty: (**ty).clone(),
                        default: defaults.remove(ident),
                        variadic: is_slice_ref(ty),
                    });
                }
            }
        }
        reject_unknown_defaults(defaults)?;
        Ok(result)
    }

    /// Emits `const __REQUIRED: usize`, the number of leading required
    /// parameters.
    ///
    /// With `#[php(optional = x)]` it is the position of `x`. Otherwise it is
    /// computed by `ext_php_rs::args::required_count` from each parameter's
    /// `FromZvalMut::NULLABLE` and default, so the rule follows the type and
    /// not its spelling. A variadic parameter is counted by the runtime.
    pub fn required_count(&self, optional: Option<&Ident>) -> TokenStream {
        if let Some(optional) = optional
            && let Some(index) = self.typed.iter().position(|arg| arg.name == optional)
        {
            return quote! { const __REQUIRED: usize = #index; };
        }
        let omittable = self.typed.iter().map(|arg| {
            if arg.variadic {
                return quote! { false };
            }
            let ty = arg.clean_ty();
            let has_default = arg.default.is_some();
            quote! { <#ty as ::ext_php_rs::convert::FromZvalMut>::NULLABLE || #has_default }
        });
        quote! {
            const __REQUIRED: usize = ::ext_php_rs::args::required_count(&[#(#omittable),*]);
        }
    }
}

fn reject_unknown_defaults(defaults: HashMap<Ident, Expr>) -> Result<()> {
    let mut unknown: Vec<Ident> = defaults.into_keys().collect();
    unknown.sort();
    unknown
        .into_iter()
        .map(|name| err!(name => "no parameter named `{name}`; `defaults` keys must match a parameter name"))
        .reduce(|mut all, next| {
            all.combine(next);
            all
        })
        .map_or(Ok(()), Err)
}

/// A `&[T]` parameter is the variadic tail. The element type decides the
/// PHP type, nullability and pass-by-reference of each variadic value.
fn is_slice_ref(ty: &Type) -> bool {
    matches!(ty, Type::Reference(reference) if matches!(*reference.elem, Type::Slice(_)))
}

impl TypedArg<'_> {
    /// Returns a 'clean type' with the lifetimes removed. This allows the type
    /// to be used outside of the original function context.
    fn clean_ty(&self) -> Type {
        let mut ty = self.ty.clone();
        ty.drop_lifetimes();

        // Variadic arguments are passed as &[T], so we need to extract the
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
        let name = ident_to_php_name(self.name);
        let ty = self.clean_ty();
        let default = self.default.as_ref().map(|default| {
            quote! {
                .default({
                    let __default: #ty = (#default).into();
                    ::ext_php_rs::convert::StubLiteral::stub_literal(&__default)
                })
            }
        });
        let variadic = self.variadic.then(|| quote! { .is_variadic() });
        quote! {
            ::ext_php_rs::args::Arg::of::<#ty>(#name)
                #default
                #variadic
        }
    }

    /// The value of the argument when the caller omitted it: the default when
    /// one is declared, otherwise what the type says (`Some(None)` for
    /// `Option<T>`, `None` for a required type).
    fn missing_value(&self, ty: &Type) -> TokenStream {
        if let Some(default) = &self.default {
            quote! { ::std::option::Option::Some((#default).into()) }
        } else {
            quote! { <#ty as ::ext_php_rs::convert::FromZvalMut>::from_missing() }
        }
    }

    /// Get the accessor used to access the value of the argument.
    fn accessor(&self, bail_fn: impl Fn(TokenStream) -> TokenStream) -> TokenStream {
        let name = self.name;
        if self.variadic {
            let variadic_name = format_ident!("__variadic_{}", name);
            return quote! {
                #variadic_name.as_slice()
            };
        }

        let ty = self.clean_ty();
        let bail_invalid = bail_fn(quote! {
            ::ext_php_rs::exception::PhpException::from_message(
                concat!("Invalid value given for argument `", stringify!(#name), "`.").into()
            )
        });
        let reject_null = self.default.as_ref().map(|_| {
            let bail_null = bail_fn(quote! {
                ::ext_php_rs::exception::PhpException::new(
                    concat!("Argument `$", stringify!(#name), "` must not be null").into(),
                    0,
                    ::ext_php_rs::zend::ce::type_error(),
                )
            });
            quote! {
                Some(zval) if !<#ty as ::ext_php_rs::convert::FromZvalMut>::NULLABLE && zval.is_null() => {
                    #bail_null
                }
            }
        });
        let missing = self.missing_value(&ty);

        quote! {
            match match #name.zval() {
                #reject_null
                Some(zval) => <#ty as ::ext_php_rs::convert::FromZvalMut>::from_zval_mut(zval.dereference_mut()),
                None => #missing,
            } {
                Some(value) => value,
                None => {
                    #bail_invalid
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darling::ToTokens;

    fn parse_args(sig: &str) -> Args<'static> {
        let sig: &'static syn::Signature =
            Box::leak(Box::new(syn::parse_str::<syn::Signature>(sig).unwrap()));
        Args::parse_from_fnargs(sig.inputs.iter(), HashMap::new()).unwrap()
    }

    #[test]
    fn slice_reference_in_last_position_is_variadic_whatever_the_element() {
        let args = parse_args("fn f(a: i64, rest: &[&ext_php_rs::types::Zval])");
        assert!(!args.typed[0].variadic);
        assert!(args.typed[1].variadic);
        let args = parse_args("fn f(rest: &[i64])");
        assert!(args.typed[0].variadic);
        assert_eq!(
            args.typed[0].clean_ty().to_token_stream().to_string(),
            "i64"
        );
    }

    #[test]
    fn variadic_must_be_last() {
        let sig = syn::parse_str::<syn::Signature>("fn f(rest: &[i64], a: i64)").unwrap();
        let err = Args::parse_from_fnargs(sig.inputs.iter(), HashMap::new()).unwrap_err();
        assert!(err.to_string().contains("must be the last parameter"));
    }

    #[test]
    fn required_count_follows_the_optional_attribute_or_the_types() {
        let args = parse_args("fn f(a: MaybeAge, b: Option<i64>)");
        let by_type = args.required_count(None).to_string();
        assert!(by_type.contains("required_count"));
        assert!(
            by_type.contains("MaybeAge as :: ext_php_rs :: convert :: FromZvalMut > :: NULLABLE")
        );
        let explicit = args.required_count(Some(&format_ident!("b"))).to_string();
        assert_eq!(explicit, "const __REQUIRED : usize = 1usize ;");
    }

    #[test]
    fn variadic_slot_is_never_counted_as_omittable() {
        let args = parse_args("fn f(a: i64, rest: &[i64])");
        let tokens = args.required_count(None).to_string();
        assert!(tokens.ends_with("|| false , false]) ;"));
    }

    fn parse_with_defaults(sig: &str, defaults: &[&str]) -> Result<Args<'static>> {
        let sig: &'static syn::Signature =
            Box::leak(Box::new(syn::parse_str::<syn::Signature>(sig).unwrap()));
        let defaults = defaults
            .iter()
            .map(|name| (format_ident!("{name}"), syn::parse_quote!(0)))
            .collect();
        Args::parse_from_fnargs(sig.inputs.iter(), defaults)
    }

    #[test]
    fn defaults_are_taken_by_their_parameter() {
        let args = parse_with_defaults("fn f(a: i64, b: i64)", &["b"]).unwrap();
        assert!(args.typed[0].default.is_none());
        assert!(args.typed[1].default.is_some());
    }

    #[test]
    fn defaults_for_unknown_parameters_are_rejected_in_name_order() {
        let err = parse_with_defaults("fn f(a: i64)", &["zed", "a", "bee"]).unwrap_err();
        let messages: Vec<String> = err.into_iter().map(|e| e.to_string()).collect();
        assert_eq!(
            messages,
            [
                "no parameter named `bee`; `defaults` keys must match a parameter name",
                "no parameter named `zed`; `defaults` keys must match a parameter name",
            ]
        );
    }

    #[test]
    fn test_only_reference_receivers_are_accepted() {
        let by_ref: FnArg = syn::parse_quote!(&self);
        let by_mut: FnArg = syn::parse_quote!(&mut self);
        let by_value: FnArg = syn::parse_quote!(self);
        let boxed: FnArg = syn::parse_quote!(self: Box<Self>);

        assert!(Args::parse_from_fnargs([&by_ref].into_iter(), HashMap::new()).is_ok());
        assert!(Args::parse_from_fnargs([&by_mut].into_iter(), HashMap::new()).is_ok());
        assert!(Args::parse_from_fnargs([&by_value].into_iter(), HashMap::new()).is_err());
        assert!(Args::parse_from_fnargs([&boxed].into_iter(), HashMap::new()).is_err());
    }
}
