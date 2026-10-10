use std::hint::black_box;
use std::time::{Duration, Instant};

use bitstruct_rs::bitstruct;

bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    pub struct Header {
        dscp: u6,
        ecn: u2,
        length: u16,
        reserved: u8,
        #[word(u16)]
        flags: u3,
        fragment: u13,
    }
}

bitstruct! {
    #[bitstruct(order = msb0, endian = little)]
    pub struct Little {
        #[word(u16)]
        flags: u3,
        fragment: u13,
    }
}

// Cross-byte fields without `#[word]`: `fragment` spans 2 bytes, `wide` spans 3.
bitstruct! {
    #[bitstruct(order = msb0, endian = big)]
    pub struct Flat {
        flags: u3,
        fragment: u13,
        pad: u3,
        wide: u20,
        tail: u1,
    }
}

bitstruct! {
    #[bitstruct(order = lsb0, endian = little)]
    pub struct LsbFlat {
        low: u3,
        value: u13,
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn generated_packed_set(bytes: &mut [u8], value: u16) {
    FlatMut::from_bytes(bytes)
        .unwrap()
        .set_fragment(value & 0x1fff)
        .unwrap();
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn generated_packed_get(bytes: &[u8]) -> u32 {
    Flat::from_bytes(bytes).unwrap().wide()
}

// Kept out of line to make representative optimized code easy to inspect.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn generated_word_set(bytes: &mut [u8], value: u16) {
    HeaderMut::from_bytes(bytes)
        .unwrap()
        .set_fragment(value & 0x1fff)
        .unwrap();
}

#[unsafe(no_mangle)]
#[inline(never)]
pub fn handwritten_word_set(bytes: &mut [u8], value: u16) {
    let old = u16::from_be_bytes([bytes[4], bytes[5]]);
    let word = (old & 0xe000) | (value & 0x1fff);
    bytes[4..6].copy_from_slice(&word.to_be_bytes());
}

fn measure(mut operation: impl FnMut(u16)) -> Duration {
    let start = Instant::now();
    for i in 0..2_000_000u32 {
        operation(black_box(i as u16));
    }
    start.elapsed()
}

fn compare(label: &str, mut generated: impl FnMut(u16), mut manual: impl FnMut(u16)) {
    // Warm up and alternate execution order; report the median of seven samples.
    for i in 0..10_000 {
        generated(black_box(i));
        manual(black_box(i));
    }
    let mut g = Vec::new();
    let mut m = Vec::new();
    for i in 0..7 {
        if i % 2 == 0 {
            g.push(measure(&mut generated));
            m.push(measure(&mut manual));
        } else {
            m.push(measure(&mut manual));
            g.push(measure(&mut generated));
        }
    }
    g.sort();
    m.sort();
    println!(
        "{label}: generated {:?}, handwritten {:?}, ratio {:.3}",
        g[3],
        m[3],
        g[3].as_secs_f64() / m[3].as_secs_f64()
    );
}

fn main() {
    let mut generated = [0; 6];
    let mut manual = [0; 6];
    let mut header = HeaderMut::from_bytes(&mut generated).unwrap();
    compare(
        "DSCP set/get",
        |value| {
            let view = black_box(&mut header);
            view.set_dscp((value & 63) as u8).unwrap();
            black_box(view.dscp());
        },
        |value| {
            let bytes = black_box(&mut manual);
            bytes[0] = (bytes[0] & 3) | (((value & 63) as u8) << 2);
            black_box(bytes[0] >> 2);
        },
    );
    compare(
        "aligned u16 set/get",
        |value| {
            let view = black_box(&mut header);
            view.set_length(value).unwrap();
            black_box(view.length());
        },
        |value| {
            let bytes = black_box(&mut manual);
            bytes[1..3].copy_from_slice(&value.to_be_bytes());
            black_box(u16::from_be_bytes([bytes[1], bytes[2]]));
        },
    );
    compare(
        "big-endian u13 set/get",
        |value| {
            let view = black_box(&mut header);
            view.set_fragment(value & 0x1fff).unwrap();
            black_box(view.fragment());
        },
        |value| {
            let bytes = black_box(&mut manual);
            let word = u16::from_be_bytes([bytes[4], bytes[5]]);
            bytes[4..6].copy_from_slice(&((word & 0xe000) | (value & 0x1fff)).to_be_bytes());
            black_box(u16::from_be_bytes([bytes[4], bytes[5]]) & 0x1fff);
        },
    );
    let mut bytes = [0; 2];
    let mut little_manual = [0; 2];
    let mut little = LittleMut::from_bytes(&mut bytes).unwrap();
    compare(
        "little-endian u13 set/get",
        |value| {
            let view = black_box(&mut little);
            view.set_fragment(value & 0x1fff).unwrap();
            black_box(view.fragment());
        },
        |value| {
            let bytes = black_box(&mut little_manual);
            let word = u16::from_le_bytes(*bytes);
            *bytes = ((word & 0xe000) | (value & 0x1fff)).to_le_bytes();
            black_box(u16::from_le_bytes(*bytes) & 0x1fff);
        },
    );
    let mut flat_bytes = [0; Flat::BYTE_LEN];
    let mut flat_manual = [0; Flat::BYTE_LEN];
    let mut flat = FlatMut::from_bytes(&mut flat_bytes).unwrap();
    compare(
        "packed msb0 u13 set/get",
        |value| {
            let view = black_box(&mut flat);
            view.set_fragment(value & 0x1fff).unwrap();
            black_box(view.fragment());
        },
        |value| {
            let bytes = black_box(&mut flat_manual);
            let word = u16::from_be_bytes([bytes[0], bytes[1]]);
            bytes[0..2].copy_from_slice(&((word & 0xe000) | (value & 0x1fff)).to_be_bytes());
            black_box(u16::from_be_bytes([bytes[0], bytes[1]]) & 0x1fff);
        },
    );
    compare(
        "packed msb0 u20 (3 bytes) set/get",
        |value| {
            let view = black_box(&mut flat);
            view.set_wide(u32::from(value) << 4).unwrap();
            black_box(view.wide());
        },
        |value| {
            let bytes = black_box(&mut flat_manual);
            let window = u32::from_be_bytes([bytes[2], bytes[3], bytes[4], 0]);
            let keep = !(0xf_ffff << 9);
            let window = (window & keep) | ((u32::from(value) << 4) << 9);
            bytes[2..5].copy_from_slice(&window.to_be_bytes()[..3]);
            black_box((u32::from_be_bytes([bytes[2], bytes[3], bytes[4], 0]) >> 9) & 0xf_ffff);
        },
    );
    let mut lsb_bytes = [0; 2];
    let mut lsb_manual = [0; 2];
    let mut lsb = LsbFlatMut::from_bytes(&mut lsb_bytes).unwrap();
    compare(
        "packed lsb0 u13 set/get",
        |value| {
            let view = black_box(&mut lsb);
            view.set_value(value & 0x1fff).unwrap();
            black_box(view.value());
        },
        |value| {
            let bytes = black_box(&mut lsb_manual);
            let word = u16::from_le_bytes(*bytes);
            *bytes = ((word & 0x7) | ((value & 0x1fff) << 3)).to_le_bytes();
            black_box(u16::from_le_bytes(*bytes) >> 3);
        },
    );
    generated_word_set(black_box(&mut generated), black_box(42));
    handwritten_word_set(black_box(&mut manual), black_box(42));
    generated_packed_set(black_box(&mut flat_bytes), black_box(42));
    black_box(generated_packed_get(black_box(&flat_bytes)));
}
