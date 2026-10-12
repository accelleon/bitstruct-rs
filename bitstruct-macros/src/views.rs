//! Generates the borrowed view types for a schema.
//!
//! `bitstruct! { #[bitstruct(..)] struct Header { .. } }` expands to:
//!
//! - `Header<'a>`: read-only view over `&'a [u8]`, with getters.
//! - `HeaderMut<'a>`: read-write view over `&'a mut [u8]`, with getters and setters.
//!
//! Both hold only a reference to the caller's bytes; nothing is copied.
//! Padding fields named `_` get no accessors.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ItemStruct;
use syn::parse::{ParseStream, Parser};

use crate::layout::{self, Field, FieldKind, Layout, mask, native_type};
use crate::options::{self, Options};
use crate::{access, runtime_crate};

/// Expands every struct in the macro input, reporting all their errors together.
pub fn expand_all(input: TokenStream) -> syn::Result<TokenStream> {
    let items = (|input: ParseStream| {
        let mut items = Vec::new();
        while !input.is_empty() {
            items.push(input.parse::<ItemStruct>()?);
        }
        Ok(items)
    })
    .parse2(input)?;

    let mut output = TokenStream::new();
    let mut errors: Option<syn::Error> = None;
    for item in items {
        match expand(item) {
            Ok(tokens) => output.extend(tokens),
            Err(error) => match &mut errors {
                Some(errors) => errors.combine(error),
                None => errors = Some(error),
            },
        }
    }
    errors.map_or(Ok(output), Err)
}

fn expand(mut item: ItemStruct) -> syn::Result<TokenStream> {
    let options = options::take(&mut item)?;
    let rt = runtime_crate()?;
    let layout = layout::parse(&item)?;
    check_name_collisions(&layout)?;

    let name = &item.ident;
    let name_mut = format_ident!("{}Mut", name);
    let vis = &item.vis;
    let docs = &item.attrs;
    let byte_len = layout.total_bits / 8;
    let bit_len = layout.total_bits;
    let order = options.order.name();
    let endian = options.endian.name();
    let getters: Vec<_> = layout
        .fields
        .iter()
        .filter(|field| !field.padding)
        .map(|field| getter(field, &options, byte_len, &rt))
        .collect();
    let setters: Vec<_> = layout
        .fields
        .iter()
        .filter(|field| !field.padding)
        .map(|field| setter(field, &options, byte_len, &rt))
        .collect();

    let constants = quote! {
        /// Size of the schema in bytes.
        pub const BYTE_LEN: usize = #byte_len;
        /// Size of the schema in bits.
        pub const BIT_LEN: usize = #bit_len;
        /// Bit order of packed fields: `"msb0"` or `"lsb0"`.
        pub const ORDER: &'static str = #order;
        /// Byte order of words: `"big"`, `"little"` or `"native"`.
        pub const ENDIAN: &'static str = #endian;
    };

    // A fixed-size array reference lets the compiler drop per-access bounds checks.
    Ok(quote! {
        #(#docs)*
        #[derive(Clone, Copy)]
        #vis struct #name<'a> { inner: &'a [u8; #byte_len] }

        #(#docs)*
        #vis struct #name_mut<'a> { inner: &'a mut [u8; #byte_len] }

        impl<'a> #name<'a> {
            #constants

            /// Borrows the first `BYTE_LEN` bytes of `bytes`.
            #[inline]
            pub fn from_bytes(bytes: &'a [u8]) -> ::core::result::Result<Self, #rt::Error> {
                let actual = bytes.len();
                let inner = bytes.first_chunk::<#byte_len>().ok_or(#rt::Error::BufferTooShort {
                    required: #byte_len, actual,
                })?;
                Ok(Self { inner })
            }

            /// The `BYTE_LEN` bytes this view reads.
            #[inline]
            pub fn as_bytes(&self) -> &'a [u8] { self.inner }

            #(#getters)*
        }

        impl<'a> #name_mut<'a> {
            #constants

            /// Borrows the first `BYTE_LEN` bytes of `bytes` for in-place edits.
            #[inline]
            pub fn from_bytes(bytes: &'a mut [u8]) -> ::core::result::Result<Self, #rt::Error> {
                let actual = bytes.len();
                let inner = bytes.first_chunk_mut::<#byte_len>().ok_or(#rt::Error::BufferTooShort {
                    required: #byte_len, actual,
                })?;
                Ok(Self { inner })
            }

            /// The `BYTE_LEN` bytes this view reads and writes.
            #[inline]
            pub fn as_bytes(&self) -> &[u8] { self.inner }

            /// Mutable access to the `BYTE_LEN` bytes this view reads and writes.
            #[inline]
            pub fn as_bytes_mut(&mut self) -> &mut [u8] { self.inner }

            /// Temporary read-only view of the same bytes.
            #[inline]
            pub fn as_view(&self) -> #name<'_> { #name { inner: self.inner } }

            /// Temporary read-write view of the same bytes.
            #[inline]
            pub fn reborrow(&mut self) -> #name_mut<'_> { #name_mut { inner: self.inner } }

            #(#getters)*
            #(#setters)*
        }
    })
}

