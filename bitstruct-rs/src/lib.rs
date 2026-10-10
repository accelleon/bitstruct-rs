#![no_std]
#![doc = include_str!("../README.md")]

extern crate self as bitstruct_rs;

pub use bitstruct_macros::{BitEnum, BitFlags, bitstruct};

/// A failed buffer borrow, field write, or typed field decode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    BufferTooShort { required: usize, actual: usize },
    ValueOutOfRange { bits: u32, value: u128 },
    InvalidValue { value: u128 },
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BufferTooShort { required, actual } => {
                write!(f, "buffer needs {required} bytes, got {actual}")
            }
            Self::ValueOutOfRange { bits, value } => {
                write!(f, "value {value} does not fit in {bits} bits")
            }
            Self::InvalidValue { value } => write!(f, "invalid field value {value}"),
        }
    }
}

impl core::error::Error for Error {}

/// Conversion for enums, flags, and other typed fields annotated with `#[bits(N)]`.
///
/// Implementations must report invalid encodings explicitly. Flag codecs can use
/// `from_bits_retain` to preserve unknown bits. Writes are width-checked by the
/// generated setter before any backing bytes are modified.
pub trait BitCodec: Sized {
    fn from_bits(bits: u128) -> Result<Self, Error>;
    fn to_bits(&self) -> u128;
}

impl BitCodec for bool {
    fn from_bits(bits: u128) -> Result<Self, Error> {
        match bits {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(Error::InvalidValue { value }),
        }
    }

    fn to_bits(&self) -> u128 {
        u128::from(*self)
    }
}

macro_rules! integer_codec {
    ($($ty:ty),* $(,)?) => {$(
        impl BitCodec for $ty {
            fn from_bits(bits: u128) -> Result<Self, Error> {
                Self::try_from(bits).map_err(|_| Error::InvalidValue { value: bits })
            }

            fn to_bits(&self) -> u128 {
                u128::from(*self)
            }
        }
    )*};
}

integer_codec!(u8, u16, u32, u64, u128);
