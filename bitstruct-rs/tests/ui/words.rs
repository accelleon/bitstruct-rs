use bitstruct_rs::bitstruct;

#[bitstruct(order = msb0, endian = big)]
struct Unaligned { a: u3, #[word(u16)] b: u13 }

#[bitstruct(order = msb0, endian = big)]
struct Nested { #[word(u16)] a: u8, #[word(u8)] b: u8 }

#[bitstruct(order = msb0, endian = big)]
struct Overflow { #[word(u16)] a: u3, b: u14, c: u7 }

#[bitstruct(order = msb0, endian = big)]
struct Incomplete { #[word(u16)] a: u8 }

#[bitstruct(order = msb0, endian = big)]
struct Invalid { #[word(u24)] a: u24 }

fn main() {}
