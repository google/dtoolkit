// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Error types for the `dtoolkit` crate.

use core::fmt::{self, Display, Formatter};

use thiserror::Error;

/// An error that can occur when parsing or accessing a device tree.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum FdtError {
    /// There was an error parsing the device tree.
    #[error("{0}")]
    Parse(#[from] FdtParseError),
    /// The `status` property of a node had an invalid value.
    #[error("Invalid status value")]
    InvalidStatus,
    /// The required `/memory` node wasn't found.
    #[error("/memory node missing")]
    MemoryMissing,
    /// The size of a prop-encoded-array property wasn't a multiple of the
    /// expected element size.
    #[error(
        "prop-encoded-array property was {size} bytes, but should have been a multiple of {chunk} cells"
    )]
    PropEncodedArraySizeMismatch {
        /// The size in bytes of the prop-encoded-array property.
        size: usize,
        /// The number of 4 byte cells expected in each element of the array.
        chunk: usize,
    },
}

/// An error that can occur when parsing a device tree.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[non_exhaustive]
pub struct FdtParseError {
    offset: usize,
    /// The type of the error that has occurred.
    pub kind: FdtErrorKind,
}

impl FdtParseError {
    pub(crate) fn new(kind: FdtErrorKind, offset: usize) -> Self {
        Self { offset, kind }
    }
}

impl Display for FdtParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} at offset {}", self.kind, self.offset)
    }
}
/// The kind of an error that can occur when parsing a device tree.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[non_exhaustive]
pub enum FdtErrorKind {
    /// The magic number of the device tree is invalid.
    #[error("Invalid FDT magic number")]
    InvalidMagic,
    /// The Device Tree version is not supported by this library.
    #[error("FDT version {0} is not supported")]
    UnsupportedVersion(u32),
    /// The length of the device tree is invalid.
    #[error("Invalid FDT length")]
    InvalidLength,
    /// The header failed validation.
    #[error("FDT header has failed validation: {0}")]
    InvalidHeader(&'static str),
    /// An invalid token was encountered.
    #[error("Bad FDT token: {0:#x}")]
    BadToken(u32),
    /// A read from data at invalid offset was attempted.
    #[error("Invalid offset in FDT")]
    InvalidOffset,
    /// An invalid string was encountered.
    #[error("Invalid string in FDT")]
    InvalidString,
    /// Memory reservation block has not been terminated with a null entry.
    #[error("Memory reservation block was not terminated with a null entry")]
    MemReserveNotTerminated,
    /// Memory reservation block has an entry that is unaligned or has invalid
    /// size.
    #[error("Memory reservation block has an entry that is unaligned or has invalid size")]
    MemReserveInvalid,
}
