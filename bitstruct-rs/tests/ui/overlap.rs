use bitstruct_rs::bitstruct;

#[bitstruct(order = msb0, endian = big)]
struct Header { a: u8 }

fn main() {
    let mut bytes = [0];
    let mut view = HeaderMut::from_bytes(&mut bytes).unwrap();
    let mut first = view.reborrow();
    let mut second = view.reborrow();
    first.set_a(1).unwrap();
    second.set_a(2).unwrap();
}
