//! Code generators behind the `bitstruct` crate.
//!
//! A procedural macro is a small program the compiler runs while building the
//! code that uses it. It receives the source inside `bitstruct! { ... }` and
//! returns replacement source. Nothing in this crate runs when packets are processed;
//! it only writes the getters and setters that do.
//!
//! Reading order:
//!
//! - `options.rs`: parses `#[bitstruct(order = ..., endian = ...)]` on each schema.
//! - `layout.rs`: gives every field a fixed bit position and validates the schema.
//! - `access.rs`: writes the shift-and-mask code that reads or writes one field.
//! - `views.rs`: assembles the generated `Name` and `NameMut` types.
//! - `codecs.rs`: `#[derive(BitEnum)]` and `#[derive(BitFlags)]`.
//!
//! Generated code is written with `quote! { ... }`, a template for Rust
//! source: `#name` inserts a value and `#(#items)*` repeats a list.

mod access;
mod codecs;
mod layout;
mod options;
mod views;

use proc_macro::TokenStream;

/// One or more structs of bit fields, each marked
/// `#[bitstruct(order = msb0, endian = big)]`.
///
/// A function-like macro rather than an attribute, because rustc rejects `_`
/// field names before an attribute macro could accept them as padding.
#[proc_macro]
pub fn bitstruct(input: TokenStream) -> TokenStream {
    report(views::expand_all(input.into()))
}

/// Field codec for a fieldless enum with an unsigned `#[repr(...)]`.
#[proc_macro_derive(BitEnum)]
pub fn bit_enum(item: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(item as syn::DeriveInput);
    report(codecs::bit_enum(&item))
}

/// Field codec for a `bitflags` type, using its `bits()` and `from_bits_retain()`.
#[proc_macro_derive(BitFlags)]
pub fn bit_flags(item: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(item as syn::DeriveInput);
    report(codecs::bit_flags(&item))
}

/// Turns a failure into a compiler error pointing at the offending source.
fn report(result: syn::Result<proc_macro2::TokenStream>) -> TokenStream {
    result.unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Path to the runtime crate, even if the user renamed the dependency.
fn runtime_crate() -> syn::Result<proc_macro2::TokenStream> {
    use proc_macro_crate::{FoundCrate, crate_name};
    match crate_name("bitstruct-rs") {
        Ok(FoundCrate::Itself) => Ok(quote::quote!(::bitstruct_rs)),
        Ok(FoundCrate::Name(name)) => {
            let name = quote::format_ident!("{}", name);
            Ok(quote::quote!(::#name))
        }
        Err(error) => Err(syn::Error::new(proc_macro2::Span::call_site(), error)),
    }
}
