// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A read-only API for parsing and traversing a [Flattened Device Tree (FDT)].
//!
//! This module provides the [`Fdt`] struct, which is the entry point for
//! parsing and traversing an FDT blob. The API is designed to be safe and
//! efficient, performing no memory allocation and providing a zero-copy view
//! of the FDT data.
//!
//! [Flattened Device Tree (FDT)]: https://devicetree-specification.readthedocs.io/en/latest/chapter5-flattened-format.html

use core::mem::offset_of;
use core::ptr;

use zerocopy::byteorder::big_endian;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use crate::error::{FdtError, FdtErrorKind};

/// Version of the FDT specification supported by this library.
const FDT_VERSION: u32 = 17;
pub(crate) const FDT_MAGIC: u32 = 0xd00d_feed;

#[repr(C, packed)]
#[derive(Debug, Copy, Clone, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout)]
pub(crate) struct FdtHeader {
    /// Magic number of the device tree.
    pub(crate) magic: big_endian::U32,
    /// Total size of the device tree.
    pub(crate) totalsize: big_endian::U32,
    /// Offset of the device tree structure.
    pub(crate) off_dt_struct: big_endian::U32,
    /// Offset of the device tree strings.
    pub(crate) off_dt_strings: big_endian::U32,
    /// Offset of the memory reservation map.
    pub(crate) off_mem_rsvmap: big_endian::U32,
    /// Version of the device tree.
    pub(crate) version: big_endian::U32,
    /// Last compatible version of the device tree.
    pub(crate) last_comp_version: big_endian::U32,
    /// Physical ID of the boot CPU.
    pub(crate) boot_cpuid_phys: big_endian::U32,
    /// Size of the device tree strings.
    pub(crate) size_dt_strings: big_endian::U32,
    /// Size of the device tree structure.
    pub(crate) size_dt_struct: big_endian::U32,
}

impl FdtHeader {
    pub(crate) fn magic(&self) -> u32 {
        self.magic.get()
    }

    pub(crate) fn totalsize(&self) -> u32 {
        self.totalsize.get()
    }

    pub(crate) fn off_dt_struct(&self) -> u32 {
        self.off_dt_struct.get()
    }

    pub(crate) fn off_dt_strings(&self) -> u32 {
        self.off_dt_strings.get()
    }

    pub(crate) fn off_mem_rsvmap(&self) -> u32 {
        self.off_mem_rsvmap.get()
    }

    pub(crate) fn version(&self) -> u32 {
        self.version.get()
    }

    pub(crate) fn last_comp_version(&self) -> u32 {
        self.last_comp_version.get()
    }

    pub(crate) fn boot_cpuid_phys(&self) -> u32 {
        self.boot_cpuid_phys.get()
    }

    pub(crate) fn size_dt_strings(&self) -> u32 {
        self.size_dt_strings.get()
    }

    pub(crate) fn size_dt_struct(&self) -> u32 {
        self.size_dt_struct.get()
    }
}

/// A flattened device tree.
#[derive(Debug, Clone, Copy)]
pub struct Fdt<'a> {
    pub(crate) data: &'a [u8],
}

impl<'a> Fdt<'a> {
    /// Creates a new `Fdt` from the given byte slice.
    ///
    /// # Errors
    ///
    /// Returns an [`FdtErrorKind::InvalidLength`] if `data` is too short to
    /// contain a valid FDT header or if the `totalsize` field in the header
    /// does not match the length of `data`.
    ///
    /// Returns an [`FdtErrorKind::InvalidMagic`] if the `magic` field in the
    /// header is not `0xd00dfeed`.
    ///
    /// Returns an [`FdtErrorKind::UnsupportedVersion`] if the `version` field
    /// in the header is not supported by this library.
    ///
    /// Returns an [`FdtErrorKind::InvalidHeader`] if the header fails to pass
    /// the header integrity checks.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::fdt::Fdt;
    /// # let dtb = include_bytes!("../../tests/dtb/test.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// ```
    pub fn new(data: &'a [u8]) -> Result<Self, FdtError> {
        if data.len() < size_of::<FdtHeader>() {
            return Err(FdtError::new(FdtErrorKind::InvalidLength, 0));
        }

        let fdt = Fdt { data };
        let header = fdt.header();

        if header.magic() != FDT_MAGIC {
            return Err(FdtError::new(
                FdtErrorKind::InvalidMagic,
                offset_of!(FdtHeader, magic),
            ));
        }
        if !(header.last_comp_version()..=header.version()).contains(&FDT_VERSION) {
            return Err(FdtError::new(
                FdtErrorKind::UnsupportedVersion(header.version()),
                offset_of!(FdtHeader, version),
            ));
        }

        if header.totalsize() as usize != data.len() {
            return Err(FdtError::new(
                FdtErrorKind::InvalidLength,
                offset_of!(FdtHeader, totalsize),
            ));
        }

        fdt.validate_header()?;

        Ok(fdt)
    }

