// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::fmt::{self, Display, Formatter};
use core::ops::Deref;

use crate::error::PropertyError;
use crate::fdt::{Fdt, FdtNode};
use crate::{Node, Property};

impl<'a> Fdt<'a> {
    /// Returns the `/chosen` node, if it exists.
    #[must_use]
    pub fn chosen(self) -> Option<Chosen<FdtNode<'a>>> {
        let node = self.find_node("/chosen")?;
        Some(Chosen { node })
    }
}

/// Typed wrapper for a `/chosen` node.
#[derive(Clone, Copy, Debug)]
pub struct Chosen<N> {
    node: N,
}

impl<N> Deref for Chosen<N> {
    type Target = N;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl<N: Display> Display for Chosen<N> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.node.fmt(f)
    }
}

impl<'a, N: Node<'a>> Chosen<N> {
    /// Returns the value of the standard `bootargs` property.
    ///
    /// # Errors
    ///
    /// Returns an [`PropertyError::InvalidString`] if the property's value is
    /// not a null-terminated string or contains invalid UTF-8.
    pub fn bootargs(&self) -> Result<Option<&'a str>, PropertyError> {
        self.node
            .property("bootargs")
            .map(|value| value.as_str())
            .transpose()
    }

    /// Returns the value of the standard `stdout-path` property.
    ///
    /// # Errors
    ///
    /// Returns an [`PropertyError::InvalidString`] if the property's value is
    /// not a null-terminated string or contains invalid UTF-8.
    pub fn stdout_path(&self) -> Result<Option<&'a str>, PropertyError> {
        self.node
            .property("stdout-path")
            .map(|value| value.as_str())
            .transpose()
    }

    /// Returns the value of the standard `stdin-path` property.
    ///
    /// # Errors
    ///
    /// Returns an [`PropertyError::InvalidString`] if the property's value is
    /// not a null-terminated string or contains invalid UTF-8.
    pub fn stdin_path(&self) -> Result<Option<&'a str>, PropertyError> {
        self.node
            .property("stdin-path")
            .map(|value| value.as_str())
            .transpose()
    }
}
