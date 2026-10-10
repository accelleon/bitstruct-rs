use bitstruct_rs::bitstruct;

#[bitstruct(order = msb0, endian = big)]
struct Header { a: u8 }

fn main() {
    let mut bytes = [0];
    let view = HeaderMut::from_bytes(&mut bytes).unwrap();
    let _copy = view.clone();
}