    /// Creates a new `Fdt` from the given pointer.
    ///
    /// # Safety
    ///
    /// The `data` pointer must be a valid pointer to a Flattened Device Tree
    /// (FDT) blob. The memory region starting at `data` and spanning
    /// `totalsize` bytes (as specified in the FDT header) must be valid and
    /// accessible for reading. The FDT blob must be well-formed and adhere
    /// to the Device Tree Specification.
    ///
    /// # Errors
    ///
    /// This function can return the same errors as [`Fdt::new`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use dtoolkit::fdt::Fdt;
    /// # let dtb = include_bytes!("../../tests/dtb/test.dtb");
    /// let ptr = dtb.as_ptr();
    /// let fdt = unsafe { Fdt::from_raw(ptr).unwrap() };
    /// ```
    pub unsafe fn from_raw(data: *const u8) -> Result<Self, FdtError> {
        // SAFETY: The caller guarantees that `data` is a valid pointer to a Flattened
        // Device Tree (FDT) blob. We are reading an `FdtHeader` from this
        // pointer, which is a `#[repr(C, packed)]` struct. The `totalsize`
        // field of this header is then used to determine the total size of the FDT
        // blob. The caller must ensure that the memory at `data` is valid for
        // at least `size_of::<FdtHeader>()` bytes.
        let header = unsafe { ptr::read_unaligned(data.cast::<FdtHeader>()) };
        let size = header.totalsize();
        // SAFETY: The caller must ensure that `data` is a valid pointer to a Flattened
        // Device Tree (FDT) blob. The caller must ensure the `data` spans
        // `totalsize` bytes (as specified in the FDT header).
        let slice = unsafe { core::slice::from_raw_parts(data, size as usize) };
        Fdt::new(slice)
    }

    fn validate_header(&self) -> Result<(), FdtError> {
        let header = self.header();
        let data = &self.data;

        let off_mem_rsvmap = header.off_mem_rsvmap() as usize;
        let off_dt_struct = header.off_dt_struct() as usize;
        let off_dt_strings = header.off_dt_strings() as usize;
        if off_mem_rsvmap > off_dt_struct {
            return Err(FdtError::new(
                FdtErrorKind::InvalidHeader("dt_struct not after memrsvmap"),
                offset_of!(FdtHeader, off_mem_rsvmap),
            ));
        }
        if off_dt_struct > data.len() {
            return Err(FdtError::new(
                FdtErrorKind::InvalidHeader("struct offset out of bounds"),
                offset_of!(FdtHeader, off_dt_struct),
            ));
        }
        if off_dt_strings > data.len() {
            return Err(FdtError::new(
                FdtErrorKind::InvalidHeader("strings offset out of bounds"),
                offset_of!(FdtHeader, off_dt_strings),
            ));
        }

        let size_dt_struct = header.size_dt_struct() as usize;
        let size_dt_strings = header.size_dt_strings() as usize;
        if off_dt_struct.saturating_add(size_dt_struct) > data.len() {
            return Err(FdtError::new(
                FdtErrorKind::InvalidHeader("struct block overflows"),
                offset_of!(FdtHeader, size_dt_struct),
            ));
        }
        if off_dt_strings.saturating_add(size_dt_strings) > data.len() {
            return Err(FdtError::new(
                FdtErrorKind::InvalidHeader("strings block overflows"),
                offset_of!(FdtHeader, size_dt_strings),
            ));
        }
        if off_dt_struct.saturating_add(size_dt_struct) > off_dt_strings {
            return Err(FdtError::new(
                FdtErrorKind::InvalidHeader("strings block not after struct block"),
                offset_of!(FdtHeader, off_dt_strings),
            ));
        }

        Ok(())
    }

