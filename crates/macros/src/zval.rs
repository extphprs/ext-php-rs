use darling::ToTokens;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    DataEnum, DataStruct, DeriveInput, GenericParam, Generics, Ident, ImplGenerics, Lifetime,
    LifetimeParam, TypeGenerics, Variant, WhereClause, punctuated::Punctuated, token::Where,
};

use crate::parsing::ident_to_php_name;
use crate::prelude::*;

pub fn parser(input: DeriveInput) -> Result<TokenStream> {
    let DeriveInput {
        generics, ident, ..
    } = input;

    let (into_impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let mut into_where_clause = where_clause.cloned().unwrap_or_else(|| WhereClause {
        where_token: Where {
            span: Span::call_site(),
        },
        predicates: Punctuated::default(),
    });
    let mut from_where_clause = into_where_clause.clone();

    // FIXME(david): work around since mutating `generics` will add the lifetime to
    // the struct generics as well, leading to an error as we would have
    // `impl<'a> FromZendObject<'a> for Struct<'a>` when `Struct` has no lifetime.
    let from_impl_generics = {
        let tokens = into_impl_generics.to_token_stream();
        let mut parsed: Generics = syn::parse2(tokens).expect("couldn't reparse generics");
        parsed
            .params
            .push(GenericParam::Lifetime(LifetimeParam::new(Lifetime::new(
                "'_zval",
                Span::call_site(),
            ))));
        parsed
    };

    for generic in &generics.params {
        match generic {
            GenericParam::Type(ty) => {
                let ident = &ty.ident;
                into_where_clause.predicates.push(
                    syn::parse2(quote! {
                        #ident: ::ext_php_rs::convert::IntoZval
                    })
                    .expect("couldn't parse where predicate"),
                );
                from_where_clause.predicates.push(
                    syn::parse2(quote! {
                        #ident: ::ext_php_rs::convert::FromZval<'_zval>
                    })
                    .expect("couldn't parse where predicate"),
                );
            }
            GenericParam::Lifetime(lt) => from_where_clause.predicates.push(
                syn::parse2(quote! {
                    '_zval: #lt
                })
                .expect("couldn't parse where predicate"),
            ),
            GenericParam::Const(_) => {}
        }
    }

    match input.data {
        syn::Data::Struct(data) => parse_struct(
            &data,
            &ident,
            &into_impl_generics,
            &from_impl_generics,
            &into_where_clause,
            &from_where_clause,
            &ty_generics,
        ),
        syn::Data::Enum(data) => parse_enum(
            &data,
            &ident,
            &into_impl_generics,
            &from_impl_generics,
            &into_where_clause,
            &from_where_clause,
            &ty_generics,
        ),
        syn::Data::Union(_) => {
            bail!(ident.span() => "Only structs and enums are supported by the `#[derive(ZvalConvert)]` macro.")
        }
    }
}

fn parse_struct(
    data: &DataStruct,
    ident: &Ident,
    into_impl_generics: &ImplGenerics,
    from_impl_generics: &Generics,
    into_where_clause: &WhereClause,
    from_where_clause: &WhereClause,
    ty_generics: &TypeGenerics,
) -> Result<TokenStream> {
    let into_fields = data
        .fields
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().ok_or_else(|| {
                err!(field => "Fields require names when using the `#[derive(ZvalConvert)]` macro on a struct.")
            })?;
            let field_name = ident_to_php_name(ident);

            Ok(quote! {
                obj.set_property(#field_name, self.#ident)?;
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let from_fields = data
        .fields
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().ok_or_else(|| {
                err!(field => "Fields require names when using the `#[derive(ZvalConvert)]` macro on a struct.")
            })?;
            let field_name = ident_to_php_name(ident);

            Ok(quote! {
                #ident: obj.get_property(#field_name)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(quote! {
        impl #into_impl_generics ::ext_php_rs::convert::IntoZendObject for #ident #ty_generics #into_where_clause {
            fn into_zend_object(self) -> ::ext_php_rs::error::Result<
                ::ext_php_rs::boxed::ZBox<
                    ::ext_php_rs::types::ZendObject
                >
            > {
                use ::ext_php_rs::convert::IntoZval;

                let mut obj = ::ext_php_rs::types::ZendObject::new_stdclass();
                #(#into_fields)*
                ::ext_php_rs::error::Result::Ok(obj)
            }
        }

        impl #into_impl_generics ::ext_php_rs::convert::IntoZval for #ident #ty_generics #into_where_clause {
            const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::Object(None);
            const NULLABLE: bool = false;

            fn set_zval(self, zv: &mut ::ext_php_rs::types::Zval, persistent: bool) -> ::ext_php_rs::error::Result<()> {
                use ::ext_php_rs::convert::{IntoZval, IntoZendObject};

                self.into_zend_object()?.set_zval(zv, persistent)
            }
        }

        impl #from_impl_generics ::ext_php_rs::convert::FromZendObject<'_zval> for #ident #ty_generics #from_where_clause {
            fn from_zend_object(obj: &'_zval ::ext_php_rs::types::ZendObject) -> ::ext_php_rs::error::Result<Self> {
                ::ext_php_rs::error::Result::Ok(Self {
                    #(#from_fields)*
                })
            }
        }

        impl #from_impl_generics ::ext_php_rs::convert::FromZval<'_zval> for #ident #ty_generics #from_where_clause {
            const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::Object(None);

            fn from_zval(zv: &'_zval ::ext_php_rs::types::Zval) -> ::std::option::Option<Self> {
                use ::ext_php_rs::convert::FromZendObject;

                Self::from_zend_object(zv.object()?).ok()
            }
        }
    })
}

