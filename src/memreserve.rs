// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Device tree memory reservations.

/// A 64-bit memory reservation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemoryReservation {
    address: u64,
    size: u64,
}

impl MemoryReservation {
    /// Creates a new [`MemoryReservation`].
    #[must_use]
    pub fn new(address: u64, size: u64) -> Self {
        Self { address, size }
    }

    /// Returns the physical address of the reserved memory region.
    #[must_use]
    pub fn address(&self) -> u64 {
        self.address
    }

    /// Returns the size of the reserved memory region.
    #[must_use]
    pub fn size(&self) -> u64 {
        self.size
    }
}
