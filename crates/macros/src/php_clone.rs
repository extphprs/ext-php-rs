//! Implementation for the `#[derive(PhpClone)]` macro.

use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

/// Parses the derive input and generates the trait implementations for
/// cloneable PHP classes.
pub fn parser(input: DeriveInput) -> TokenStream {
    let DeriveInput { ident, .. } = input;

    quote! {
        impl ::ext_php_rs::convert::FromZendObject<'_> for #ident {
            fn from_zend_object(
                obj: &::ext_php_rs::types::ZendObject,
            ) -> ::ext_php_rs::error::Result<Self> {
                let class_obj =
                    ::ext_php_rs::types::ZendClassObject::<#ident>::from_zend_obj(obj)
                        .ok_or(::ext_php_rs::error::Error::ZendClassObjectExtraction)?;
                ::ext_php_rs::error::Result::Ok((**class_obj).clone())
            }
        }

        impl ::ext_php_rs::convert::FromZval<'_> for #ident {
            const TYPE: ::ext_php_rs::flags::DataType = ::ext_php_rs::flags::DataType::Object(
                ::std::option::Option::Some(
                    <#ident as ::ext_php_rs::class::RegisteredClass>::CLASS_NAME,
                ),
            );

            fn from_zval(
                zval: &::ext_php_rs::types::Zval,
            ) -> ::std::option::Option<Self> {
                let obj = zval.object()?;
                <Self as ::ext_php_rs::convert::FromZendObject>::from_zend_object(obj).ok()
            }
        }
    }
}
