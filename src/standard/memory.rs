// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::fmt::{self, Display, Formatter};
use core::ops::Deref;

use crate::error::StandardError;
use crate::fdt::{Fdt, FdtNode};
use crate::{Cells, Node, Property};

impl<'a> Fdt<'a> {
    /// Returns the `/memory` node.
    ///
    /// This should always be included in a valid device tree.
    ///
    /// # Errors
    ///
    /// Returns a parse error if there was a problem reading the FDT structure
    /// to find the node, or `FdtError::MemoryMissing` if the memory node is
    /// missing.
    pub fn memory(self) -> Result<Memory<FdtNode<'a>>, StandardError> {
        let node = self
            .find_node("/memory")
            .ok_or(StandardError::MemoryMissing)?;
        Ok(Memory { node })
    }
}

/// Typed wrapper for a `/memory` node.
#[derive(Clone, Copy, Debug)]
pub struct Memory<N> {
    node: N,
}

impl<N> Deref for Memory<N> {
    type Target = N;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl<N: Display> Display for Memory<N> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.node.fmt(f)
    }
}

impl<'a, N: Node<'a>> Memory<N> {
    /// Returns the value of the standard `initial-mapped-area` property of the
    /// memory node.
    ///
    /// # Errors
    ///
    /// Returns an error if a property's name or value cannot be read, or the
    /// size of the value isn't a multiple of 5 cells.
    pub fn initial_mapped_area(
        &self,
    ) -> Result<Option<impl Iterator<Item = InitialMappedArea> + use<'a, N>>, StandardError> {
        if let Some(property) = self.node.property("initial-mapped-area") {
            Ok(Some(
                property
                    .as_prop_encoded_array([2, 2, 1])?
                    .map(|chunk| InitialMappedArea::from_cells(chunk)),
            ))
        } else {
            Ok(None)
        }
    }

    /// Returns the value of the standard `hotpluggable` property of the memory
    /// node.
    #[must_use]
    pub fn hotpluggable(&self) -> bool {
        self.node.property("hotpluggable").is_some()
    }
}

/// The value of an `initial-mapped-area` property.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct InitialMappedArea {
    /// The effective address.
    pub effective_address: u64,
    /// The physical address.
    pub physical_address: u64,
    /// The size of the area.
    pub size: u32,
}

impl InitialMappedArea {
    /// Creates an `InitialMappedArea` from an array of three `Cells` containing
    /// the effective address, physical address and size respectively.
    ///
    /// These `Cells` must contain 2, 2 and 1 cells respectively, or the method
    /// will panic.
    #[expect(
        clippy::unwrap_used,
        reason = "The Cells passed are always the correct size"
    )]
    fn from_cells([ea, pa, size]: [Cells; 3]) -> Self {
        Self {
            effective_address: ea.to_int().unwrap(),
            physical_address: pa.to_int().unwrap(),
            size: size.to_int().unwrap(),
        }
    }
}
