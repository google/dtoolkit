// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::fmt::{self, Display, Formatter};
use core::ops::Deref;

use crate::error::{PropertyError, StandardError};
use crate::fdt::{Fdt, FdtNode};
use crate::standard::Reg;
use crate::{Node, Property, ToCellInt};

impl<'a> Fdt<'a> {
    /// Returns the `/memory` node.
    ///
    /// This should always be included in a valid device tree.
    ///
    /// # Errors
    ///
    /// Returns [`StandardError::MemoryMissing`] if the memory node is missing.
    pub fn memory(self) -> Result<Memory<FdtNode<'a>>, StandardError> {
        let node = self
            .find_node("/memory")
            .ok_or(StandardError::MemoryMissing)?;
        Ok(Memory { node })
    }

    /// Returns the `/reserved-memory/*` nodes, if any.
    #[must_use]
    pub fn reserved_memory(self) -> Option<impl Iterator<Item = ReservedMemory<FdtNode<'a>>>> {
        Some(
            self.find_node("/reserved-memory")?
                .children()
                .map(|node| ReservedMemory { node }),
        )
    }
}

/// Typed wrapper for a `/memory` node.
#[derive(Debug, Clone, Copy)]
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

impl<N: Node> Memory<N> {
    /// Returns the value of the standard `initial-mapped-area` property of the
    /// memory node.
    ///
    /// # Errors
    ///
    /// Returns an error if the size of the value isn't a multiple of 5 cells.
    pub fn initial_mapped_area<'a>(
        &'a self,
    ) -> Result<Option<impl Iterator<Item = InitialMappedArea> + 'a>, PropertyError>
    where
        N: 'a,
    {
        if let Some(property) = self.node.property("initial-mapped-area") {
            Ok(Some(
                property
                    .as_prop_encoded_array([2, 2, 1])?
                    .map(InitialMappedArea::from_cells),
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
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
    fn from_cells<C: ToCellInt>([ea, pa, size]: [C; 3]) -> Self {
        Self {
            effective_address: ea.to_int().unwrap(),
            physical_address: pa.to_int().unwrap(),
            size: size.to_int().unwrap(),
        }
    }
}

/// Typed wrapper for a `/reserved-memory/*` node.
#[derive(Debug, Clone, Copy)]
pub struct ReservedMemory<N> {
    node: N,
}

impl<N> Deref for ReservedMemory<N> {
    type Target = N;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl<N: Display> Display for ReservedMemory<N> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.node.fmt(f)
    }
}

impl<N: Node> ReservedMemory<N> {
    /// Returns the value of the standard `size` property of the reserved memory
    /// node, if it is present.
    ///
    /// # Errors
    ///
    /// Returns an error if the value of the property isn't a multiple of 4
    /// bytes long.
    pub fn size<'a>(&'a self) -> Result<Option<crate::Cells<'a>>, PropertyError>
    where
        N: 'a,
    {
        self.node
            .property("size")
            .map(|value| value.value_as::<crate::Cells<'a>>())
            .transpose()
    }

    /// Returns the value of the standard `alignment` property of the reserved
    /// memory node, if it is present.
    ///
    /// # Errors
    ///
    /// Returns an error if the value of the property isn't a multiple of 4
    /// bytes long.
    pub fn alignment<'a>(&'a self) -> Result<Option<crate::Cells<'a>>, PropertyError>
    where
        N: 'a,
    {
        self.node
            .property("alignment")
            .map(|value| value.value_as::<crate::Cells<'a>>())
            .transpose()
    }

    /// Returns whether the standard `no-map` property is present.
    pub fn no_map(&self) -> bool {
        self.node.property("no-map").is_some()
    }

    /// Returns whether the standard `no-map-fixup` property is present.
    pub fn no_map_fixup(&self) -> bool {
        self.node.property("no-map-fixup").is_some()
    }

    /// Returns whether the standard `reusable` property is present.
    pub fn reusable(&self) -> bool {
        self.node.property("reusable").is_some()
    }
}

impl<'a> ReservedMemory<FdtNode<'a>> {
    /// Returns the value of the standard `alloc-ranges` property.
    ///
    /// # Errors
    ///
    /// Returns an error if the size of the value isn't a multiple of the
    /// expected number of address and size cells.
    pub fn alloc_ranges(self) -> Result<Option<impl Iterator<Item = Reg<'a>> + 'a>, StandardError> {
        let address_cells = self.node.parent_address_space.address_cells as usize;
        let size_cells = self.node.parent_address_space.size_cells as usize;
        if let Some(property) = self.node.property("alloc-ranges") {
            Ok(Some(
                property
                    .as_prop_encoded_array([address_cells, size_cells])?
                    .map(Reg::from_cells),
            ))
        } else {
            Ok(None)
        }
    }
}
