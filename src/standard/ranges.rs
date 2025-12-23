// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::fmt::{self, Display, Formatter};
use core::ops::{BitOr, Shl};

use crate::error::StandardError;
use crate::fdt::Cells;

/// One of the values of a `ranges` property.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Range<'a> {
    /// The address in address space of the child bus.
    pub child_bus_address: Cells<'a>,
    /// The address in the address space of the parent bus.
    pub parent_bus_address: Cells<'a>,
    /// The size of the range in the child's address space.
    pub length: Cells<'a>,
}

impl<'a> Range<'a> {
    pub(crate) fn from_cells(
        [child_bus_address, parent_bus_address, length]: [Cells<'a>; 3],
    ) -> Self {
        Self {
            child_bus_address,
            parent_bus_address,
            length,
        }
    }

    /// Attempts to return the child-bus-address as the given type, if it will
    /// fit.
    ///
    /// # Errors
    ///
    /// Returns `FdtError::TooManyCells` if the child-bus-address doesn't fit in
    /// `T`.
    pub fn child_bus_address<
        T: Default + From<u32> + Shl<usize, Output = T> + BitOr<Output = T>,
    >(
        &self,
    ) -> Result<T, StandardError> {
        self.child_bus_address.to_int()
    }

    /// Attempts to return the parent-bus-address as the given type, if it will
    /// fit.
    ///
    /// # Errors
    ///
    /// Returns `FdtError::TooManyCells` if the parent-bus-address doesn't fit
    /// in `T`.
    pub fn parent_bus_address<
        T: Default + From<u32> + Shl<usize, Output = T> + BitOr<Output = T>,
    >(
        &self,
    ) -> Result<T, StandardError> {
        self.parent_bus_address.to_int()
    }

    /// Attempts to return the length as the given type, if it will fit.
    ///
    /// # Errors
    ///
    /// Returns `FdtError::TooManyCells` if the length doesn't fit in `T`.
    pub fn length<T: Default + From<u32> + Shl<usize, Output = T> + BitOr<Output = T>>(
        &self,
    ) -> Result<T, StandardError> {
        self.length.to_int()
    }
}

impl Display for Range<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.child_bus_address, self.parent_bus_address, self.length
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_range() {
        let child_bus_address = [0x0.into(), 0x4000.into()];
        let parent_bus_address = [0xe000_0000.into()];
        let length = [0x10_0000.into()];
        let range = Range {
            child_bus_address: Cells(&child_bus_address),
            parent_bus_address: Cells(&parent_bus_address),
            length: Cells(&length),
        };
        assert_eq!(
            range.to_string(),
            "0x0000000000004000 0xe0000000 0x00100000"
        );
    }
}
