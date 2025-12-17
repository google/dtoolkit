// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::ops::Deref;

use zerocopy::big_endian;

use crate::error::FdtError;
use crate::fdt::{Fdt, FdtNode};

impl Fdt<'_> {
    /// Returns the /memory node.
    ///
    /// This should always be included in a valid device tree.
    ///
    /// # Errors
    ///
    /// Returns a parse error if there was a problem reading the FDT structure
    /// to find the node, or `FdtError::MemoryMissing` if the memory node is
    /// missing.
    pub fn memory(&self) -> Result<Memory<'_>, FdtError> {
        let node = self.find_node("/memory")?.ok_or(FdtError::MemoryMissing)?;
        Ok(Memory { node })
    }
}

/// Typed wrapper for a "/memory" node.
#[derive(Clone, Copy, Debug)]
pub struct Memory<'a> {
    node: FdtNode<'a>,
}

impl<'a> Deref for Memory<'a> {
    type Target = FdtNode<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl<'a> Memory<'a> {
    /// Returns the value of the standard `initial-mapped-area` property of the
    /// memory node.
    ///
    /// # Errors
    ///
    /// Returns an error if a property's name or value cannot be read, or the
    /// size of the value isn't a multiple of 5 cells.
    pub fn initial_mapped_area(
        &self,
    ) -> Result<Option<impl Iterator<Item = InitialMappedArea> + use<'a>>, FdtError> {
        Ok(
            if let Some(property) = self.node.property("initial-mapped-area")? {
                Some(property.as_prop_encoded_array(5)?.map(|chunk| {
                    InitialMappedArea::from_cells(
                        #[expect(clippy::missing_panics_doc)]
                        chunk
                            .try_into()
                            .expect("as_prop_encoded_array should return chunks of the size that InitialMappedArea::from_cells expects"),
                    )
                }))
            } else {
                None
            },
        )
    }

    /// Returns the value of the standard `hotpluggable` property of the memory
    /// node.
    ///
    /// # Errors
    ///
    /// Returns an error if a property's name or value cannot be read.
    pub fn hotpluggable(&self) -> Result<bool, FdtError> {
        Ok(self.node.property("hotpluggable")?.is_some())
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
    fn from_cells([ea_high, ea_low, pa_high, pa_low, size]: [big_endian::U32; 5]) -> Self {
        Self {
            effective_address: u64::from(ea_high.get()) << 32 | u64::from(ea_low.get()),
            physical_address: u64::from(pa_high.get()) << 32 | u64::from(pa_low.get()),
            size: size.get(),
        }
    }
}
