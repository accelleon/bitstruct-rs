//! Turns the struct declaration into fields with fixed bit positions.
//!
//! Fields are laid out back to back in declaration order. Bit positions count
//! from the start of the buffer: bits 0..8 are byte 0, bits 8..16 are byte 1,
//! and so on. For example, IPv4's `version: u4, ihl: u4, dscp: u6` occupy bits
//! 0..4, 4..8 and 8..14.

use quote::format_ident;
use syn::{Attribute, Fields, Ident, ItemStruct, LitInt, Type, spanned::Spanned};

/// Sizes of Rust's native unsigned integers: `u8` to `u128`.
pub const WORD_SIZES: [usize; 5] = [8, 16, 32, 64, 128];

pub struct Layout {
    pub fields: Vec<Field>,
    pub total_bits: usize,
}

pub struct Field {
    pub name: Ident,
    /// Type taken and returned by the accessors: a native integer for `uN`
    /// fields, or the user's enum/flags type.
    pub ty: Type,
    /// True if values convert through `BitCodec` (enums, flags, `bool`).
    pub uses_codec: bool,
    /// True for `_: uN`: reserved bits that get no accessors.
    pub padding: bool,
    /// Width in bits.
    pub bits: usize,
    /// Position of the field's first bit within the buffer.
    pub start: usize,
    pub storage: Storage,
    pub docs: Vec<Attribute>,
}

/// How a field's bits are found in the buffer.
pub enum Storage {
    /// Part of an integer that is read and written whole, honoring `endian`.
    Word(Word),
    /// Spread across one or more bytes in address order, ignoring `endian`.
    Packed,
}

/// An integer of `bits` bits whose first bit is at `start`.
#[derive(Clone, Copy)]
pub struct Word {
    pub start: usize,
    pub bits: usize,
}

impl Word {
    pub fn end(self) -> usize {
        self.start + self.bits
    }

    pub fn first_byte(self) -> usize {
        self.start / 8
    }
}