fn parse_enum(
    data: &DataEnum,
    ident: &Ident,
    into_impl_generics: &ImplGenerics,
    from_impl_generics: &Generics,
    into_where_clause: &WhereClause,
    from_where_clause: &WhereClause,
    ty_generics: &TypeGenerics,
) -> Result<TokenStream> {
    let into_variants = data.variants.iter().filter_map(|variant| {
        // can have default fields - in this case, return `null`.
        if variant.fields.len() != 1 {
            return None;
        }

        let variant_ident = &variant.ident;
        Some(quote! {
            #ident::#variant_ident(val) => val.set_zval(zv, persistent)
        })
    });

    let mut default = None;
    let from_variants = data.variants.iter().map(|variant| {
        let Variant {
            ident,
            fields,
            ..
        } = variant;

        match fields {
            syn::Fields::Unnamed(fields) => {
                if fields.unnamed.len() != 1 {
                    bail!(fields => "Enum variant must only have one field when using `#[derive(ZvalConvert)]`.");
                }

                let ty = &fields.unnamed.first().unwrap().ty;

                Ok(Some(quote! {
                    if let Some(value) = <#ty>::from_zval(zval) {
                        return ::std::option::Option::Some(Self::#ident(value));
                    }
                }))
            },
            syn::Fields::Unit => {
                if default.is_some() {
                    bail!(fields => "Only one enum unit type is valid as a default when using `#[derive(ZvalConvert)]`.");
                }

                default.replace(quote! {
                    ::std::option::Option::Some(Self::#ident)
                });
                Ok(None)
            }
            syn::Fields::Named(_) => bail!(fields => "Enum variants must be unnamed and have only one field inside the variant when using `#[derive(ZvalConvert)]`.")
        }
    }).collect::<Result<Vec<_>>>()?;
    let default = default.unwrap_or_else(|| quote! { None });

    Ok(quote! {
        impl #into_impl_generics ::ext_php_rs::convert::IntoZval for #ident #ty_generics #into_where_clause {
            const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::Mixed;
            const NULLABLE: bool = false;

            fn set_zval(
                self,
                zv: &mut ::ext_php_rs::types::Zval,
                persistent: bool,
            ) -> ::ext_php_rs::error::Result<()> {
                use ::ext_php_rs::convert::IntoZval;

                match self {
                    #(#into_variants,)*
                    _ => {
                        zv.set_null();
                        ::ext_php_rs::error::Result::Ok(())
                    }
                }
            }
        }

        impl #from_impl_generics ::ext_php_rs::convert::FromZval<'_zval> for #ident #ty_generics #from_where_clause {
            const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::Mixed;

            fn from_zval(zval: &'_zval ::ext_php_rs::types::Zval) -> ::std::option::Option<Self> {
                #(#from_variants)*
                #default
            }
        }
    })
}
