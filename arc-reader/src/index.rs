use std::ops::Range;

pub const INVALID_INDEX: u32 = 0x00FF_FFFF;

pub fn checked_range(start: u32, count: u32) -> Range<u32> {
    if (start + count) >= INVALID_INDEX {
        0..0
    } else {
        start..start + count
    }
}
