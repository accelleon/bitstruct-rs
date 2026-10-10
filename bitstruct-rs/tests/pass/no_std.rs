#![no_std]
extern crate std;

use bitstruct_rs::{bitstruct, BitEnum};

bitflags::bitflags! {
    #[derive(bitstruct_rs::BitFlags)]
    struct Flags: u8 {
        const A = 1;
    }
}

#[derive(BitEnum)]
#[repr(u16)]
enum Mode { A = 0, B = 1 }

bitstruct! {
    #[bitstruct(order = lsb0, endian = native)]
    pub struct Header {
        #[word(u16)]
        #[bits(2)]
        mode: Mode,
        rest: u14,
    }
}

fn main() {
    let flags = <Flags as bitstruct_rs::BitCodec>::from_bits(3).unwrap();
    assert_eq!(flags.bits(), 3);
    let mut bytes = [0; Header::BYTE_LEN];
    let mut header = HeaderMut::from_bytes(&mut bytes).unwrap();
    header.set_mode(Mode::B).unwrap();
    assert_eq!(header.mode_raw(), 1);
}
