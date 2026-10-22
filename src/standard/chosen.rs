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
use crate::{Node, Property};

/// Typed wrapper for a `/chosen` node.
#[derive(Debug, Clone, Copy)]
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
    pub(super) const fn new(node: N) -> Self {
        Self { node }
    }

    /// Returns the value of the standard `bootargs` property.
    ///
    /// # Errors
    ///
    /// Returns an [`PropertyError::InvalidString`] if the property's value is
    /// not a null-terminated string or contains invalid UTF-8.
    pub fn bootargs(self) -> Result<Option<&'a str>, PropertyError> {
        self.node
            .property("bootargs")
            .map(Property::value_as)
            .transpose()
    }

    /// Returns the value of the standard `stdout-path` property.
    ///
    /// # Errors
    ///
    /// Returns an [`PropertyError::InvalidString`] if the property's value is
    /// not a null-terminated string or contains invalid UTF-8.
    pub fn stdout_path(self) -> Result<Option<&'a str>, PropertyError> {
        self.node
            .property("stdout-path")
            .map(Property::value_as)
            .transpose()
    }

    /// Returns the value of the standard `stdin-path` property.
    ///
    /// # Errors
    ///
    /// Returns an [`PropertyError::InvalidString`] if the property's value is
    /// not a null-terminated string or contains invalid UTF-8.
    pub fn stdin_path(self) -> Result<Option<&'a str>, PropertyError> {
        self.node
            .property("stdin-path")
            .map(Property::value_as)
            .transpose()
    }
}