pub fn parse(item: &ItemStruct) -> syn::Result<Layout> {
    reject_unsupported(item)?;
    let Fields::Named(named) = &item.fields else {
        return Err(syn::Error::new(item.span(), "expected named fields"));
    };
    if named.named.is_empty() {
        return Err(syn::Error::new(item.span(), "schema must not be empty"));
    }

    let mut fields = Vec::new();
    let mut next_bit: usize = 0;
    // The `#[word(..)]` group currently being filled, if any.
    let mut open_word: Option<Word> = None;

    for field in &named.named {
        let attrs = FieldAttrs::parse(field)?;

        if let Some(bits) = attrs.word {
            if open_word.is_some() || !next_bit.is_multiple_of(8) {
                return Err(syn::Error::new(
                    field.span(),
                    "word must start at a byte boundary outside another word",
                ));
            }
            open_word = Some(Word {
                start: next_bit,
                bits,
            });
        }

        // `u6` declares its own width; other types need `#[bits(N)]`.
        let declared = unsigned_width(&field.ty);
        let uses_codec = attrs.bits.is_some() || declared.is_none();
        let name = field.ident.clone().expect("named field");
        let padding = name == "_";
        if padding && uses_codec {
            return Err(syn::Error::new(
                field.ty.span(),
                "padding fields must be u1..=u128",
            ));
        }
        let bits = attrs
            .bits
            .or(declared)
            .ok_or_else(|| syn::Error::new(field.ty.span(), "typed fields require #[bits(N)]"))?;
        if !(1..=128).contains(&bits) {
            return Err(syn::Error::new(field.span(), "field width must be 1..=128"));
        }
        let end = next_bit
            .checked_add(bits)
            .ok_or_else(|| syn::Error::new(field.span(), "schema is too large"))?;
        if open_word.is_some_and(|word| end > word.end()) {
            return Err(syn::Error::new(field.span(), "field exceeds word boundary"));
        }

        let storage = match open_word {
            Some(word) => Storage::Word(word),
            // A byte-aligned u8/u16/u32/u64/u128 is its own word, so it honors `endian`.
            None if next_bit.is_multiple_of(8) && WORD_SIZES.contains(&bits) => {
                Storage::Word(Word {
                    start: next_bit,
                    bits,
                })
            }
            None => Storage::Packed,
        };
        let ty = if uses_codec {
            field.ty.clone()
        } else {
            let native = native_type(bits);
            syn::parse_quote!(#native)
        };

        fields.push(Field {
            name,
            ty,
            uses_codec,
            padding,
            bits,
            start: next_bit,
            storage,
            docs: attrs.docs,
        });

        next_bit = end;
        if open_word.is_some_and(|word| word.end() == next_bit) {
            open_word = None;
        }
    }

    if open_word.is_some() || !next_bit.is_multiple_of(8) {
        return Err(syn::Error::new(
            item.span(),
            "schema and word groups must be fully byte-sized",
        ));
    }
    Ok(Layout {
        fields,
        total_bits: next_bit,
    })
}

/// The struct is replaced by generated views, so most struct-level options
/// (generics, derives, repr) would be silently meaningless. Reject them.
fn reject_unsupported(item: &ItemStruct) -> syn::Result<()> {
    if !item.generics.params.is_empty() || item.generics.where_clause.is_some() {
        return Err(syn::Error::new(
            item.generics.span(),
            "generics are not supported",
        ));
    }
    for attr in &item.attrs {
        if !attr.path().is_ident("doc") {
            return Err(syn::Error::new(
                attr.span(),
                "only documentation attributes are supported on a bitstruct schema",
            ));
        }
    }
    Ok(())
}

/// Attributes allowed on a field: `#[bits(N)]`, `#[word(uN)]` and doc comments.
struct FieldAttrs {
    bits: Option<usize>,
    word: Option<usize>,
    docs: Vec<Attribute>,
}

impl FieldAttrs {
    fn parse(field: &syn::Field) -> syn::Result<Self> {
        let mut result = FieldAttrs {
            bits: None,
            word: None,
            docs: Vec::new(),
        };
        for attr in &field.attrs {
            if attr.path().is_ident("bits") {
                if result.bits.is_some() {
                    return Err(syn::Error::new(attr.span(), "duplicate bits"));
                }
                result.bits = Some(attr.parse_args::<LitInt>()?.base10_parse()?);
            } else if attr.path().is_ident("word") {
                if result.word.is_some() {
                    return Err(syn::Error::new(attr.span(), "duplicate word"));
                }
                let ty: Type = attr.parse_args()?;
                let bits = unsigned_width(&ty)
                    .filter(|bits| WORD_SIZES.contains(bits))
                    .ok_or_else(|| {
                        syn::Error::new(ty.span(), "expected u8, u16, u32, u64, or u128")
                    })?;
                result.word = Some(bits);
            } else if attr.path().is_ident("doc") {
                result.docs.push(attr.clone());
            } else {
                return Err(syn::Error::new(attr.span(), "expected bits, word, or doc"));
            }
        }
        Ok(result)
    }
}

/// Width of an unsigned type name such as `u6` or `u16`; `None` for anything else.
pub fn unsigned_width(ty: &Type) -> Option<usize> {
    let Type::Path(path) = ty else { return None };
    if path.qself.is_some() || path.path.segments.len() != 1 {
        return None;
    }
    let segment = path.path.segments.first()?;
    if !matches!(segment.arguments, syn::PathArguments::None) {
        return None;
    }
    segment.ident.to_string().strip_prefix('u')?.parse().ok()
}

/// Smallest native unsigned integer holding `bits` bits: 6 -> `u8`, 13 -> `u16`.
pub fn native_type(bits: usize) -> Ident {
    let size = WORD_SIZES
        .into_iter()
        .find(|&size| size >= bits)
        .expect("width validated to be at most 128");
    format_ident!("u{}", size)
}

/// A value with the low `bits` bits set: 3 -> `0b111`.
pub fn mask(bits: usize) -> u128 {
    u128::MAX >> (128 - bits)
}
