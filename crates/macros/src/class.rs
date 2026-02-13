use darling::util::Flag;
use darling::{FromAttributes, FromMeta, ToTokens};
use proc_macro2::TokenStream;
use quote::{TokenStreamExt, quote};
use syn::{Attribute, Expr, Fields, GenericArgument, ItemStruct, PathArguments, Type};

use crate::helpers::get_docs;

/// Check if a type is `Option<T>` and return the inner type if so.
fn is_option_type(ty: &Type) -> Option<&Type> {
    let Type::Path(type_path) = ty else {
        return None;
    };
    if type_path.qself.is_some() {
        return None;
    }
    let segments = &type_path.path.segments;
    if segments.len() != 1 {
        return None;
    }
    let segment = &segments[0];
    if segment.ident != "Option" {
        return None;
    }
    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    if args.args.len() != 1 {
        return None;
    }
    if let GenericArgument::Type(inner) = &args.args[0] {
        return Some(inner);
    }
    None
}

/// Convert an expression to a PHP-compatible default string for stub generation.
fn expr_to_php_default_string(expr: &Expr) -> String {
    // For simple literals, we can convert them directly
    // For complex expressions, we use a string representation
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Str(s) => format!("'{}'", s.value().replace('\'', "\\'")),
            syn::Lit::Int(i) => i.to_string(),
            syn::Lit::Float(f) => f.to_string(),
            syn::Lit::Bool(b) => if b.value { "true" } else { "false" }.to_string(),
            _ => expr.to_token_stream().to_string(),
        },
        Expr::Array(_) => "[]".to_string(),
        Expr::Path(path) => {
            // Handle constants like `None`, `true`, `false`
            let path_str = path.to_token_stream().to_string();
            if path_str == "None" {
                "null".to_string()
            } else {
                path_str
            }
        }
        _ => expr.to_token_stream().to_string(),
    }
}
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
    /// Whether the class is readonly (PHP 8.2+).
    /// Readonly classes have all properties implicitly readonly.
    #[darling(rename = "readonly")]
    readonly: Flag,
    extends: Option<ClassEntryAttribute>,
    #[darling(multiple)]
    implements: Vec<ClassEntryAttribute>,
    attrs: Vec<Attribute>,
}

/// Represents a class entry reference, either explicit (with `ce` and `stub`)
/// or a simple type reference to a Rust type implementing `RegisteredClass`.
///
/// # Examples
///
/// Explicit form (for built-in PHP classes):
/// ```ignore
/// #[php(extends(ce = ce::exception, stub = "\\Exception"))]
/// ```
///
/// Simple type form (for Rust-defined classes):
/// ```ignore
/// #[php(extends(Base))]
/// ```
#[derive(Debug)]
pub enum ClassEntryAttribute {
    /// Explicit class entry with a function returning `&'static ClassEntry` and
    /// a stub name
    Explicit { ce: syn::Expr, stub: String },
    /// A Rust type that implements `RegisteredClass`
    Type(syn::Path),
}

impl FromMeta for ClassEntryAttribute {
    fn from_meta(item: &syn::Meta) -> darling::Result<Self> {
        match item {
            syn::Meta::List(list) => {
                // Try to parse as explicit form first: extends(ce = ..., stub = "...")
                // by checking if it contains '='
                let tokens_str = list.tokens.to_string();
                if tokens_str.contains('=') {
                    // Parse as explicit form with named parameters
                    #[derive(FromMeta)]
                    struct ExplicitForm {
                        ce: syn::Expr,
                        stub: String,
                    }
                    let explicit: ExplicitForm = FromMeta::from_meta(item)?;
                    Ok(ClassEntryAttribute::Explicit {
                        ce: explicit.ce,
                        stub: explicit.stub,
                    })
                } else {
                    // Parse as simple type form: extends(TypeName)
                    let path: syn::Path = list.parse_args().map_err(|e| {
                        darling::Error::custom(format!(
                            "Expected a type path (e.g., `MyClass`) or explicit form \
                             (e.g., `ce = expr, stub = \"Name\"`): {e}"
                        ))
                    })?;
                    Ok(ClassEntryAttribute::Type(path))
                }
            }
            _ => Err(darling::Error::unsupported_format("expected list format")),
        }
    }
}

