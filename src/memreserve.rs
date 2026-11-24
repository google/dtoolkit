// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Device tree memory reservations.

use zerocopy::big_endian;
// Glob import to avoid duplicate import errors if the `derive` feature is
// enabled on `zerocopy`. See https://github.com/google/zerocopy/issues/1587.
#[allow(clippy::wildcard_imports)]
use zerocopy_derive::*;

/// A 64-bit memory reservation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MemoryReservation {
    address: u64,
    size: u64,
}

impl MemoryReservation {
    /// Creates a new [`MemoryReservation`].
    #[must_use]
    pub const fn new(address: u64, size: u64) -> Self {
        Self { address, size }
    }

    /// Returns the physical address of the reserved memory region.
    #[must_use]
    pub const fn address(&self) -> u64 {
        self.address
    }

    /// Returns the size of the reserved memory region.
    #[must_use]
    pub const fn size(&self) -> u64 {
        self.size
    }

    pub(crate) const fn from_raw(raw: RawMemoryReservation) -> Self {
        Self::new(raw.address.get(), raw.size.get())
    }

    #[cfg(feature = "write")]
    pub(crate) const fn to_raw(self) -> RawMemoryReservation {
        RawMemoryReservation::new(self.address, self.size)
    }
}

/// The on-disk (big-endian) representation of a [`MemoryReservation`].
///
/// This is deliberately private to the crate so that `zerocopy` traits do not
/// become part of the public API.
#[derive(Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C)]
pub(crate) struct RawMemoryReservation {
    address: big_endian::U64,
    size: big_endian::U64,
}

impl RawMemoryReservation {
    pub(crate) const TERMINATOR: Self = Self::new(0, 0);

    pub(crate) const fn new(address: u64, size: u64) -> Self {
        Self {
            address: big_endian::U64::new(address),
            size: big_endian::U64::new(size),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_reservation_new() {
        let res = MemoryReservation::new(0x1000, 0x100);

        assert_eq!(res.address(), 0x1000);
        assert_eq!(res.size(), 0x100);
    }
}
