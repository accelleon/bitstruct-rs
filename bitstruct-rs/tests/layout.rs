use bitstruct_rs::{BitCodec, BitEnum, Error, bitstruct};

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    pub struct Ipv4 {
        version: u4,
        ihl: u4,
        dscp: u6,
        ecn: u2,
        total_length: u16,
        identification: u16,
        flags: u3,
        fragment: u13,
        ttl: u8,
        protocol: u8,
        checksum: u16,
        source: u32,
        destination: u32,
    }
}

// RFC 8200. `msb0` packed fields already follow network bit order, so the
// first 32 bits need no `#[word]`.
bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    pub struct Ipv6 {
        version: u4,
        priority: u8,
        flow: u20,
        payload_length: u16,
        next_header: u8,
        hop_limit: u8,
        source: u128,
        destination: u128,
    }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    pub struct Ipv6Word {
        #[word(u32)]
        version: u4,
        priority: u8,
        flow: u20,
        payload_length: u16,
        next_header: u8,
        hop_limit: u8,
        source: u128,
        destination: u128,
    }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    struct Big {
        #[word(u16)]
        flags: u3,
        fragment: u13,
    }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = little)]
    struct Little {
        #[word(u16)]
        flags: u3,
        fragment: u13,
    }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = native)]
    struct Native {
        #[word(u16)]
        flags: u3,
        fragment: u13,
    }
}

bitstruct! {
    #[bitstruct(order = lsb0, endian = big)]
    struct LsbBig {
        #[word(u16)]
        flags: u3,
        fragment: u13,
    }
}

bitstruct! {
    #[bitstruct(order = lsb0, endian = little)]
    struct LsbLittle {
        #[word(u16)]
        flags: u3,
        fragment: u13,
    }
}

bitstruct! {
    #[bitstruct(order = lsb0, endian = little)]
    struct Lsb {
        first: u3,
        middle: u10,
        last: u3,
    }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    struct Msb {
        first: u3,
        middle: u10,
        last: u3,
    }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = little)]
    struct Wide {
        a: u8,
        b: u16,
        c: u32,
        d: u64,
        e: u128,
        prefix: u3,
        unaligned: u128,
        suffix: u5,
    }
}

bitstruct! {
    #[bitstruct(order = lsb0, endian = big)]
    struct LsbWide {
        prefix: u7,
        unaligned: u128,
        suffix: u1,
        #[word(u128)]
        high: u1,
        low: u127,
    }
}

#[derive(Debug, PartialEq, Eq, BitEnum)]
#[repr(u8)]
enum Mode {
    A = 0,
    B = 0x2,
    TooWide = 4,
}

bitflags::bitflags! {
    #[derive(Debug, PartialEq, Eq, bitstruct_rs::BitFlags)]
    struct Flags: u8 {
        const A = 1;
        const B = 2;
    }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    struct Typed {
        #[bits(2)]
        mode: Mode,
        #[bits(3)]
        flags: Flags,
        enabled: bool,
        reserved: u2,
    }
}

