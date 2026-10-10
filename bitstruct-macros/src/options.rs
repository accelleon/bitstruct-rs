//! Parses the arguments of `#[bitstruct(order = ..., endian = ...)]`.

use proc_macro2::{Span, TokenStream};
use syn::Ident;

/// Which end of a byte or word the first field occupies.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// First field in the high bits (mask `0x80` first), as in RFC diagrams.
    Msb0,
    /// First field in the low bits (mask `0x01` first).
    Lsb0,
}

/// Byte order of multi-byte integers.
#[derive(Clone, Copy)]
pub enum Endian {
    Big,
    Little,
    Native,
}

pub struct Options {
    pub order: Order,
    pub endian: Endian,
}

impl Order {
    pub fn name(self) -> &'static str {
        match self {
            Order::Msb0 => "msb0",
            Order::Lsb0 => "lsb0",
        }
    }
}

impl Endian {
    pub fn name(self) -> &'static str {
        match self {
            Endian::Big => "big",
            Endian::Little => "little",
            Endian::Native => "native",
        }
    }

    /// Suffix of Rust's integer conversions, as in `u16::from_be_bytes`.
    pub fn suffix(self) -> &'static str {
        match self {
            Endian::Big => "be",
            Endian::Little => "le",
            Endian::Native => "ne",
        }
    }
}

pub fn parse(args: TokenStream) -> syn::Result<Options> {
    let mut order = None;
    let mut endian = None;

    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("order") {
            if order.is_some() {
                return Err(meta.error("duplicate order"));
            }
            let value: Ident = meta.value()?.parse()?;
            order = Some(match value.to_string().as_str() {
                "msb0" => Order::Msb0,
                "lsb0" => Order::Lsb0,
                _ => return Err(syn::Error::new(value.span(), "expected msb0 or lsb0")),
            });
        } else if meta.path.is_ident("endian") {
            if endian.is_some() {
                return Err(meta.error("duplicate endian"));
            }
            let value: Ident = meta.value()?.parse()?;
            endian = Some(match value.to_string().as_str() {
                "big" => Endian::Big,
                "little" => Endian::Little,
                "native" => Endian::Native,
                _ => {
                    return Err(syn::Error::new(
                        value.span(),
                        "expected big, little, or native",
                    ));
                }
            });
        } else {
            return Err(meta.error("expected order or endian"));
        }
        Ok(())
    });
    syn::parse::Parser::parse2(parser, args)?;

    let missing = |message| syn::Error::new(Span::call_site(), message);
    Ok(Options {
        order: order.ok_or_else(|| missing("order is required"))?,
        endian: endian.ok_or_else(|| missing("endian is required"))?,
    })
}
