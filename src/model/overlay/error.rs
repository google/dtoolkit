// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Error types for the `dtoolkit` crate.

use alloc::string::String;
use core::fmt;

/// An error that can occur when applying an overlay to a device tree.
#[derive(Debug)]
#[non_exhaustive]
pub struct OverlayError {
    fragment: String,
    /// The type of the error that has occurred.
    pub kind: OverlayErrorKind,
}

impl OverlayError {
    pub(crate) fn new(kind: OverlayErrorKind, fragment: String) -> Self {
        Self { fragment, kind }
    }
}

/// The kind of an error that can occur when applying an overlay to a device
/// tree.
#[derive(Debug)]
#[non_exhaustive]
pub enum OverlayErrorKind {
    TargetPathNotFound,
    TargetPathInvalid,
    TargetNodeNotFound,
    SourceNodeNotFound,
    PhandleNotFound,
    CorruptedPhandle,
}

impl fmt::Display for OverlayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} in fragment `{}`", self.kind, self.fragment)
    }
}

impl fmt::Display for OverlayErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TargetPathNotFound => write!(f, "target-path property not found"),
            Self::TargetPathInvalid => write!(f, "target-path property is not a valid string"),
            Self::TargetNodeNotFound => write!(f, "target node not found"),
            Self::SourceNodeNotFound => write!(f, "source node not found"),
            Self::PhandleNotFound => write!(f, "phandle property not found"),
            Self::CorruptedPhandle => write!(f, "phandle property is corrupted"),
        }
    }
}

impl core::error::Error for OverlayError {}