#[test]
fn ipv4_layout_and_in_place_mutation() {
    let mut bytes = [
        0x45, 0xb9, 0x12, 0x34, 0xab, 0xcd, 0x40, 1, 64, 17, 0xfe, 0xdc, 192, 0, 2, 1, 198, 51,
        100, 2, 0xaa, 0xbb,
    ];
    let pointer = bytes.as_ptr();
    let mut view = Ipv4Mut::from_bytes(&mut bytes).unwrap();
    assert_eq!(view.as_bytes().as_ptr(), pointer);
    assert_eq!(view.as_bytes().len(), 20);
    assert_eq!(Ipv4::BIT_LEN, 160);
    assert_eq!(view.version(), 4);
    assert_eq!(view.ihl(), 5);
    assert_eq!(view.dscp(), 46);
    assert_eq!(view.ecn(), 1);
    assert_eq!(view.total_length(), 0x1234);
    assert_eq!(view.identification(), 0xabcd);
    assert_eq!(view.flags(), 2);
    assert_eq!(view.fragment(), 1);
    assert_eq!(view.ttl(), 64);
    assert_eq!(view.protocol(), 17);
    assert_eq!(view.checksum(), 0xfedc);
    assert_eq!(view.source(), 0xc0000201);
    assert_eq!(view.destination(), 0xc6336402);
    let before: [u8; 20] = view.as_bytes().try_into().unwrap();
    view.set_dscp(3).unwrap();
    assert_eq!(view.as_bytes()[1], 0x0d);
    assert_eq!(&view.as_bytes()[2..], &before[2..]);
    assert_eq!(view.as_bytes()[0], before[0]);
    view.set_flags(7).unwrap();
    view.set_fragment(0x1234).unwrap();
    assert_eq!(&view.as_bytes()[6..8], &[0xf2, 0x34]);
    {
        let mut short = view.reborrow();
        short.set_ttl(63).unwrap();
        assert_eq!(short.as_view().ttl(), 63);
    }
    assert_eq!(bytes[8], 63);
    assert_eq!(&bytes[20..], &[0xaa, 0xbb]);
}

const IPV6: [u8; 40] = [
    0x6b, 0x8a, 0xbc, 0xde, // version 6, priority 0xb8, flow 0xabcde
    0x00, 0x20, 0x06, 0x40, // payload length 32, TCP, hop limit 64
    0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x01, // 2001:db8::1
    0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x02, // 2001:db8::2
];

#[test]
fn ipv6_header_without_word() {
    use std::net::Ipv6Addr;

    let view = Ipv6::from_bytes(&IPV6).unwrap();
    assert_eq!(Ipv6::BYTE_LEN, 40);
    assert_eq!(view.version(), 6);
    assert_eq!(view.priority(), 0xb8);
    assert_eq!(view.flow(), 0xabcde);
    assert_eq!(view.payload_length(), 32);
    assert_eq!(view.next_header(), 6);
    assert_eq!(view.hop_limit(), 64);
    assert_eq!(
        Ipv6Addr::from(view.source()),
        "2001:db8::1".parse::<Ipv6Addr>().unwrap()
    );
    assert_eq!(
        Ipv6Addr::from(view.destination()),
        "2001:db8::2".parse::<Ipv6Addr>().unwrap()
    );

    let mut bytes = IPV6;
    let mut view = Ipv6Mut::from_bytes(&mut bytes).unwrap();
    view.set_priority(0x02).unwrap();
    view.set_flow(0x12345).unwrap();
    view.set_hop_limit(63).unwrap();
    view.set_destination(Ipv6Addr::LOCALHOST.into()).unwrap();
    assert_eq!(view.version(), 6);
    assert_eq!(view.payload_length(), 32);
    assert_eq!(
        &bytes[..8],
        &[0x60, 0x21, 0x23, 0x45, 0x00, 0x20, 0x06, 0x3f]
    );
    assert_eq!(&bytes[8..24], &IPV6[8..24]);
    assert_eq!(&bytes[24..], &Ipv6Addr::LOCALHOST.octets());
    assert!(matches!(
        Ipv6Mut::from_bytes(&mut bytes).unwrap().set_flow(1 << 20),
        Err(Error::ValueOutOfRange { bits: 20, .. })
    ));
}

