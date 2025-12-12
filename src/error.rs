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
#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum FdtError {
    /// There was an error parsing the device tree.
    #[error("{0}")]
    Parse(#[from] FdtParseError),
}

/// An error that can occur when parsing a device tree.
#[derive(Clone, Debug, Eq, PartialEq)]
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

/// The kind of an error that can occur when parsing a device tree.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FdtErrorKind {
    /// The magic number of the device tree is invalid.
    InvalidMagic,
    /// The Device Tree version is not supported by this library.
    UnsupportedVersion(u32),
    /// The length of the device tree is invalid.
    InvalidLength,
    /// The header failed validation.
    InvalidHeader(&'static str),
    /// An invalid token was encountered.
    BadToken(u32),
    /// A read from data at invalid offset was attempted.
    InvalidOffset,
    /// An invalid string was encountered.
    InvalidString,
    /// Memory reservation block has not been terminated with a null entry.
    MemReserveNotTerminated,
    /// Memory reservation block has an entry that is unaligned or has invalid
    /// size.
    MemReserveInvalid,
}

impl Display for FdtParseError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} at offset {}", self.kind, self.offset)
    }
}

impl Display for FdtErrorKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            FdtErrorKind::InvalidMagic => write!(f, "invalid FDT magic number"),
            FdtErrorKind::UnsupportedVersion(version) => {
                write!(f, "the FDT version {version} is not supported")
            }
            FdtErrorKind::InvalidLength => write!(f, "invalid FDT length"),
            FdtErrorKind::InvalidHeader(msg) => {
                write!(f, "FDT header has failed validation: {msg}")
            }
            FdtErrorKind::BadToken(token) => write!(f, "bad FDT token: 0x{token:x}"),
            FdtErrorKind::InvalidOffset => write!(f, "invalid offset in FDT"),
            FdtErrorKind::InvalidString => write!(f, "invalid string in FDT"),
            FdtErrorKind::MemReserveNotTerminated => write!(
                f,
                "memory reservation block not terminated with a null entry"
            ),
            FdtErrorKind::MemReserveInvalid => write!(
                f,
                "memory reservation block has an entry that is unaligned or has invalid size"
            ),
        }
    }
}

impl core::error::Error for FdtParseError {}
