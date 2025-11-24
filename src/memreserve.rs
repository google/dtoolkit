// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Device tree memory reservations.

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, big_endian};

/// A 64-bit memory reservation.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    FromBytes,
    IntoBytes,
    Immutable,
    KnownLayout,
)]
#[repr(C)]
pub struct MemoryReservation {
    address: big_endian::U64,
    size: big_endian::U64,
}

impl MemoryReservation {
    pub(crate) const TERMINATOR: Self = Self::new(0, 0);

    /// Creates a new [`MemoryReservation`].
    #[must_use]
    pub const fn new(address: u64, size: u64) -> Self {
        Self {
            address: big_endian::U64::new(address),
            size: big_endian::U64::new(size),
        }
    }

    /// Returns the physical address of the reserved memory region.
    #[must_use]
    pub const fn address(&self) -> u64 {
        self.address.get()
    }

    /// Returns the size of the reserved memory region.
    #[must_use]
    pub const fn size(&self) -> u64 {
        self.size.get()
    }
}
