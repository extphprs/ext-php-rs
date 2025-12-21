use darling::FromAttributes;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ItemConst;

use crate::helpers::get_docs;
use crate::parsing::{PhpNameContext, PhpRename, RenameRule, ident_to_php_name, validate_php_name};
use crate::prelude::*;

const INTERNAL_CONST_DOC_PREFIX: &str = "_internal_const_docs_";
const INTERNAL_CONST_NAME_PREFIX: &str = "_internal_const_name_";

#[derive(FromAttributes, Default, Debug)]
#[darling(default, attributes(php), forward_attrs(doc))]
pub(crate) struct PhpConstAttribute {
    #[darling(flatten)]
    pub(crate) rename: PhpRename,
    // TODO: Implement const Visibility
    // pub(crate) vis: Option<Visibility>,
    pub(crate) attrs: Vec<syn::Attribute>,
}

pub fn parser(mut item: ItemConst) -> Result<TokenStream> {
    let attr = PhpConstAttribute::from_attributes(&item.attrs)?;

    let name = attr
        .rename
        .rename(ident_to_php_name(&item.ident), RenameRule::ScreamingSnake);
    validate_php_name(&name, PhpNameContext::Constant, item.ident.span())?;
    let name_ident = format_ident!("{INTERNAL_CONST_NAME_PREFIX}{}", item.ident);

    let docs = get_docs(&attr.attrs)?;
    let docs_ident = format_ident!("{INTERNAL_CONST_DOC_PREFIX}{}", item.ident);
    item.attrs.retain(|attr| !attr.path().is_ident("php"));

    Ok(quote! {
        #item
        #[allow(non_upper_case_globals)]
        const #docs_ident: &[&str] = &[#(#docs),*];
        #[allow(non_upper_case_globals)]
        const #name_ident: &str = #name;
    })
}

pub fn wrap(input: &syn::Path) -> Result<TokenStream> {
    let Some(const_name) = input.get_ident().map(ToString::to_string) else {
        bail!(input => "Pass a PHP const into `wrap_constant!()`.");
    };
    let doc_const = format_ident!("{INTERNAL_CONST_DOC_PREFIX}{const_name}");
    let const_name = format_ident!("{INTERNAL_CONST_NAME_PREFIX}{const_name}");

    Ok(quote! {
        (#const_name, #input, #doc_const)
    })
}
