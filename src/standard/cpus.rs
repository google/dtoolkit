// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::fmt::{self, Display, Formatter};
use core::ops::Deref;

use crate::error::{FdtError, FdtParseError};
use crate::fdt::{Cells, Fdt, FdtNode};

impl<'a> Fdt<'a> {
    /// Returns the `/cpus` node.
    ///
    /// This should always be included in a valid device tree.
    ///
    /// # Errors
    ///
    /// Returns a parse error if there was a problem reading the FDT structure
    /// to find the node, or `FdtError::CpusMissing` if the CPUs node is
    /// missing.
    pub fn cpus(self) -> Result<Cpus<'a>, FdtError> {
        let node = self.find_node("/cpus")?.ok_or(FdtError::CpusMissing)?;
        Ok(Cpus { node })
    }
}

/// Typed wrapper for a `/cpus` node.
#[derive(Clone, Copy, Debug)]
pub struct Cpus<'a> {
    node: FdtNode<'a>,
}

impl<'a> Deref for Cpus<'a> {
    type Target = FdtNode<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl Display for Cpus<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.node.fmt(f)
    }
}

impl<'a> Cpus<'a> {
    /// Returns an iterator over the `/cpus/cpu@*` nodes.
    pub fn cpus(&self) -> impl Iterator<Item = Result<Cpu<'a>, FdtParseError>> + use<'a> {
        self.node.children().filter_map(|child| {
            let child = match child {
                Ok(child) => child,
                Err(e) => return Some(Err(e)),
            };
            let name = match child.name_without_address() {
                Ok(name) => name,
                Err(e) => return Some(Err(e)),
            };
            if name == "cpu" {
                Some(Ok(Cpu { node: child }))
            } else {
                None
            }
        })
    }
}

/// Typed wrapper for a `/cpus/cpu` node.
#[derive(Clone, Copy, Debug)]
pub struct Cpu<'a> {
    node: FdtNode<'a>,
}

impl<'a> Deref for Cpu<'a> {
    type Target = FdtNode<'a>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl Display for Cpu<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.node.fmt(f)
    }
}

impl<'a> Cpu<'a> {
    /// Returns an iterator over the IDs of the CPU, from the standard `reg`
    /// property.
    ///
    /// # Errors
    ///
    /// Returns an error if a property's name or value cannot be read, or the
    /// `reg` property is missing, or the size of the value isn't a multiple of
    /// the expected number of address and size cells.
    pub fn ids(&self) -> Result<impl Iterator<Item = Cells<'a>> + use<'a>, FdtError> {
        Ok(self
            .reg()?
            .ok_or(FdtError::CpuMissingReg)?
            .map(|reg| reg.address))
    }
}
