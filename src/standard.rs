// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Standard nodes and properties.

mod memory;
mod status;

pub use self::memory::{InitialMappedArea, Memory};
pub use self::status::Status;
use crate::error::{FdtError, FdtParseError};
use crate::fdt::FdtNode;

const DEFAULT_ADDRESS_CELLS: u32 = 2;
const DEFAULT_SIZE_CELLS: u32 = 1;

impl<'a> FdtNode<'a> {
    /// Returns the value of the standard `compatible` property.
    ///
    /// # Errors
    ///
    /// Returns an error if a property's name or value cannot be read.
    pub fn compatible(
        &self,
    ) -> Result<Option<impl Iterator<Item = &'a str> + use<'a>>, FdtParseError> {
        Ok(self
            .property("compatible")?
            .map(|property| property.as_str_list()))
    }

    /// Returns whether this node has a `compatible` properties containing the
    /// given string.
    ///
    /// # Errors
    ///
    /// Returns an error if a property's name or value cannot be read.
    pub fn is_compatible(&self, compatible_filter: &str) -> Result<bool, FdtParseError> {
        Ok(if let Some(mut compatible) = self.compatible()? {
            compatible.any(|c| c == compatible_filter)
        } else {
            false
        })
    }

    /// Finds all child nodes with a `compatible` property containing the given
    /// string.
    pub fn find_compatible<'f>(
        &self,
        compatible_filter: &'f str,
    ) -> impl Iterator<Item = Result<FdtNode<'a>, FdtParseError>> + use<'a, 'f> {
        self.children().filter_map(|child| match child {
            Ok(child) => match child.is_compatible(compatible_filter) {
                Ok(true) => Some(Ok(child)),
                Ok(false) => None,
                Err(e) => Some(Err(e)),
            },
            Err(e) => Some(Err(e)),
        })
    }

    /// Returns the value of the standard `model` property.
    ///
    /// # Errors
    ///
    /// Returns an error if a property's name or value cannot be read, or the
    /// value isn't a valid UTF-8 string.
    pub fn model(&self) -> Result<Option<&'a str>, FdtParseError> {
        Ok(if let Some(model) = self.property("model")? {
            Some(model.as_str()?)
        } else {
            None
        })
    }

    /// Returns the value of the standard `phandle` property.
    ///
    /// # Errors
    ///
    /// Returns an error if a property's name or value cannot be read, or the
    /// value isn't a valid u32.
    pub fn phandle(&self) -> Result<Option<u32>, FdtParseError> {
        Ok(if let Some(property) = self.property("phandle")? {
            Some(property.as_u32()?)
        } else {
            None
        })
    }

    /// Returns the value of the standard `status` property.
    ///
    /// If there is no `status` property then `okay` is assumed.
    ///
    /// # Errors
    ///
    /// Returns an error if a property's name or value cannot be read, or the
    /// value isn't a valid status.
    pub fn status(&self) -> Result<Status, FdtError> {
        Ok(if let Some(status) = self.property("status")? {
            status.as_str()?.parse()?
        } else {
            Status::Okay
        })
    }

    /// Returns the value of the standard `#address-cells` property.
    ///
    /// # Errors
    ///
    /// Returns an error if a property's name or value cannot be read, or the
    /// value isn't a valid u32.
    pub fn address_cells(&self) -> Result<u32, FdtParseError> {
        Ok(if let Some(property) = self.property("#address-cells")? {
            property.as_u32()?
        } else {
            DEFAULT_ADDRESS_CELLS
        })
    }

    /// Returns the value of the standard `#size-cells` property.
    ///
    /// # Errors
    ///
    /// Returns an error if a property's name or value cannot be read, or the
    /// value isn't a valid u32.
    pub fn size_cells(&self) -> Result<u32, FdtParseError> {
        Ok(if let Some(model) = self.property("#size-cells")? {
            model.as_u32()?
        } else {
            DEFAULT_SIZE_CELLS
        })
    }

    /// Returns the value of the standard `virtual-reg` property.
    ///
    /// # Errors
    ///
    /// Returns an error if a property's name or value cannot be read, or the
    /// value isn't a valid u32.
    pub fn virtual_reg(&self) -> Result<Option<u32>, FdtParseError> {
        Ok(if let Some(property) = self.property("virtual-reg")? {
            Some(property.as_u32()?)
        } else {
            None
        })
    }

    /// Returns whether the standard `dma-coherent` property is present.
    ///
    /// # Errors
    ///
    /// Returns an error if a property can't be read.
    pub fn dma_coherent(&self) -> Result<bool, FdtParseError> {
        Ok(self.property("dma-coherent")?.is_some())
    }
}
