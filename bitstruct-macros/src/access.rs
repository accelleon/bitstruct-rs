//! Writes the code that reads or writes a single field.
//!
//! Everything here is computed while compiling, so the generated accessors
//! contain only constant offsets, shifts and masks.
//!
//! Every access goes through windows: runs of bytes decoded as one native
//! integer, so a wide field costs one load (and one store) per window rather
//! than per byte.
//!
//! Word fields use their word as the window. IPv4 `flags: u3, fragment: u13`
//! in a big-endian `#[word(u16)]` at bytes 6..8 with `msb0`. The `fragment`
//! setter becomes:
//!
//! ```text
//! let window = (u16::from_be_bytes([bytes[6], bytes[7]]) & 0xe000) | (value << 0);
//! bytes[6..8].copy_from_slice(&window.to_be_bytes()[..2]);
//! ```
//!
//! Packed fields within two bytes use one window per byte, as in `dscp: u6`
//! in byte 1 with `msb0`:
//!
//! ```text
//! let window = (u8::from_be_bytes([bytes[1]]) & 0x03) | ((value & 0x3f) << 2);
//! bytes[1..2].copy_from_slice(&window.to_be_bytes()[..1]);
//! ```
//!
//! Wider packed fields are covered by windows of up to 8 bytes, decoded big-endian
//! for `msb0` and little-endian for `lsb0` so that bits stay in address order.
//! A window is widened to a native size by taking neighboring bytes, which are
//! written back unchanged; past the end of the buffer it is padded with zeros
//! that are never stored. `x: u20` at bits 3..23 of a 4-byte `msb0` buffer
//! becomes:
//!
//! ```text
//! let window = (u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) & 0xe00001ff)
//!     | ((value & 0xfffff) << 9);
//! bytes[0..4].copy_from_slice(&window.to_be_bytes()[..4]);
//! ```
//!
//! Fields spanning more than 8 bytes are split into several windows.
//!
//! Values are widened to `u128` inside the expressions; the compiler narrows
//! them back to the real integer sizes.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::layout::{Field, Storage, Word, mask, native_type};
use crate::options::{Options, Order};

/// Expression evaluating to the field's raw bits as a `u128`.
///
/// `byte_len` is the size of the whole buffer.
pub fn read(field: &Field, options: &Options, byte_len: usize) -> TokenStream {
    let parts = chunks(field, options, byte_len).into_iter().map(|chunk| {
        let load = chunk.window.load();
        let Chunk {
            bits,
            shift_in_window,
            shift_in_value,
            ..
        } = chunk;
        let chunk_mask = mask(bits);
        quote!(((((#load as u128) >> #shift_in_window) & #chunk_mask) << #shift_in_value))
    });
    quote!((#(#parts)|*))
}

/// Statements storing `value` (a `u128` already checked to fit) into the field.
pub fn write(field: &Field, options: &Options, byte_len: usize) -> TokenStream {
    let stores = chunks(field, options, byte_len).into_iter().map(|chunk| {
        let Chunk {
            window,
            bits,
            shift_in_window,
            shift_in_value,
        } = chunk;
        let load = window.load();
        let ty = native_type(window.bits);
        let to_bytes = format_ident!("to_{}_bytes", window.endian);
        let (first, end, len) = (
            window.first_byte,
            window.first_byte + window.len,
            window.len,
        );
        let chunk_mask = mask(bits);
        // Bits of the window that belong to other fields or padding.
        let keep = mask(window.bits) ^ (chunk_mask << shift_in_window);
        quote! {
            let window = (((#load as u128) & #keep)
                | (((value >> #shift_in_value) & #chunk_mask) << #shift_in_window)) as #ty;
            self.inner[#first..#end].copy_from_slice(&window.#to_bytes()[..#len]);
        }
    });
    quote!(#(#stores)*)
}

/// Bytes of the buffer decoded as one native integer.
struct Window {
    /// Index of the first byte in the buffer.
    first_byte: usize,
    /// How many bytes come from the buffer; the rest of the integer is zero padding.
    len: usize,
    /// Width of the integer: 8, 16, 32, 64 or 128.
    bits: usize,
    /// Suffix of the byte conversion: `be`, `le` or `ne`.
    endian: &'static str,
}

impl Window {
    /// Expression loading the window, e.g. `u16::from_be_bytes([self.inner[6], self.inner[7]])`.
    fn load(&self) -> TokenStream {
        let ty = native_type(self.bits);
        let from_bytes = format_ident!("from_{}_bytes", self.endian);
        let bytes = self.first_byte..self.first_byte + self.len;
        let padding = (self.len..self.bits / 8).map(|_| quote!(0));
        quote!(#ty::#from_bytes([#(self.inner[#bytes],)* #(#padding,)*]))
    }
}

/// The part of a field stored in one window.
struct Chunk {
    window: Window,
    /// How many of the field's bits are in this window.
    bits: usize,
    /// Distance from the window's lowest bit to this chunk's lowest bit.
    shift_in_window: usize,
    /// Distance from the value's lowest bit to this chunk's lowest bit.
    shift_in_value: usize,
}

fn chunks(field: &Field, options: &Options, byte_len: usize) -> Vec<Chunk> {
    match field.storage {
        Storage::Word(word) => vec![Chunk {
            window: Window {
                first_byte: word.first_byte(),
                len: word.bits / 8,
                bits: word.bits,
                endian: options.endian.suffix(),
            },
            bits: field.bits,
            shift_in_window: shift_within_word(field, word, options.order),
            shift_in_value: 0,
        }],
        Storage::Packed => packed_chunks(field, options.order, byte_len),
    }
}

/// How far the field's lowest bit sits above the word's lowest bit.
fn shift_within_word(field: &Field, word: Word, order: Order) -> usize {
    let bits_before = field.start - word.start;
    match order {
        // First field at the top: skip everything below it.
        Order::Msb0 => word.bits - bits_before - field.bits,
        // First field at the bottom: skip the fields before it.
        Order::Lsb0 => bits_before,
    }
}

/// Covers a packed field with windows of at most 8 bytes.
///
/// `msb0` decodes windows big-endian and puts the first chunk in the value's
/// high bits; `lsb0` mirrors both.
fn packed_chunks(field: &Field, order: Order, byte_len: usize) -> Vec<Chunk> {
    // LLVM splits a two-byte window back into byte loads plus a byte swap, so
    // fields within two bytes are faster handled one byte per window.
    let max_window_bits = if (field.start % 8 + field.bits).div_ceil(8) <= 2 {
        8
    } else {
        64
    };
    let mut chunks = Vec::new();
    let mut done = 0;
    while done < field.bits {
        let position = field.start + done;
        let bits = (max_window_bits - position % 8).min(field.bits - done);
        let span = (position % 8 + bits).div_ceil(8);
        let size = span.next_power_of_two();
        // Prefer starting at the field; slide back if the buffer ends first.
        let first_byte = (position / 8).min(byte_len.saturating_sub(size));
        let offset = position - first_byte * 8;
        let (shift_in_window, shift_in_value) = match order {
            Order::Msb0 => (size * 8 - offset - bits, field.bits - done - bits),
            Order::Lsb0 => (offset, done),
        };
        chunks.push(Chunk {
            window: Window {
                first_byte,
                len: size.min(byte_len - first_byte),
                bits: size * 8,
                endian: match order {
                    Order::Msb0 => "be",
                    Order::Lsb0 => "le",
                },
            },
            bits,
            shift_in_window,
            shift_in_value,
        });
        done += bits;
    }
    chunks
}
