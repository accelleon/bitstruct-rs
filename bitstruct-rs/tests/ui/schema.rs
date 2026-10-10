use bitstruct_rs::bitstruct;

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    struct Partial { a: u3 }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    struct Oversize { a: u129 }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    struct Unknown { a: String }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    struct Collision { a: u8, set_a: u8 }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    #[repr(u16)]
    struct Repr { a: u16 }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    #[derive(Clone)]
    struct Derive { a: u8 }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    struct TypedPadding { _: String, a: u8 }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    struct WideBool { #[bits(2)] a: bool, b: u6 }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    struct BoolPadding { _: bool, a: u7 }
}

fn main() {}
