//! `#[derive(BitEnum)]` and `#[derive(BitFlags)]`.
//!
//! Both implement the runtime's `BitCodec` trait, which converts a typed value
//! to and from the raw bits stored in a field.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Fields, Type, spanned::Spanned};

use crate::layout::{WORD_SIZES, mask, unsigned_width};
use crate::runtime_crate;

/// For a `bitflags` type: keeps every bit, including ones without a named flag.
pub fn bit_flags(item: &DeriveInput) -> syn::Result<TokenStream> {
    let rt = runtime_crate()?;
    if !matches!(item.data, syn::Data::Struct(_)) {
        return Err(syn::Error::new(
            item.span(),
            "BitFlags requires a flags struct",
        ));
    }
    reject_generics(item, "generic flags are not supported")?;

    let name = &item.ident;
    Ok(quote! {
        impl #rt::BitCodec for #name {
            #[inline]
            fn from_bits(value: u128) -> ::core::result::Result<Self, #rt::Error> {
                let bits = ::core::convert::TryFrom::try_from(value)
                    .map_err(|_| #rt::Error::InvalidValue { value })?;
                Ok(Self::from_bits_retain(bits))
            }

            #[inline]
            fn to_bits(&self) -> u128 { u128::from(self.bits()) }
        }
    })
}

/// For a fieldless enum: maps each variant to its explicit discriminant.
/// Unknown values decode to `Error::InvalidValue`.
///
/// ```text
/// #[derive(BitEnum)]
/// #[repr(u8)]
/// enum Ecn { NotEct = 0, Ect1 = 1, Ect0 = 2, Congestion = 3 }
/// ```
pub fn bit_enum(item: &DeriveInput) -> syn::Result<TokenStream> {
    let rt = runtime_crate()?;
    reject_generics(item, "generic enums are not supported")?;
    let syn::Data::Enum(data) = &item.data else {
        return Err(syn::Error::new(item.span(), "BitEnum requires an enum"));
    };
    let repr_bits = repr_width(item)?;

    let mut seen = HashSet::new();
    let mut decode = Vec::new();
    let mut encode = Vec::new();
    for variant in &data.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return Err(syn::Error::new(
                variant.span(),
                "BitEnum variants must be fieldless",
            ));
        }
        let value = discriminant(variant)?;
        if value > mask(repr_bits) || !seen.insert(value) {
            return Err(syn::Error::new(
                variant.span(),
                "discriminant is out of range or duplicated",
            ));
        }
        let name = &variant.ident;
        decode.push(quote!(#value => Ok(Self::#name)));
        encode.push(quote!(Self::#name => #value));
    }

    let name = &item.ident;
    Ok(quote! {
        impl #rt::BitCodec for #name {
            #[inline]
            fn from_bits(value: u128) -> ::core::result::Result<Self, #rt::Error> {
                match value {
                    #(#decode,)*
                    _ => Err(#rt::Error::InvalidValue { value }),
                }
            }

            #[inline]
            fn to_bits(&self) -> u128 {
                match self { #(#encode,)* }
            }
        }
    })
}

/// Width of the enum's single unsigned `#[repr(uN)]`.
fn repr_width(item: &DeriveInput) -> syn::Result<usize> {
    let reprs: Vec<_> = item
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("repr"))
        .collect();
    let [repr] = reprs.as_slice() else {
        return Err(syn::Error::new(
            item.span(),
            "expected one unsigned #[repr(...)]",
        ));
    };
    let ty: Type = repr.parse_args()?;
    unsigned_width(&ty)
        .filter(|bits| WORD_SIZES.contains(bits))
        .ok_or_else(|| syn::Error::new(ty.span(), "expected unsigned integer repr"))
}

/// The variant's `= N`, which must be an integer literal.
fn discriminant(variant: &syn::Variant) -> syn::Result<u128> {
    match &variant.discriminant {
        Some((
            _,
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(value),
                ..
            }),
        )) => value.base10_parse(),
        _ => Err(syn::Error::new(
            variant.span(),
            "expected explicit unsigned integer literal discriminant",
        )),
    }
}

fn reject_generics(item: &DeriveInput, message: &str) -> syn::Result<()> {
    if item.generics.params.is_empty() && item.generics.where_clause.is_none() {
        Ok(())
    } else {
        Err(syn::Error::new(item.span(), message))
    }
}