#[test]
fn ipv6_packed_matches_word() {
    let mut state = 0x2545_f491_4f6c_dd1du64;
    let mut random = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state as u32
    };
    for round in 0..20_000 {
        let first = match round {
            0 => 0,
            1 => u32::MAX,
            _ => random(),
        };
        let mut bytes = IPV6;
        bytes[..4].copy_from_slice(&first.to_be_bytes());
        let packed = Ipv6::from_bytes(&bytes).unwrap();
        let word = Ipv6Word::from_bytes(&bytes).unwrap();
        assert_eq!(packed.version(), (first >> 28) as u8);
        assert_eq!(packed.priority(), (first >> 20) as u8);
        assert_eq!(packed.flow(), first & 0xf_ffff);
        assert_eq!(word.version(), packed.version());
        assert_eq!(word.priority(), packed.priority());
        assert_eq!(word.flow(), packed.flow());

        let (version, priority, flow) =
            ((random() & 0xf) as u8, random() as u8, random() & 0xf_ffff);
        let mut expected = bytes;
        expected[..4].copy_from_slice(
            &((u32::from(version) << 28) | (u32::from(priority) << 20) | flow).to_be_bytes(),
        );
        let mut packed_bytes = bytes;
        let mut view = Ipv6Mut::from_bytes(&mut packed_bytes).unwrap();
        view.set_version(version).unwrap();
        view.set_priority(priority).unwrap();
        view.set_flow(flow).unwrap();
        let mut word_bytes = bytes;
        let mut view = Ipv6WordMut::from_bytes(&mut word_bytes).unwrap();
        view.set_version(version).unwrap();
        view.set_priority(priority).unwrap();
        view.set_flow(flow).unwrap();
        assert_eq!(packed_bytes, expected);
        assert_eq!(word_bytes, expected);
    }
}

// Views only borrow, so an enum of them is a tagged union over one buffer:
// the version nibble picks the layout, and each variant checks its own length.
bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    struct IpVersion {
        version: u4,
        low: u4,
    }
}

