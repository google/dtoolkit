// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Standard nodes and properties.

mod chosen;
mod cpus;
mod memory;
mod ranges;
mod reg;
mod status;

pub use self::chosen::Chosen;
pub use self::cpus::{Cpu, Cpus};
pub use self::memory::{InitialMappedArea, Memory, ReservedMemory};
pub use self::ranges::Range;
pub use self::reg::Reg;
pub use self::status::Status;
use crate::error::{PropertyError, StandardError};
use crate::fdt::FdtNode;
use crate::{Node, Property};

pub(crate) const DEFAULT_ADDRESS_CELLS: u32 = 2;
pub(crate) const DEFAULT_SIZE_CELLS: u32 = 1;

/// Methods to access standard properties on FDT nodes.
pub trait NodeStandard: Node {
    /// Returns the value of the standard `compatible` property.
    #[must_use]
    fn compatible(&self) -> Option<impl Iterator<Item = &'_ str> + '_> {
        self.property("compatible").and_then(|property| {
            property
                .value_as::<crate::values::FdtStringListIterator>()
                .ok()
        })
    }

    /// Returns whether this node has a `compatible` property containing the
    /// given string.
    #[must_use]
    fn is_compatible(&self, compatible_filter: &str) -> bool {
        if let Some(prop) = self.property("compatible") {
            return prop
                .value_as::<crate::values::FdtStringListIterator>()
                .is_ok_and(|mut it| it.any(|c| c == compatible_filter));
        }
        false
    }

    /// Finds all child nodes with a `compatible` property containing the given
    /// string.
    fn find_compatible<'f>(
        &'f self,
        compatible_filter: &'f str,
    ) -> impl Iterator<Item = Self::Child<'f>> + 'f {
        self.children()
            .filter(move |child| child.is_compatible(compatible_filter))
    }

    /// Returns the value of the standard `model` property.
    ///
    /// # Errors
    ///
    /// Returns an error if the value isn't a valid UTF-8 string.
    fn model(&self) -> Result<Option<&str>, PropertyError> {
        if let Some(model) = self.property("model") {
            Ok(Some(model.value_as::<&str>()?))
        } else {
            Ok(None)
        }
    }

    /// Returns the value of the standard `phandle` property.
    ///
    /// # Errors
    ///
    /// Returns an error if the value isn't a valid u32.
    fn phandle(&self) -> Result<Option<u32>, PropertyError> {
        if let Some(property) = self.property("phandle") {
            Ok(Some(property.value_as::<u32>()?))
        } else {
            Ok(None)
        }
    }

    /// Returns the value of the standard `status` property.
    ///
    /// If there is no `status` property then `okay` is assumed.
    ///
    /// # Errors
    ///
    /// Returns an error if the value isn't a valid status.
    fn status(&self) -> Result<Status, StandardError> {
        if let Some(status) = self.property("status") {
            Ok(status.value_as::<&str>()?.parse()?)
        } else {
            Ok(Status::Okay)
        }
    }

    /// Returns the value of the standard `#address-cells` property.
    ///
    /// # Errors
    ///
    /// Returns an error if the value isn't a valid u32.
    fn address_cells(&self) -> Result<u32, PropertyError> {
        if let Some(property) = self.property("#address-cells") {
            Ok(property.value_as::<u32>()?)
        } else {
            Ok(DEFAULT_ADDRESS_CELLS)
        }
    }

    /// Returns the value of the standard `#size-cells` property.
    ///
    /// # Errors
    ///
    /// Returns an error if the value isn't a valid u32.
    fn size_cells(&self) -> Result<u32, PropertyError> {
        if let Some(model) = self.property("#size-cells") {
            Ok(model.value_as::<u32>()?)
        } else {
            Ok(DEFAULT_SIZE_CELLS)
        }
    }

    /// Returns the values of the standard `#address-cells` and `#size_cells`
    /// properties.
    #[must_use]
    fn address_space(&self) -> AddressSpaceProperties {
        AddressSpaceProperties {
            address_cells: self.address_cells().unwrap_or(DEFAULT_ADDRESS_CELLS),
            size_cells: self.size_cells().unwrap_or(DEFAULT_SIZE_CELLS),
        }
    }

    /// Returns the value of the standard `virtual-reg` property.
    ///
    /// # Errors
    ///
    /// Returns an error if the value isn't a valid u32.
    fn virtual_reg(&self) -> Result<Option<u32>, PropertyError> {
        if let Some(property) = self.property("virtual-reg") {
            Ok(Some(property.value_as::<u32>()?))
        } else {
            Ok(None)
        }
    }

    /// Returns whether the standard `dma-coherent` property is present.
    #[must_use]
    fn dma_coherent(&self) -> bool {
        self.property("dma-coherent").is_some()
    }
}

impl<T: Node> NodeStandard for T {}

impl<'a> FdtNode<'a> {
    /// Returns the value of the standard `compatible` property.
    #[must_use]
    pub fn compatible(&self) -> Option<impl Iterator<Item = &'a str> + 'a> {
        self.property("compatible").and_then(|property| {
            property
                .value_as::<crate::values::FdtStringListIterator<'a>>()
                .ok()
        })
    }

    /// Returns the value of the standard `model` property.
    ///
    /// # Errors
    ///
    /// Returns an error if the value isn't a valid UTF-8 string.
    pub fn model(&self) -> Result<Option<&'a str>, PropertyError> {
        if let Some(model) = self.property("model") {
            Ok(Some(model.value_as::<&'a str>()?))
        } else {
            Ok(None)
        }
    }

    /// Returns the value of the standard `reg` property.
    ///
    /// # Errors
    ///
    /// Returns an error if the size of the value isn't a multiple of the
    /// expected number of address and size cells.
    pub fn reg(self) -> Result<Option<impl Iterator<Item = Reg<'a>> + 'a>, PropertyError> {
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
    pub fn ranges(self) -> Result<Option<impl Iterator<Item = Range<'a>> + 'a>, PropertyError> {
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
    pub fn dma_ranges(self) -> Result<Option<impl Iterator<Item = Range<'a>> + 'a>, PropertyError> {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
