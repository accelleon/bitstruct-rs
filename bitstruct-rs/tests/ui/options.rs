use bitstruct_rs::bitstruct;

bitstruct! {
    #[bitstruct(endian = big)]
    struct Missing { a: u8 }
}

bitstruct! {
    #[bitstruct(order = msb, endian = big)]
    struct Invalid { a: u8 }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = big, endian = little)]
    struct Duplicate { a: u8 }
}

bitstruct! {
    #[bitstruct(order = lsb0)]
    struct MissingEndian { a: u8 }
}

bitstruct! {
    #[bitstruct(order = lsb0, endian = sideways)]
    struct InvalidEndian { a: u8 }
}

bitstruct! {
    #[bitstruct(order = msb0, order = lsb0, endian = big)]
    struct DuplicateOrder { a: u8 }
}

bitstruct! {
    struct NoOptions { a: u8 }
}

bitstruct! {
    #[bitstruct]
    struct EmptyOptions { a: u8 }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    #[bitstruct(order = msb0, endian = big)]
    struct TwiceOptions { a: u8 }
}

fn main() {}
