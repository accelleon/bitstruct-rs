use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use bitstruct_rs::bitstruct;

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if ACTIVE.try_with(Cell::get).unwrap_or(false) {
            let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
        }
        // Forward the allocator contract unchanged to System.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // The pointer and layout originate from System.alloc.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if ACTIVE.try_with(Cell::get).unwrap_or(false) {
            let _ = ALLOCATIONS.try_with(|n| n.set(n.get() + 1));
        }
        // Forward the allocator contract unchanged to System.
        unsafe { System.realloc(ptr, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

#[bitstruct(order = msb0, endian = little)]
struct Header {
    dscp: u6,
    ecn: u2,
    length: u16,
    reserved: u8,
    #[word(u16)]
    flags: u3,
    fragment: u13,
}

#[test]
fn borrowing_and_access_allocate_nothing() {
    let mut bytes = [0; Header::BYTE_LEN];
    let ptr = bytes.as_ptr();
    ALLOCATIONS.with(|n| n.set(0));
    ACTIVE.with(|n| n.set(true));
    let mut view = HeaderMut::from_bytes(&mut bytes).unwrap();
    for i in 0..4096u16 {
        view.set_length(i).unwrap();
        view.set_fragment(i).unwrap();
        view.set_flags(2).unwrap();
        view.set_dscp((i % 64) as u8).unwrap();
        std::hint::black_box(view.as_view().length());
        std::hint::black_box(view.fragment());
    }
    assert_eq!(view.as_bytes().as_ptr(), ptr);
    ACTIVE.with(|n| n.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    assert_eq!(bytes[1..3], 4095u16.to_le_bytes());
}