enum IpHeader<'a> {
    V4(Ipv4<'a>),
    V6(Ipv6<'a>),
}

enum IpHeaderMut<'a> {
    V4(Ipv4Mut<'a>),
    V6(Ipv6Mut<'a>),
}

impl<'a> IpHeader<'a> {
    fn from_bytes(bytes: &'a [u8]) -> Result<Self, Error> {
        match IpVersion::from_bytes(bytes)?.version() {
            4 => Ipv4::from_bytes(bytes).map(Self::V4),
            6 => Ipv6::from_bytes(bytes).map(Self::V6),
            version => Err(Error::InvalidValue {
                value: version.into(),
            }),
        }
    }

    fn hop_limit(&self) -> u8 {
        match self {
            Self::V4(header) => header.ttl(),
            Self::V6(header) => header.hop_limit(),
        }
    }
}

impl<'a> IpHeaderMut<'a> {
    fn from_bytes(bytes: &'a mut [u8]) -> Result<Self, Error> {
        let version = IpVersion::from_bytes(bytes)?.version();
        match version {
            4 => Ipv4Mut::from_bytes(bytes).map(Self::V4),
            6 => Ipv6Mut::from_bytes(bytes).map(Self::V6),
            version => Err(Error::InvalidValue {
                value: version.into(),
            }),
        }
    }

    fn decrement_hop_limit(&mut self) -> Result<(), Error> {
        match self {
            Self::V4(header) => header.set_ttl(header.ttl() - 1),
            Self::V6(header) => header.set_hop_limit(header.hop_limit() - 1),
        }
    }
}

#[test]
fn ip_header_selects_layout_by_version() {
    // A bare 20-byte IPv4 header is accepted although IPv6 needs 40.
    let mut ipv4 = [
        0x45, 0, 0, 20, 0, 0, 0x40, 0, 64, 17, 0, 0, 192, 0, 2, 1, 198, 51, 100, 2,
    ];
    let mut ipv6 = IPV6;

    let IpHeader::V4(header) = IpHeader::from_bytes(&ipv4).unwrap() else {
        panic!("expected IPv4");
    };
    assert_eq!(header.source(), 0xc0000201);
    assert_eq!(IpHeader::from_bytes(&ipv4).unwrap().hop_limit(), 64);
    let IpHeader::V6(header) = IpHeader::from_bytes(&ipv6).unwrap() else {
        panic!("expected IPv6");
    };
    assert_eq!(header.flow(), 0xabcde);
    assert_eq!(IpHeader::from_bytes(&ipv6).unwrap().hop_limit(), 64);

    IpHeaderMut::from_bytes(&mut ipv4)
        .unwrap()
        .decrement_hop_limit()
        .unwrap();
    IpHeaderMut::from_bytes(&mut ipv6)
        .unwrap()
        .decrement_hop_limit()
        .unwrap();
    assert_eq!(ipv4[8], 63);
    assert_eq!(ipv6[7], 63);
    assert_eq!(&ipv6[8..], &IPV6[8..]);

    // An IPv6 version nibble on a 20-byte buffer is too short, not misread.
    ipv4[0] = 0x60;
    assert!(matches!(
        IpHeader::from_bytes(&ipv4),
        Err(Error::BufferTooShort {
            required: 40,
            actual: 20
        })
    ));
    ipv4[0] = 0x55;
    assert!(matches!(
        IpHeader::from_bytes(&ipv4),
        Err(Error::InvalidValue { value: 5 })
    ));
    assert!(matches!(
        IpHeader::from_bytes(&[]),
        Err(Error::BufferTooShort {
            required: 1,
            actual: 0
        })
    ));
}

#[test]
fn all_dscp_ecn_encodings_preserve_neighbors() {
    for initial in 0..=255 {
        let mut bytes = [
            0x45, initial, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let mut view = Ipv4Mut::from_bytes(&mut bytes).unwrap();
        assert_eq!(view.dscp(), initial >> 2);
        assert_eq!(view.ecn(), initial & 3);
        for dscp in 0..64 {
            view.set_dscp(dscp).unwrap();
            assert_eq!(view.as_bytes()[1], (dscp << 2) | (initial & 3));
            assert_eq!(view.ecn(), initial & 3);
        }
    }
}

#[test]
fn word_order_and_endian_are_independent() {
    let mut big = [0; 2];
    let mut little = [0; 2];
    let mut native = [0; 2];
    let mut lsb_big = [0; 2];
    let mut lsb_little = [0; 2];
    macro_rules! set {
        ($ty:ident, $bytes:ident) => {{
            let mut view = $ty::from_bytes(&mut $bytes).unwrap();
            view.set_flags(2).unwrap();
            view.set_fragment(1).unwrap();
            assert_eq!(view.flags(), 2);
            assert_eq!(view.fragment(), 1);
        }};
    }
    set!(BigMut, big);
    set!(LittleMut, little);
    set!(NativeMut, native);
    set!(LsbBigMut, lsb_big);
    set!(LsbLittleMut, lsb_little);
    assert_eq!(big, 0x4001u16.to_be_bytes());
    assert_eq!(little, 0x4001u16.to_le_bytes());
    assert_eq!(native, 0x4001u16.to_ne_bytes());
    assert_eq!(lsb_big, 10u16.to_be_bytes());
    assert_eq!(lsb_little, 10u16.to_le_bytes());
}

#[test]
fn exhaustive_word_decoding_and_neighbor_preserving_writes() {
    for word in 0..=u16::MAX {
        let big_bytes = word.to_be_bytes();
        let little_bytes = word.to_le_bytes();
        let big = Big::from_bytes(&big_bytes).unwrap();
        let little = Little::from_bytes(&little_bytes).unwrap();
        let lsb_big = LsbBig::from_bytes(&big_bytes).unwrap();
        let lsb_little = LsbLittle::from_bytes(&little_bytes).unwrap();
        assert_eq!(big.flags(), (word >> 13) as u8);
        assert_eq!(little.flags(), (word >> 13) as u8);
        assert_eq!(big.fragment(), word & 0x1fff);
        assert_eq!(little.fragment(), word & 0x1fff);
        assert_eq!(lsb_big.flags(), (word & 7) as u8);
        assert_eq!(lsb_little.flags(), (word & 7) as u8);
        assert_eq!(lsb_big.fragment(), word >> 3);
        assert_eq!(lsb_little.fragment(), word >> 3);

        let mut big_bytes = big_bytes;
        let mut little_bytes = little_bytes;
        let new_fragment = (word.wrapping_add(1)) & 0x1fff;
        let expected = (word & 0xe000) | new_fragment;
        BigMut::from_bytes(&mut big_bytes)
            .unwrap()
            .set_fragment(new_fragment)
            .unwrap();
        LittleMut::from_bytes(&mut little_bytes)
            .unwrap()
            .set_fragment(new_fragment)
            .unwrap();
        assert_eq!(big_bytes, expected.to_be_bytes());
        assert_eq!(little_bytes, expected.to_le_bytes());
    }
}

// Independent, deliberately bit-at-a-time reference for packed fields.
fn reference_read(bytes: &[u8], start: usize, width: usize, msb: bool) -> u128 {
    let mut value = 0;
    for i in 0..width {
        let at = start + i;
        let physical = if msb { 7 - at % 8 } else { at % 8 };
        let bit = (bytes[at / 8] >> physical) & 1;
        let significance = if msb { width - i - 1 } else { i };
        value |= u128::from(bit) << significance;
    }
    value
}

fn reference_write(bytes: &mut [u8], start: usize, width: usize, value: u128, msb: bool) {
    for i in 0..width {
        let at = start + i;
        let physical = if msb { 7 - at % 8 } else { at % 8 };
        let significance = if msb { width - i - 1 } else { i };
        bytes[at / 8] = (bytes[at / 8] & !(1 << physical))
            | ((((value >> significance) & 1) as u8) << physical);
    }
}

#[test]
fn exhaustive_unaligned_small_fields_both_orders() {
    for word in 0..=u16::MAX {
        let bytes = word.to_be_bytes();
        let msb = Msb::from_bytes(&bytes).unwrap();
        let lsb = Lsb::from_bytes(&bytes).unwrap();
        assert_eq!(u128::from(msb.first()), reference_read(&bytes, 0, 3, true));
        assert_eq!(
            u128::from(msb.middle()),
            reference_read(&bytes, 3, 10, true)
        );
        assert_eq!(u128::from(msb.last()), reference_read(&bytes, 13, 3, true));
        assert_eq!(u128::from(lsb.first()), reference_read(&bytes, 0, 3, false));
        assert_eq!(
            u128::from(lsb.middle()),
            reference_read(&bytes, 3, 10, false)
        );
        assert_eq!(u128::from(lsb.last()), reference_read(&bytes, 13, 3, false));
    }
    for value in 0..1024 {
        let mut msb = [0xa5, 0x5a];
        let mut lsb = msb;
        let mut expected_msb = msb;
        let mut expected_lsb = msb;
        reference_write(&mut expected_msb, 3, 10, value, true);
        reference_write(&mut expected_lsb, 3, 10, value, false);
        MsbMut::from_bytes(&mut msb)
            .unwrap()
            .set_middle(value as u16)
            .unwrap();
        LsbMut::from_bytes(&mut lsb)
            .unwrap()
            .set_middle(value as u16)
            .unwrap();
        assert_eq!(msb, expected_msb);
        assert_eq!(lsb, expected_lsb);
    }
}

#[test]
fn scalars_and_unaligned_u128() {
    let mut bytes = [0xa5; Wide::BYTE_LEN];
    let mut view = WideMut::from_bytes(&mut bytes).unwrap();
    view.set_a(u8::MAX).unwrap();
    view.set_b(0x1234).unwrap();
    view.set_c(0x12345678).unwrap();
    view.set_d(0x123456789abcdef0).unwrap();
    view.set_e(u128::MAX - 1).unwrap();
    assert_eq!(view.a(), u8::MAX);
    assert_eq!(view.b(), 0x1234);
    assert_eq!(view.c(), 0x12345678);
    assert_eq!(view.d(), 0x123456789abcdef0);
    assert_eq!(view.e(), u128::MAX - 1);
    assert_eq!(&view.as_bytes()[1..3], &[0x34, 0x12]);
    let prefix = view.prefix();
    let suffix = view.suffix();
    for value in [0, 1, u128::MAX, 0x123456789abcdef0123456789abcdef0] {
        let mut expected = [0; Wide::BYTE_LEN];
        expected.copy_from_slice(view.as_bytes());
        reference_write(&mut expected, 31 * 8 + 3, 128, value, true);
        view.set_unaligned(value).unwrap();
        assert_eq!(view.as_bytes(), expected);
        assert_eq!(view.unaligned(), value);
        assert_eq!(view.prefix(), prefix);
        assert_eq!(view.suffix(), suffix);
    }
}

#[test]
fn lsb_unaligned_and_full_width_words() {
    let mut bytes = [0x5a; LsbWide::BYTE_LEN];
    let mut view = LsbWideMut::from_bytes(&mut bytes).unwrap();
    let prefix = view.prefix();
    let suffix = view.suffix();
    for value in [0, 1, u128::MAX, 0x123456789abcdef0123456789abcdef0] {
        let mut expected = [0; LsbWide::BYTE_LEN];
        expected.copy_from_slice(view.as_bytes());
        reference_write(&mut expected, 7, 128, value, false);
        view.set_unaligned(value).unwrap();
        assert_eq!(view.as_bytes(), expected);
        assert_eq!(view.unaligned(), value);
        assert_eq!(view.prefix(), prefix);
        assert_eq!(view.suffix(), suffix);
    }
    view.set_high(1).unwrap();
    view.set_low((u128::MAX >> 1) - 1).unwrap();
    assert_eq!(&view.as_bytes()[17..], &(u128::MAX - 2).to_be_bytes());
    assert_eq!(view.high(), 1);
    assert_eq!(view.low(), (u128::MAX >> 1) - 1);
    let before: [u8; LsbWide::BYTE_LEN] = view.as_bytes().try_into().unwrap();
    assert!(matches!(
        view.set_low(u128::MAX),
        Err(Error::ValueOutOfRange { bits: 127, .. })
    ));
    assert_eq!(view.as_bytes(), before);
}

#[test]
fn errors_are_explicit_and_writes_are_atomic() {
    assert!(matches!(
        Ipv4::from_bytes(&[0; 19]),
        Err(Error::BufferTooShort {
            required: 20,
            actual: 19
        })
    ));
    assert!(matches!(
        Ipv4Mut::from_bytes(&mut []),
        Err(Error::BufferTooShort {
            required: 20,
            actual: 0
        })
    ));
    let mut bytes = [0xa5; 2];
    let mut view = BigMut::from_bytes(&mut bytes).unwrap();
    assert_eq!(
        view.set_flags(8),
        Err(Error::ValueOutOfRange { bits: 3, value: 8 })
    );
    assert_eq!(
        view.set_fragment(8192),
        Err(Error::ValueOutOfRange {
            bits: 13,
            value: 8192
        })
    );
    assert_eq!(view.as_bytes(), &[0xa5; 2]);
    let mut bytes = [0x40];
    let mut view = TypedMut::from_bytes(&mut bytes).unwrap();
    assert_eq!(view.mode_raw(), 1);
    assert_eq!(view.mode(), Err(Error::InvalidValue { value: 1 }));
    assert_eq!(
        view.set_mode(Mode::TooWide),
        Err(Error::ValueOutOfRange { bits: 2, value: 4 })
    );
    assert_eq!(
        view.set_flags(Flags::from_bits_retain(8)),
        Err(Error::ValueOutOfRange { bits: 3, value: 8 })
    );
    assert_eq!(view.as_bytes(), &[0x40]);
}

#[test]
fn enums_flags_and_bool_round_trip() {
    let mut bytes = [0];
    let mut view = TypedMut::from_bytes(&mut bytes).unwrap();
    view.set_mode(Mode::B).unwrap();
    view.set_flags(Flags::A | Flags::B).unwrap();
    view.set_enabled(true).unwrap();
    assert_eq!(view.mode().unwrap(), Mode::B);
    assert_eq!(view.flags().unwrap(), Flags::A | Flags::B);
    assert!(view.enabled());
    view.set_flags(Flags::from_bits_retain(7)).unwrap();
    assert_eq!(view.flags().unwrap().bits(), 7);
    assert_eq!(view.mode_raw(), 2);
    assert_eq!(view.reserved(), 0);
    for mode in [Mode::A, Mode::B, Mode::TooWide] {
        assert_eq!(Mode::from_bits(mode.to_bits()).unwrap(), mode);
    }
}

fn check_packed(
    byte_len: usize,
    start: usize,
    width: usize,
    msb: bool,
    get: impl Fn(&[u8]) -> u128,
    set: impl Fn(&mut [u8], u128),
) {
    let mut state = 0x9e37_79b9_7f4a_7c15_u64 ^ (start * 131 + width) as u64;
    let mut random = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        u128::from(state) << 64 | u128::from(state.rotate_left(29))
    };
    let field_mask = u128::MAX >> (128 - width);
    for round in 0..500 {
        let mut bytes = [0u8; 24];
        let fill = match round {
            0 => 0,
            1 => u128::MAX,
            _ => random(),
        };
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = (fill.rotate_left(i as u32 * 8) >> 120) as u8;
        }
        let value = match round {
            0 => field_mask,
            1 => 0,
            _ => random() & field_mask,
        };
        assert_eq!(get(&bytes), reference_read(&bytes, start, width, msb));
        let mut expected = bytes;
        reference_write(&mut expected[..byte_len], start, width, value, msb);
        set(&mut bytes, value);
        assert_eq!(bytes, expected, "start {start}, width {width}, msb {msb}");
        assert_eq!(get(&bytes), value);
    }
}

macro_rules! packed_case {
    ($order:ident, $msb:literal, $start:literal, $width:literal, { $($field:ident: $ty:ident),* }) => {{
        bitstruct! {
            #[bitstruct(order = $order, endian = little)]
            struct Case { $($field: $ty),* }
        }
        check_packed(
            Case::BYTE_LEN,
            $start,
            $width,
            $msb,
            |bytes| u128::from(Case::from_bytes(bytes).unwrap().x()),
            |bytes, value| CaseMut::from_bytes(bytes).unwrap().set_x(value.try_into().unwrap()).unwrap(),
        );
    }};
}

macro_rules! packed_cases {
    ($order:ident, $msb:literal) => {
        // One byte.
        packed_case!($order, $msb, 4, 4, { pre: u4, x: u4 });
        // Two bytes, exact window.
        packed_case!($order, $msb, 1, 13, { pre: u1, x: u13, post: u2 });
        // Three bytes: zero-padded, exact, slid back, and extended forward.
        packed_case!($order, $msb, 3, 20, { pre: u3, x: u20, post: u1 });
        packed_case!($order, $msb, 3, 20, { pre: u3, x: u20, post: u9 });
        packed_case!($order, $msb, 11, 20, { pre: u11, x: u20, post: u1 });
        packed_case!($order, $msb, 11, 20, { pre: u11, x: u20, post: u9 });
        packed_case!($order, $msb, 0, 24, { x: u24 });
        packed_case!($order, $msb, 0, 24, { x: u24, post: u8 });
        // Six bytes padded to eight, and a full eight-byte window.
        packed_case!($order, $msb, 5, 37, { pre: u5, x: u37, post: u6 });
        packed_case!($order, $msb, 5, 59, { pre: u5, x: u59 });
        // Split across windows.
        packed_case!($order, $msb, 7, 64, { pre: u7, x: u64, post: u1 });
        packed_case!($order, $msb, 3, 100, { pre: u3, x: u100, post: u1 });
        packed_case!($order, $msb, 1, 128, { pre: u1, x: u128, post: u7 });
    };
}

#[test]
fn packed_windows_match_reference() {
    packed_cases!(msb0, true);
    packed_cases!(lsb0, false);
}

bitstruct! {
    /// IPv4 flags and fragment offset; the first flag bit is reserved.
    #[bitstruct(order = msb0, endian = big)]
    struct PaddedFragment {
        _: u1,
        dont_fragment: u1,
        more_fragments: u1,
        offset: u13,
        _: u8,
        ttl: u8,
    }

    #[bitstruct(order = msb0, endian = little)]
    struct PaddedWord {
        #[word(u16)]
        _: u4,
        value: u12,
    }

    #[bitstruct(order = lsb0, endian = little)]
    struct PaddedLsb {
        _: u3,
        value: u10,
        _: u3,
    }
}

#[test]
fn padding_fields_reserve_bits_without_accessors() {
    assert_eq!(PaddedFragment::BYTE_LEN, 4);
    let mut bytes = [0xff; 4];
    let mut view = PaddedFragmentMut::from_bytes(&mut bytes).unwrap();
    assert_eq!(view.offset(), 0x1fff);
    assert_eq!(view.ttl(), 0xff);
    view.set_dont_fragment(0).unwrap();
    view.set_more_fragments(0).unwrap();
    view.set_offset(0x0123).unwrap();
    view.set_ttl(64).unwrap();
    // Reserved bit 0 and byte 2 keep their contents.
    assert_eq!(bytes, [0x81, 0x23, 0xff, 64]);

    let mut bytes = [0xa5, 0x5a];
    let mut view = PaddedWordMut::from_bytes(&mut bytes).unwrap();
    assert_eq!(view.value(), 0xaa5);
    view.set_value(0x123).unwrap();
    assert_eq!(u16::from_le_bytes(bytes), 0x5123);

    for initial in [0u16, 0xffff, 0xa5a5] {
        let mut bytes = initial.to_le_bytes();
        let mut view = PaddedLsbMut::from_bytes(&mut bytes).unwrap();
        assert_eq!(view.value(), (initial >> 3) & 0x3ff);
        view.set_value(0x2aa).unwrap();
        assert_eq!(u16::from_le_bytes(bytes), (initial & 0xe007) | (0x2aa << 3));
    }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    struct Bools {
        b0: bool,
        b1: bool,
        b2: bool,
        b3: bool,
        b4: bool,
        b5: bool,
        b6: bool,
        b7: bool,
    }

    #[bitstruct(order = lsb0, endian = little)]
    struct LsbBools {
        first: bool,
        // Redundant but allowed.
        #[bits(1)]
        second: bool,
        _: u5,
        last: bool,
        #[word(u16)]
        low: bool,
        rest: u15,
    }
}

type BoolSetter = fn(&mut BoolsMut<'_>, bool) -> Result<(), Error>;

#[test]
fn bool_fields_are_single_bits() {
    let getters: [fn(&Bools<'_>) -> bool; 8] = [
        |view| view.b0(),
        |view| view.b1(),
        |view| view.b2(),
        |view| view.b3(),
        |view| view.b4(),
        |view| view.b5(),
        |view| view.b6(),
        |view| view.b7(),
    ];
    let setters: [BoolSetter; 8] = [
        |view, value| view.set_b0(value),
        |view, value| view.set_b1(value),
        |view, value| view.set_b2(value),
        |view, value| view.set_b3(value),
        |view, value| view.set_b4(value),
        |view, value| view.set_b5(value),
        |view, value| view.set_b6(value),
        |view, value| view.set_b7(value),
    ];
    for initial in 0..=u8::MAX {
        for (i, (get, set)) in getters.iter().zip(setters).enumerate() {
            let mask = 0x80 >> i;
            assert_eq!(
                get(&Bools::from_bytes(&[initial]).unwrap()),
                initial & mask != 0
            );
            for value in [false, true] {
                let mut bytes = [initial];
                set(&mut BoolsMut::from_bytes(&mut bytes).unwrap(), value).unwrap();
                let expected = if value {
                    initial | mask
                } else {
                    initial & !mask
                };
                assert_eq!(bytes, [expected]);
            }
        }
    }

    let mut bytes = [0b1000_0001, 0x01, 0x80];
    let mut view = LsbBoolsMut::from_bytes(&mut bytes).unwrap();
    assert!(view.first());
    assert!(!view.second());
    assert!(view.last());
    assert!(view.low());
    assert_eq!(view.rest(), 0x4000);
    view.set_first(false).unwrap();
    view.set_second(true).unwrap();
    view.set_low(false).unwrap();
    assert_eq!(bytes, [0b1000_0010, 0x00, 0x80]);
}
