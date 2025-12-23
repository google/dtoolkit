// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Error types for the `dtoolkit` crate.

use thiserror::Error;

/// An error that can occur when accessing a standard node or property.
#[derive(Copy, Clone, Debug, Eq, Error, PartialEq)]
pub enum StandardError {
    /// There was an error when converting the property value.
    #[error("error occurred when converting the property value: {0}")]
    PropertyConversion(#[from] PropertyError),
    /// The `status` property of a node had an invalid value.
    #[error("Invalid status value")]
    InvalidStatus,
    /// The required `/cpus` node wasn't found.
    #[error("/cpus node missing")]
    CpusMissing,
    /// A `/cpus/cpu` node didn't have the required `reg` property.
    #[error("/cpus/cpu node missing reg property")]
    CpuMissingReg,
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
    /// Tried to convert part of a prop-encoded-array property to a type which
    /// was too small.
    #[error("prop-encoded-array field too big for chosen type ({cells} cells)")]
    TooManyCells {
        /// The number of (32-bit) cells in the field.
        cells: usize,
    },
}

/// An error that can occur when parsing a device tree.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[non_exhaustive]
#[error("{kind} at offset {offset}")]
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

/// An error that can occur when parsing a property.
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum PropertyError {
    /// The property's value has an invalid length for the requested conversion.
    #[error("property has an invalid length")]
    InvalidLength,
    /// The property's value is not a valid string.
    #[error("property is not a valid string")]
    InvalidString,
}
