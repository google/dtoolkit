// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::fmt::{self, Debug, Display, Formatter};
use core::ops::{BitOr, Shl};

use crate::error::StandardError;
use crate::fdt::Cells;

/// The value of a `reg` property.
#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub struct Reg<'a> {
    /// The address of the device within the address space of the parent bus.
    pub address: Cells<'a>,
    /// The size of the device within the address space of the parent bus.
    pub size: Cells<'a>,
}

impl<'a> Reg<'a> {
    pub(crate) fn from_cells([address, size]: [Cells<'a>; 2]) -> Self {
        Self { address, size }
    }

    /// Attempts to return the address as the given type, if it will fit.
    ///
    /// # Errors
    ///
    /// Returns `FdtError::TooManyCells` if the address doesn't fit in `T`.
    pub fn address<T: Default + From<u32> + Shl<usize, Output = T> + BitOr<Output = T>>(
        self,
    ) -> Result<T, StandardError> {
        self.address.to_int()
    }

    /// Attempts to return the size as the given type, if it will fit.
    ///
    /// # Errors
    ///
    /// Returns `FdtError::TooManyCells` if the size doesn't fit in `T`.
    pub fn size<T: Default + From<u32> + Shl<usize, Output = T> + BitOr<Output = T>>(
        self,
    ) -> Result<T, StandardError> {
        self.size.to_int()
    }
}

impl Display for Reg<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} {}", self.address, self.size)
    }
}

impl Debug for Reg<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "Reg {{ address: {}, size: {} }}",
            self.address, self.size
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_reg() {
        let address = [0x123_45678.into(), 0xabcd_0000.into()];
        let size = [0x1122_3344.into()];
        let reg = Reg {
            address: Cells(&address),
            size: Cells(&size),
        };
        assert_eq!(reg.to_string(), "0x12345678abcd0000 0x11223344");
    }

    #[test]
    fn address_size() {
        let address = [0x123_45678.into(), 0xabcd_0000.into()];
        let size = [0x1122_3344.into()];
        let reg = Reg {
            address: Cells(&address),
            size: Cells(&size),
        };
        assert_eq!(
            reg.address::<u32>(),
            Err(StandardError::TooManyCells { cells: 2 })
        );
        assert_eq!(reg.address::<u64>(), Ok(0x1234_5678_abcd_0000));
        assert_eq!(reg.size::<u32>(), Ok(0x1122_3344));
        assert_eq!(reg.size::<u64>(), Ok(0x1122_3344));
    }
}
