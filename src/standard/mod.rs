// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Standard nodes and properties.

mod cpus;
mod memory;
mod ranges;
mod reg;
mod status;

pub use self::cpus::{Cpu, Cpus};
pub use self::memory::{InitialMappedArea, Memory};
pub use self::ranges::Range;
pub use self::reg::Reg;
pub use self::status::Status;
use crate::error::StandardError;
use crate::fdt::FdtNode;
use crate::{Node, Property};

pub(crate) const DEFAULT_ADDRESS_CELLS: u32 = 2;
pub(crate) const DEFAULT_SIZE_CELLS: u32 = 1;

impl<'a> FdtNode<'a> {
    /// Returns the value of the standard `reg` property.
    ///
    /// # Errors
    ///
    /// Returns an error if the size of the value isn't a multiple of the
    /// expected number of address and size cells.
    pub fn reg(&self) -> Result<Option<impl Iterator<Item = Reg<'a>> + use<'a>>, StandardError> {
        let address_cells = self.parent_address_space.address_cells as usize;
        let size_cells = self.parent_address_space.size_cells as usize;
        if let Some(property) = self.property("reg") {
            Ok(Some(
                property
                    .as_prop_encoded_array([address_cells, size_cells])?
                    .map(Reg::from_cells),
            ))
        } else {
            Ok(None)
        }
    }

    /// Returns the value of the standard `ranges` property.
    ///
    /// # Errors
    ///
    /// Returns an error if the size of the value isn't a multiple of the
    /// expected number of cells.
    pub fn ranges(
        &self,
    ) -> Result<Option<impl Iterator<Item = Range<'a>> + use<'a>>, StandardError> {
        if let Some(property) = self.property("ranges") {
            Ok(Some(
                property
                    .as_prop_encoded_array([
                        self.address_cells().unwrap_or(DEFAULT_ADDRESS_CELLS) as usize,
                        self.parent_address_space.address_cells as usize,
                        self.size_cells().unwrap_or(DEFAULT_SIZE_CELLS) as usize,
                    ])?
                    .map(Range::from_cells),
            ))
        } else {
            Ok(None)
        }
    }

    /// Returns the value of the standard `dma-ranges` property.
    ///
    /// # Errors
    ///
    /// Returns an error if the size of the value isn't a multiple of the
    /// expected number of cells.
    pub fn dma_ranges(
        &self,
    ) -> Result<Option<impl Iterator<Item = Range<'a>> + use<'a>>, StandardError> {
        if let Some(property) = self.property("dma-ranges") {
            Ok(Some(
                property
                    .as_prop_encoded_array([
                        self.address_cells().unwrap_or(DEFAULT_ADDRESS_CELLS) as usize,
                        self.parent_address_space.address_cells as usize,
                        self.size_cells().unwrap_or(DEFAULT_SIZE_CELLS) as usize,
                    ])?
                    .map(Range::from_cells),
            ))
        } else {
            Ok(None)
        }
    }
}

/// The `#address-cells` and `#size-cells` properties of a node.
#[derive(Debug, Clone, Copy)]
pub struct AddressSpaceProperties {
    /// The `#address-cells` property.
    pub address_cells: u32,
    /// The `#size-cells` property.
    pub size_cells: u32,
}

impl Default for AddressSpaceProperties {
    fn default() -> Self {
        Self {
            address_cells: DEFAULT_ADDRESS_CELLS,
            size_cells: DEFAULT_SIZE_CELLS,
        }
    }
}