/// `field()`, plus `field_raw()` for codec fields.
fn getter(field: &Field, options: &Options, byte_len: usize, rt: &TokenStream) -> TokenStream {
    let name = &field.name;
    let ty = &field.ty;
    let docs = docs_or(field, "Reads");
    let read = access::read(field, options, byte_len);

    if field.kind == FieldKind::Bool {
        return quote! {
            #docs
            #[inline]
            pub fn #name(&self) -> bool { (#read) != 0 }
        };
    }
    if field.kind == FieldKind::Codec {
        let raw = format_ident!("{}_raw", name);
        let raw_ty = native_type(field.bits);
        quote! {
            #docs
            #[inline]
            pub fn #name(&self) -> ::core::result::Result<#ty, #rt::Error> {
                <#ty as #rt::BitCodec>::from_bits(#read)
            }

            /// The field's bits without decoding.
            #[inline]
            pub fn #raw(&self) -> #raw_ty { (#read) as #raw_ty }
        }
    } else {
        quote! {
            #docs
            #[inline]
            pub fn #name(&self) -> #ty { (#read) as #ty }
        }
    }
}

/// `set_field(value)`: rejects values wider than the field before touching the buffer.
fn setter(field: &Field, options: &Options, byte_len: usize, rt: &TokenStream) -> TokenStream {
    let name = format_ident!("set_{}", field.name);
    let ty = &field.ty;
    let docs = docs_or(field, "Writes");
    let bits = field.bits as u32;
    let max = mask(field.bits);
    let write = access::write(field, options, byte_len);
    let to_bits = match field.kind {
        FieldKind::Codec => quote!(<#ty as #rt::BitCodec>::to_bits(&value)),
        FieldKind::Unsigned | FieldKind::Bool => quote!(value as u128),
    };

    quote! {
        #docs
        #[inline]
        pub fn #name(&mut self, value: #ty) -> ::core::result::Result<(), #rt::Error> {
            let value = #to_bits;
            if value > #max {
                return Err(#rt::Error::ValueOutOfRange { bits: #bits, value });
            }
            #write
            Ok(())
        }
    }
}

/// The field's own doc comments, or `"{verb} `name` (bits start..end)."` if it has none,
/// so generated accessors pass `missing_docs`.
fn docs_or(field: &Field, verb: &str) -> TokenStream {
    if !field.docs.is_empty() {
        let docs = &field.docs;
        return quote!(#(#docs)*);
    }
    let text = format!(
        " {verb} `{}` (bits {}..{}).",
        field.name,
        field.start,
        field.start + field.bits
    );
    quote!(#[doc = #text])
}

/// Rejects fields whose generated methods would clash, e.g. `a` and `set_a`.
fn check_name_collisions(layout: &Layout) -> syn::Result<()> {
    let mut taken: HashSet<String> = [
        "from_bytes",
        "as_bytes",
        "as_bytes_mut",
        "as_view",
        "reborrow",
        "BYTE_LEN",
        "BIT_LEN",
        "ORDER",
        "ENDIAN",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();

    for field in layout.fields.iter().filter(|field| !field.padding) {
        let mut names = vec![field.name.to_string(), format!("set_{}", field.name)];
        if field.kind == FieldKind::Codec {
            names.push(format!("{}_raw", field.name));
        }
        for name in names {
            if !taken.insert(name) {
                return Err(syn::Error::new(
                    field.name.span(),
                    "generated accessor name collision",
                ));
            }
        }
    }
    Ok(())
}
