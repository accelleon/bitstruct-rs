# bitstruct

Zero-copy bitfields over byte slices, with in-place mutation and a `no_std`
runtime. Inside `bitstruct! { ... }`, declare one or more structs marked
`#[bitstruct(order = ..., endian = ...)]` with fields `u1` through `u128` or
`bool`; each becomes an immutable `Name<'a>` and a mutable `NameMut<'a>` view.

```rust
use bitstruct_rs::bitstruct;

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    pub struct Ipv4Prefix {
        version: u4,
        ihl: u4,
        dscp: u6,
        ecn: u2,
        total_length: u16,
        identification: u16,
        #[word(u16)]
        _: u1,
        dont_fragment: bool,
        more_fragments: bool,
        fragment: u13,
    }
}

let mut bytes = [0x45, 0, 0, 20, 0, 0, 0x40, 1];
let mut header = Ipv4PrefixMut::from_bytes(&mut bytes)?;
assert_eq!(header.version(), 4);
assert!(header.dont_fragment());
assert!(!header.more_fragments());
assert_eq!(header.fragment(), 1);
header.set_dscp(46)?;
assert_eq!(header.as_bytes()[1], 0xb8);
# Ok::<(), bitstruct_rs::Error>(())
```

## Layout

Both `order` and `endian` are required:

- `msb0` packs fields from the high bits; `lsb0` packs from the low bits.
- `big`, `little`, or `native` sets the endian of byte-aligned `u8`, `u16`,
  `u32`, `u64`, and `u128` fields and explicit word groups.

`#[word(u16)]` groups consecutive fields into one endian-aware word. The group
must start on a byte boundary and its fields must exactly fill the word.
Supported word sizes are `u8`, `u16`, `u32`, `u64`, and `u128`.

Within a decoded word, `msb0` places the first field in the high bits and `lsb0`
in the low bits. For the example above, the reserved bit, `dont_fragment` set,
`more_fragments` clear and fragment `1` form `0x4001`: `[0x40, 0x01]` in big
endian or `[0x01, 0x40]` in little endian.

Ungrouped packed fields traverse bytes in address order, most-significant-first
for `msb0` or least-significant-first for `lsb0`, independently of endian.
`msb0` packing is therefore already network bit order: IPv6's `version: u4,
priority: u8, flow: u20` decodes the same with or without `#[word(u32)]`.
Schemas must total a whole number of bytes. Avoid `native` endian for portable
wire formats.

Name reserved or padding bits `_`, as in `_: u1` or `_: u8`. They take up space
like any other field, may repeat, and get no accessors, so writes to other
fields leave them unchanged.

## Access

- Numeric getters return the smallest native unsigned integer that fits.
- `bool` fields are one bit and need no `#[bits]`; their getters return `bool`.
- Setters return an error for out-of-range values without changing the buffer.
- `from_bytes()` checks length and borrows only the header-sized prefix.
- `as_view()` and `reborrow()` borrow an existing mutable view.
- `as_bytes()` and `as_bytes_mut()` expose the header bytes.

Mutable views are neither `Copy` nor `Clone`. Split the packet before borrowing
if you need concurrent mutable access to its payload.

## Enums and flags

Use `#[bits(N)]` for a type implementing `BitCodec`. `BitEnum` derives this
trait for fieldless enums with an unsigned repr and explicit integer-literal
discriminants. Unknown encodings return `Error::InvalidValue`; a generated
`field_raw()` getter exposes the bits without decoding.

```rust
use bitstruct_rs::{bitstruct, BitEnum};

#[derive(BitEnum, Debug, PartialEq)]
#[repr(u8)]
pub enum Ecn { NotEct = 0, Ect1 = 1, Ect0 = 2, Congestion = 3 }

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    pub struct Traffic {
        dscp: u6,
        #[bits(2)]
        ecn: Ecn,
    }
}

let bytes = [0xb9];
assert_eq!(Traffic::from_bytes(&bytes)?.ecn()?, Ecn::Ect1);
# Ok::<(), bitstruct_rs::Error>(())
```

For a `bitflags` type with unsigned storage, derive `BitFlags` to generate the
codec. Unknown in-width bits are retained:

```rust
use bitstruct_rs::{bitstruct, BitFlags, Error};
bitflags::bitflags! {
    #[derive(BitFlags)]
    pub struct Flags: u8 {
        const MORE_FRAGMENTS = 1;
        const DONT_FRAGMENT = 2;
    }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    pub struct Fragment {
        #[word(u16)]
        #[bits(3)]
        flags: Flags,
        offset: u13,
    }
}
let mut bytes = [0, 0];
let mut view = FragmentMut::from_bytes(&mut bytes)?;
view.set_flags(Flags::DONT_FRAGMENT | Flags::MORE_FRAGMENTS)?;
# Ok::<(), Error>(())
```

## Development

```sh
cargo test -p bitstruct -p bitstruct-macros
cargo bench -p bitstruct --bench access
```
