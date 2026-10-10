use bitstruct_rs::bitstruct;

#[bitstruct(order = msb0, endian = big)]
struct Header { a: u8 }

fn main() {
    let mut bytes = [0];
    let mut view = HeaderMut::from_bytes(&mut bytes).unwrap();
    let shared = view.as_view();
    view.set_a(2).unwrap();
    println!("{}", shared.a());
}