    /// Returns the header of the device tree.
    pub(crate) fn header(&self) -> &FdtHeader {
        let (header, _remaining_bytes) = FdtHeader::ref_from_prefix(self.data)
            .expect("new() checks if the slice is at least as big as the header");
        header
    }

    /// Returns the underlying data slice of the FDT.
    #[must_use]
    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Returns the version of the FDT.
    #[must_use]
    pub fn version(&self) -> u32 {
        self.header().version()
    }

    /// Returns the last compatible version of the FDT.
    #[must_use]
    pub fn last_comp_version(&self) -> u32 {
        self.header().last_comp_version()
    }

    /// Returns the physical ID of the boot CPU.
    #[must_use]
    pub fn boot_cpuid_phys(&self) -> u32 {
        self.header().boot_cpuid_phys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::FdtErrorKind;

    const FDT_HEADER_OK: &[u8] = &[
        0xd0, 0x0d, 0xfe, 0xed, // magic
        0x00, 0x00, 0x00, 0x3c, // totalsize = 60
        0x00, 0x00, 0x00, 0x38, // off_dt_struct = 56
        0x00, 0x00, 0x00, 0x3c, // off_dt_strings = 60
        0x00, 0x00, 0x00, 0x28, // off_mem_rsvmap = 40
        0x00, 0x00, 0x00, 0x11, // version = 17
        0x00, 0x00, 0x00, 0x10, // last_comp_version = 16
        0x00, 0x00, 0x00, 0x00, // boot_cpuid_phys = 0
        0x00, 0x00, 0x00, 0x00, // size_dt_strings = 0
        0x00, 0x00, 0x00, 0x04, // size_dt_struct = 4
        0x00, 0x00, 0x00, 0x00, // memory reservation
        0x00, 0x00, 0x00, 0x00, // ...
        0x00, 0x00, 0x00, 0x00, // ...
        0x00, 0x00, 0x00, 0x00, // ...
        0x00, 0x00, 0x00, 0x09, // dt struct
    ];

    #[test]
    fn header_is_parsed_correctly() {
        let fdt = Fdt::new(FDT_HEADER_OK).unwrap();
        let header = fdt.header();

        assert_eq!(header.totalsize(), 60);
        assert_eq!(header.off_dt_struct(), 56);
        assert_eq!(header.off_dt_strings(), 60);
        assert_eq!(header.off_mem_rsvmap(), 40);
        assert_eq!(header.version(), 17);
        assert_eq!(header.last_comp_version(), 16);
        assert_eq!(header.boot_cpuid_phys(), 0);
        assert_eq!(header.size_dt_strings(), 0);
        assert_eq!(header.size_dt_struct(), 4);
    }

    #[test]
    fn invalid_magic() {
        let mut header = FDT_HEADER_OK.to_vec();
        header[0] = 0x00;
        let result = Fdt::new(&header);
        assert!(matches!(result, Err(e) if matches!(e.kind, FdtErrorKind::InvalidMagic)));
    }

    #[test]
    fn invalid_length() {
        let header = &FDT_HEADER_OK[..10];
        let result = Fdt::new(header);
        assert!(matches!(result, Err(e) if matches!(e.kind, FdtErrorKind::InvalidLength)));
    }

    #[test]
    fn unsupported_version() {
        let mut header = FDT_HEADER_OK.to_vec();
        header[23] = 0x10;
        let result = Fdt::new(&header);
        assert!(matches!(result, Err(e) if matches!(e.kind, FdtErrorKind::UnsupportedVersion(16))));
    }
}
