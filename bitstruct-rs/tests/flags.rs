use bitstruct_rs::{BitCodec, BitFlags, Error, bitstruct};

macro_rules! check_storage {
    ($test:ident, $ty:ty) => {
        #[test]
        fn $test() {
            bitflags::bitflags! {
                #[derive(Debug, PartialEq, Eq, BitFlags)]
                struct Flags: $ty {
                    const A = 1;
                    const B = 2;
                }
            }
            for value in [0, 1, 3, 7, <$ty>::MAX as u128] {
                let flags = <Flags as BitCodec>::from_bits(value).unwrap();
                assert_eq!(BitCodec::to_bits(&flags), value);
            }
            let combined = Flags::A | Flags::B;
            assert_eq!(BitCodec::to_bits(&combined), 3);
            if let Some(value) = (<$ty>::MAX as u128).checked_add(1) {
                assert_eq!(
                    <Flags as BitCodec>::from_bits(value),
                    Err(Error::InvalidValue { value })
                );
            }
        }
    };
}

check_storage!(u8_storage, u8);
check_storage!(u16_storage, u16);
check_storage!(u32_storage, u32);
check_storage!(u64_storage, u64);
check_storage!(u128_storage, u128);

bitflags::bitflags! {
    #[derive(Debug, PartialEq, Eq, BitFlags)]
    struct Flags: u16 {
        const A = 1;
        const B = 2;
    }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = little)]
    struct Header {
        #[word(u16)]
        #[bits(3)]
        flags: Flags,
        offset: u13,
    }
}

#[test]
fn packed_flags_preserve_unknown_bits_and_reject_overflow() {
    let mut bytes = 0x8001u16.to_le_bytes();
    let mut header = HeaderMut::from_bytes(&mut bytes).unwrap();
    assert_eq!(header.flags().unwrap().bits(), 4);
    header.set_flags(Flags::A | Flags::B).unwrap();
    assert_eq!(header.offset(), 1);
    header.set_flags(Flags::from_bits_retain(7)).unwrap();
    assert_eq!(header.flags().unwrap().bits(), 7);
    assert_eq!(header.as_view().flags_raw(), 7);
    let before: [u8; 2] = header.as_bytes().try_into().unwrap();
    assert_eq!(
        header.set_flags(Flags::from_bits_retain(8)),
        Err(Error::ValueOutOfRange { bits: 3, value: 8 })
    );
    assert_eq!(header.as_bytes(), before);
}
