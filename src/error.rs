// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Error types for the `dtoolkit` crate.

#[cfg(any(feature = "write", feature = "overlay"))]
use alloc::string::String;

use thiserror::Error;

/// An error that can occur when accessing a standard node or property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
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
    /// Tried to convert part of a prop-encoded-array property to a type which
    /// was too small.
    #[error("prop-encoded-array field too big for chosen type ({cells} cells)")]
    TooManyCells {
        /// The number of (32-bit) cells in the field.
        cells: usize,
    },
}

/// An error that can occur when parsing a device tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
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
    /// A node name is invalid.
    #[error("Invalid node name")]
    InvalidNodeName,
    /// A property name is invalid.
    #[error("Invalid property name")]
    InvalidPropertyName,
    /// Memory reservation block has not been terminated with a null entry.
    #[error("Memory reservation block was not terminated with a null entry")]
    MemReserveNotTerminated,
    /// Memory reservation block has an entry that is unaligned or has invalid
    /// size.
    #[error("Memory reservation block has an entry that is unaligned or has invalid size")]
    MemReserveInvalid,
}

/// An error that can occur when parsing a property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
#[non_exhaustive]
pub enum PropertyError {
    /// The property's value has an invalid length for the requested conversion.
    #[error("property has an invalid length")]
    InvalidLength,
    /// The property's value is not a valid string.
    #[error("property is not a valid string")]
    InvalidString,
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

/// An error that can occur when building or modifying a device tree model.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[non_exhaustive]
#[cfg(feature = "write")]
pub enum ModelError {
    /// The node name is invalid.
    #[error("Invalid node name: '{0}'")]
    InvalidNodeName(String),
    /// The property name is invalid.
    #[error("Invalid property name: '{0}'")]
    InvalidPropertyName(String),
}

/// An error that can occur when resizing a buffer.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum BufferError {
    /// The requested size exceeds the capacity of the buffer.
    #[error("requested size {requested} exceeds capacity {capacity}")]
    OutOfSpace {
        /// The requested size.
        requested: usize,
        /// The capacity of the buffer.
        capacity: usize,
    },

    /// The requested size exceeds `u32::MAX`.
    #[error("requested size {requested} exceeds u32::MAX")]
    FdtLimitExceeded {
        /// The requested size.
        requested: usize,
    },

    /// A memory allocation error occurred when reserving buffer capacity.
    #[cfg(feature = "alloc")]
    #[error("memory allocation failed when resizing buffer: {0}")]
    Alloc(#[from] alloc::collections::TryReserveError),
}

/// An error that can occur when mutating a device tree.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum FdtMutError {
    /// Buffer resize failed.
    #[error("buffer resize failed: {0}")]
    Resize(#[from] BufferError),
}

/// An error that can occur when inspecting or applying a device tree overlay.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[cfg(feature = "overlay")]
#[non_exhaustive]
pub enum OverlayError {
    /// An error occurred when parsing the device tree blob or header.
    #[error("FDT parse error: {0}")]
    Parse(#[from] FdtParseError),
    /// An error occurred when accessing or converting a property value.
    #[error("property conversion error: {0}")]
    Property(#[from] PropertyError),
    /// A fragment target property (`target` or `target-path`) is missing or
    /// invalid.
    #[error("fragment target is missing or invalid")]
    InvalidFragmentTarget,
    /// A phandle value overflowed 32-bit space during renumbering or collided
    /// with a reserved value.
    #[error(
        "phandle value overflowed u32 space during renumbering or collided with reserved value"
    )]
    PhandleOverflow,
    /// A location string in `/__fixups__` has an invalid format or offset.
    #[error("invalid fixup location string")]
    InvalidFixupLocation,
    /// An external symbol referenced in `/__fixups__` could not be resolved in
    /// the base tree.
    #[error("unresolved symbol referenced in __fixups__: '{0}'")]
    UnresolvedSymbol(String),
    /// A target node referenced by a fragment could not be found in the base
    /// tree.
    #[error("target node not found in base tree: '{0}'")]
    TargetNotFound(String),
    /// A local fixup offset was out of bounds for the property value.
    #[error("local fixup offset {offset} out of bounds for property '{prop}'")]
    InvalidLocalFixup {
        /// The property name.
        prop: String,
        /// The invalid byte offset.
        offset: usize,
    },
    /// A model error occurred during overlay application.
    #[cfg(feature = "write")]
    #[error("model error: {0}")]
    Model(#[from] ModelError),
    /// A buffer or slice contains malformed data.
    #[error("malformed overlay data: {0}")]
    MalformedData(String),
}