impl ToTokens for ClassEntryAttribute {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let token = match self {
            ClassEntryAttribute::Explicit { ce, stub } => {
                // For explicit form, `ce` is expected to be a function like `ce::exception`
                quote! { (#ce, #stub) }
            }
            ClassEntryAttribute::Type(path) => {
                // For a Rust type, generate a closure that calls get_metadata().ce()
                // The closure can be coerced to a function pointer since it captures nothing
                quote! {
                    (
                        || <#path as ::ext_php_rs::class::RegisteredClass>::get_metadata().ce(),
                        <#path as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME
                    )
                }
            }
        };
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

    // Check if the struct derives Default - this is needed for exception classes
    // that extend \Exception to work correctly with zend_throw_exception_ex
    let has_derive_default = input.attrs.iter().any(|attr| {
        if attr.path().is_ident("derive")
            && let Ok(nested) = attr.parse_args_with(
                syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
            )
        {
            return nested.iter().any(|path| path.is_ident("Default"));
        }
        false
    });

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
        attr.readonly.is_present(),
        &docs,
        has_derive_default,
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
                ty: &field.ty,
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
    pub ty: &'a syn::Type,
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
    readonly: bool,
    docs: &[String],
    has_derive_default: bool,
) -> TokenStream {
    let modifier = modifier.option_tokens();

    // Separate instance properties from static properties
    let (instance_props, static_props): (Vec<_>, Vec<_>) =
        fields.iter().partition(|prop| !prop.is_static());

    // Generate instance properties (with Rust handlers)
    let instance_fields = instance_props.iter().map(|prop| {
        let name = &prop.name;
        let field_ident = prop.ident;
        let field_ty = prop.ty;
        let flags = prop
            .attr
            .flags
            .as_ref()
            .map(ToTokens::to_token_stream)
            .unwrap_or(quote! { ::ext_php_rs::flags::PropertyFlags::Public });
        let docs = &prop.docs;

        // Determine if the property is nullable (type is Option<T>)
        let nullable = is_option_type(field_ty).is_some();

        // Get the default value as a PHP-compatible string for stub generation
        let default_str = if let Some(default_expr) = &prop.attr.default {
            let s = expr_to_php_default_string(default_expr);
            quote! { ::std::option::Option::Some(#s) }
        } else {
            quote! { ::std::option::Option::None }
        };

        quote! {
            (#name, ::ext_php_rs::internal::property::PropertyInfo {
                prop: ::ext_php_rs::props::Property::field(|this: &mut Self| &mut this.#field_ident),
                flags: #flags,
                docs: &[#(#docs,)*],
                ty: ::std::option::Option::Some(<#field_ty as ::ext_php_rs::convert::IntoZval>::TYPE),
                nullable: #nullable,
                default: #default_str,
            })
        }
    });

    // Generate static properties (PHP-managed, no Rust handlers)
    // We combine the base flags with Static flag using from_bits_retain which is
    // const
    let static_fields = static_props.iter().map(|prop| {
        let name = &prop.name;
        let field_ty = prop.ty;
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

        // Determine if the property is nullable (type is Option<T>)
        let nullable = is_option_type(field_ty).is_some();

        // Get the default value as a PHP-compatible string for stub generation
        let default_str = if let Some(default_expr) = &prop.attr.default {
            let s = expr_to_php_default_string(default_expr);
            quote! { ::std::option::Option::Some(#s) }
        } else {
            quote! { ::std::option::Option::None }
        };

        // Use from_bits_retain to combine flags in a const context
        // Tuple: (name, flags, default_value, docs, type, nullable, default_str)
        quote! {
            (#name, ::ext_php_rs::flags::PropertyFlags::from_bits_retain(
                (#base_flags).bits() | ::ext_php_rs::flags::PropertyFlags::Static.bits()
            ), #default_value, &[#(#docs,)*] as &[&str], ::std::option::Option::Some(<#field_ty as ::ext_php_rs::convert::IntoZval>::TYPE), #nullable, #default_str)
        }
    });

    // Generate flags expression, combining user-provided flags with readonly if
    // specified. Note: ReadonlyClass is only available on PHP 8.2+, so we emit
    // a compile error if readonly is used on earlier PHP versions.
    // The compile_error! is placed as a statement so the block still has a valid
    // ClassFlags return type for type checking (even though compilation fails).
    let flags = match (flags, readonly) {
        (Some(flags), true) => {
            // User provided flags + readonly
            quote! {
                {
                    #[cfg(not(php82))]
                    compile_error!("The `readonly` class attribute requires PHP 8.2 or later");

                    #[cfg(php82)]
                    {
                        ::ext_php_rs::flags::ClassFlags::from_bits_retain(
                            (#flags).bits() | ::ext_php_rs::flags::ClassFlags::ReadonlyClass.bits()
                        )
                    }
                    #[cfg(not(php82))]
                    { #flags }
                }
            }
        }
        (Some(flags), false) => flags.to_token_stream(),
        (None, true) => {
            // Only readonly flag
            quote! {
                {
                    #[cfg(not(php82))]
                    compile_error!("The `readonly` class attribute requires PHP 8.2 or later");

                    #[cfg(php82)]
                    { ::ext_php_rs::flags::ClassFlags::ReadonlyClass }
                    #[cfg(not(php82))]
                    { ::ext_php_rs::flags::ClassFlags::empty() }
                }
            }
        }
        (None, false) => quote! { ::ext_php_rs::flags::ClassFlags::empty() },
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
        quote! { #imp }
    });

    let default_init_impl = generate_default_init_impl(ident, has_derive_default);

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
                use ::ext_php_rs::internal::class::PhpClassImpl;

                // Start with field properties (instance fields only, not static)
                let mut props = ::std::collections::HashMap::from_iter([
                    #(#instance_fields,)*
                ]);

                // Add method properties (from #[php(getter)] and #[php(setter)])
                let method_props = ::ext_php_rs::internal::class::PhpClassImplCollector::<Self>::default()
                    .get_method_props();
                for (name, prop_info) in method_props {
                    props.insert(name, prop_info);
                }

                props
            }

            #[must_use]
            #[allow(clippy::type_complexity)]
            fn static_properties() -> &'static [(&'static str, ::ext_php_rs::flags::PropertyFlags, ::std::option::Option<&'static (dyn ::ext_php_rs::convert::IntoZvalDyn + Sync)>, &'static [&'static str], ::std::option::Option<::ext_php_rs::flags::DataType>, bool, ::std::option::Option<&'static str>)] {
                static STATIC_PROPS: &[(&str, ::ext_php_rs::flags::PropertyFlags, ::std::option::Option<&'static (dyn ::ext_php_rs::convert::IntoZvalDyn + Sync)>, &[&str], ::std::option::Option<::ext_php_rs::flags::DataType>, bool, ::std::option::Option<&'static str>)] = &[#(#static_fields,)*];
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

            #[inline]
            fn interface_implementations() -> ::std::vec::Vec<::ext_php_rs::class::ClassEntryInfo> {
                let my_type_id = ::std::any::TypeId::of::<Self>();
                ::ext_php_rs::inventory::iter::<::ext_php_rs::internal::class::InterfaceRegistration>()
                    .filter(|reg| reg.class_type_id == my_type_id)
                    .map(|reg| (reg.interface_getter)())
                    .collect()
            }

            #[inline]
            fn interface_method_implementations() -> ::std::vec::Vec<(
                ::ext_php_rs::builders::FunctionBuilder<'static>,
                ::ext_php_rs::flags::MethodFlags,
            )> {
                use ::ext_php_rs::internal::class::InterfaceMethodsProvider;
                ::ext_php_rs::internal::class::PhpClassImplCollector::<Self>::default().get_interface_methods()
            }

            #default_init_impl
        }
    }
}

/// Generates the `default_init` method implementation for the trait.
fn generate_default_init_impl(ident: &syn::Ident, has_derive_default: bool) -> TokenStream {
    if has_derive_default {
        quote! {
            #[inline]
            #[must_use]
            fn default_init() -> ::std::option::Option<Self> {
                ::std::option::Option::Some(<#ident as ::std::default::Default>::default())
            }
        }
    } else {
        // Use the default implementation from the trait (returns None)
        quote! {}
    }
}
