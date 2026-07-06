// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A read-write API for modifying a Flattened Device Tree (FDT) in-place.
//!
//! This module provides the `FdtMut` struct, which is the entry point for
//! modifying an FDT blob without converting to an intermediate representation.

mod buffer;
mod node;
mod property;

use core::fmt::{self, Debug, Display, Formatter};
use core::ptr;

pub use buffer::FdtBuffer;
pub use node::FdtNodeMut;
pub use property::FdtPropertyMut;

use crate::error::FdtParseError;
use crate::fdt::{Fdt, FdtHeader};

/// A mutable flattened device tree.
pub struct FdtMut<B> {
    pub(crate) data: B,
}

impl<B: FdtBuffer> FdtMut<B> {
    /// Creates a new mutable FDT from a buffer.
    ///
    /// # Errors
    ///
    /// Returns an [`FdtParseError`] if the data is not a valid device tree.
    pub fn new(data: B) -> Result<Self, FdtParseError> {
        Fdt::new(data.as_ref())?;
        Ok(Self { data })
    }

    /// Creates a new `FdtMut` from the given buffer without validation.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `data` contains a valid Flattened Device
    /// Tree (FDT) blob. If the blob is invalid, methods on `Fdt` and
    /// related types may panic.
    #[must_use]
    pub fn new_unchecked(data: B) -> Self {
        Self { data }
    }

    /// Returns the data of this FDT as a mutable slice.
    pub fn data_mut(&mut self) -> &mut [u8] {
        self.data.as_mut()
    }

    /// Returns a read-only view of this FDT.
    #[must_use]
    pub fn as_read_only(&self) -> Fdt<'_> {
        Fdt {
            data: self.data.as_ref(),
        }
    }

    /// Returns the root node of the device tree.
    pub fn root_mut(&mut self) -> FdtNodeMut<'_, B> {
        let root = self.as_read_only().root();

        FdtNodeMut {
            offset: root.offset,
            parent_address_space: root.parent_address_space,
            data: self,
        }
    }

    /// Finds a node by its path.
    ///
    /// For more details, refer to [`Fdt::find_node`].
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::fdt_mut::FdtMut;
    /// use dtoolkit::{Node, Property};
    ///
    /// # let mut dtb = include_bytes!("../tests/dtb/test_traversal.dtb").to_vec();
    /// let mut fdt = FdtMut::new(&mut dtb[..]).unwrap();
    /// let mut node = fdt.find_node_mut("/a/b/c").unwrap();
    /// assert_eq!(node.property("prop").unwrap().value(), b"\0\0\x04\xd2");
    /// node.property_mut("prop").unwrap().set_value(b"foo\0");
    /// assert_eq!(node.property("prop").unwrap().value(), b"foo\0");
    /// ```
    pub fn find_node_mut(&mut self, path: &str) -> Option<FdtNodeMut<'_, B>> {
        let node = self.as_read_only().find_node(path)?;

        Some(FdtNodeMut {
            offset: node.offset,
            parent_address_space: node.parent_address_space,
            data: self,
        })
    }
}

impl FdtMut<&mut [u8]> {
    /// Creates a new `FdtMut` from the given pointer without validation.
    ///
    /// # Safety
    ///
    /// The `data` pointer must be a valid pointer to a Flattened Device Tree
    /// (FDT) blob. The memory region starting at `data` and spanning
    /// `totalsize` bytes (as specified in the FDT header) must be valid and
    /// accessible for reading.
    #[expect(
        unsafe_code,
        reason = "Having a methods that reads a Device Tree from a raw pointer is useful for \
        embedded applications, where the binary only gets a pointer to DT from the firmware or \
        a bootloader. The user must ensure it trusts the data."
    )]
    #[must_use]
    pub unsafe fn from_raw_unchecked(data: *mut u8) -> Self {
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
        let slice = unsafe { core::slice::from_raw_parts_mut(data, size as usize) };
        Self::new_unchecked(slice)
    }

    /// Creates a new `FdtMut` from the given pointer.
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
    /// This function can return the same errors as [`FdtMut::new`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::fdt_mut::FdtMut;
    /// # let mut dtb = include_bytes!("../tests/dtb/test.dtb").to_vec();
    /// let ptr = dtb.as_mut_ptr();
    /// let fdt = unsafe { FdtMut::from_raw(ptr).unwrap() };
    /// ```
    #[expect(
        unsafe_code,
        reason = "Having a methods that reads a Device Tree from a raw pointer is useful for \
        embedded applications, where the binary only gets a pointer to DT from the firmware or \
        a bootloader. The user must ensure it trusts the data."
    )]
    pub unsafe fn from_raw(data: *mut u8) -> Result<Self, FdtParseError> {
        // SAFETY: The caller guarantees that `data` is a valid pointer to a Flattened
        // Device Tree (FDT) blob.
        unsafe {
            Fdt::from_raw(&raw const *data)?;
            Ok(Self::from_raw_unchecked(data))
        }
    }
}

impl<B: FdtBuffer> Debug for FdtMut<B> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let slice = self.data.as_ref();
        write!(
            f,
            "FdtMut {{ data: {} bytes at {:?} }}",
            slice.len(),
            slice.as_ptr()
        )
    }
}

impl<B: FdtBuffer> Display for FdtMut<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.as_read_only())
    }
}

impl<'a> From<FdtMut<&'a mut [u8]>> for Fdt<'a> {
    fn from(fdt: FdtMut<&'a mut [u8]>) -> Self {
        Self { data: fdt.data }
    }
}
