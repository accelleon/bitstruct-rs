//! Generated items must be documented even when schema fields are not.
#![deny(missing_docs)]

use bitstruct_rs::{bitstruct, BitEnum};

/// A typed field.
#[derive(BitEnum)]
#[repr(u8)]
pub enum Mode {
    /// First.
    A = 0,
    /// Second.
    B = 1,
}

bitstruct! {
    /// A public schema.
    #[bitstruct(order = msb0, endian = big)]
    pub struct Header {
        /// A documented field.
        documented: u4,
        undocumented: u2,
        #[bits(1)]
        mode: Mode,
        flag: bool,
        #[word(u16)]
        high: u4,
        low: u12,
        _: u8,
    }
}

fn main() {}
