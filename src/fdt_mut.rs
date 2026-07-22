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

pub use buffer::{FdtBuffer, SliceBuffer};
pub use node::FdtNodeMut;
pub use property::FdtPropertyMut;
use zerocopy::{FromBytes, big_endian};

use crate::error::{BufferError, FdtParseError};
use crate::fdt::{FDT_TAGSIZE, Fdt, FdtHeader, FdtToken};

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
    /// Tree (FDT) blob. If the blob is invalid, methods on `FdtMut` and
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
    /// let mut fdt = FdtMut::from_slice(&mut dtb).unwrap();
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

    /// Shifts data starting from `offset` by `amount` bytes to make space.
    ///
    /// This method assumes that `offset` is within the device tree structure
    /// block. It grows the structure block by `amount` and shifts the
    /// offset of the strings block, which comes after the structure block.
    fn shift_dt_struct(&mut self, offset: usize, amount: usize) -> Result<(), BufferError> {
        debug_assert!(
            {
                let header = self.as_read_only().header();
                let struct_start = header.off_dt_struct() as usize;
                let struct_end = struct_start + header.size_dt_struct() as usize;
                (struct_start..=struct_end).contains(&offset)
            },
            "offset must be within the structure block"
        );

        let old_size = self.data.as_ref().len();
        let new_size = old_size + amount;

        self.data.try_resize(new_size)?;
        self.data
            .as_mut()
            .copy_within(offset..old_size, offset + amount);

        let header = self.header_mut();

        let old_totalsize = header.totalsize.get();
        let old_size_dt_struct = header.size_dt_struct.get();
        let old_off_dt_strings = header.off_dt_strings.get();

        let amount_u32 = u32::try_from(amount).expect("amount should fit in u32");
        header.totalsize.set(old_totalsize + amount_u32);
        header.size_dt_struct.set(old_size_dt_struct + amount_u32);
        header.off_dt_strings.set(old_off_dt_strings + amount_u32);

        Ok(())
    }

    /// Compacts the device tree by removing all NOP tags.
    ///
    /// This method shifts the structure block to overwrite NOP tags, shifts the
    /// strings block to start immediately after the compacted structure block,
    /// and truncates the underlying buffer to the new total size.
    ///
    /// If there is any data stored in the free space after the strings block,
    /// it will be lost.
    ///
    /// # Panics
    ///
    /// Panics if `FdtMut` was constructed using [`Self::new_unchecked`] but
    /// contains invalid FDT blob.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::fdt_mut::FdtMut;
    ///
    /// # let mut dtb = include_bytes!("../tests/dtb/test_props.dtb").to_vec();
    /// let mut fdt = FdtMut::new(dtb).unwrap();
    /// let initial_size = fdt.as_read_only().data().len();
    ///
    /// // Removing a property replaces its data with NOP tags.
    /// let mut node = fdt.find_node_mut("/test-props").unwrap();
    /// node.remove_property("str-prop");
    /// assert_eq!(fdt.as_read_only().data().len(), initial_size);
    ///
    /// // Compacting the tree removes the NOP tags and shrinks the buffer.
    /// fdt.compact();
    /// assert!(fdt.as_read_only().data().len() < initial_size);
    /// ```
    pub fn compact(&mut self) {
        let header = self.as_read_only().header();
        let off_dt_struct = header.off_dt_struct() as usize;
        let size_dt_struct = header.size_dt_struct() as usize;
        let off_dt_strings = header.off_dt_strings() as usize;
        let size_dt_strings = header.size_dt_strings() as usize;

        let mut read_offset = off_dt_struct;
        let mut write_offset = off_dt_struct;
        let struct_end = off_dt_struct + size_dt_struct;

        while read_offset < struct_end {
            let token = self
                .as_read_only()
                .read_token(read_offset)
                .expect("valid FDT token");
            let chunk_len = token_chunk_len(self.as_read_only(), read_offset, token);

            if token != FdtToken::Nop {
                if write_offset != read_offset {
                    self.data
                        .as_mut()
                        .copy_within(read_offset..read_offset + chunk_len, write_offset);
                }
                write_offset += chunk_len;
            }

            read_offset += chunk_len;

            if token == FdtToken::End {
                break;
            }
        }

        let new_size_dt_struct = write_offset - off_dt_struct;
        let bytes_saved = size_dt_struct - new_size_dt_struct;

        if bytes_saved == 0 {
            return;
        }

        // Shift strings block left
        let strings_start = off_dt_strings;
        let strings_end = strings_start + size_dt_strings;

        self.data
            .as_mut()
            .copy_within(strings_start..strings_end, write_offset);

        // Update header
        let header = self.header_mut();
        header
            .size_dt_struct
            .set(u32::try_from(new_size_dt_struct).expect("struct size should fit in u32"));
        header
            .off_dt_strings
            .set(u32::try_from(write_offset).expect("strings offset should fit in u32"));

        let new_totalsize =
            u32::try_from(write_offset + size_dt_strings).expect("new totalsize should fit in u32");
        header.totalsize.set(new_totalsize);

        // Truncate buffer
        self.data
            .try_resize(new_totalsize as usize)
            .expect("shrinking is infallible");
    }

    fn header_mut(&mut self) -> &mut FdtHeader {
        let (header, _) =
            FdtHeader::mut_from_prefix(self.data.as_mut()).expect("Fdt should be valid");
        header
    }
}

impl<'a> FdtMut<SliceBuffer<'a>> {
    /// Creates a new `FdtMut` from a mutable byte slice by wrapping it in a
    /// [`SliceBuffer`].
    ///
    /// Reads `totalsize` from the FDT header at the start of `slice` to
    /// determine the active size of the device tree, while using the
    /// remaining capacity of `slice` (if any) for future expansion.
    ///
    /// # Errors
    ///
    /// Returns an [`FdtParseError`] if `slice` does not contain a valid device
    /// tree.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::fdt_mut::FdtMut;
    ///
    /// # let mut dtb = include_bytes!("../tests/dtb/test_traversal.dtb").to_vec();
    /// let mut fdt = FdtMut::from_slice(&mut dtb).unwrap();
    /// let mut node = fdt.find_node_mut("/a/b/c").unwrap();
    /// ```
    pub fn from_slice(slice: &'a mut [u8]) -> Result<Self, FdtParseError> {
        let buffer = SliceBuffer::new(slice)?;
        Self::new(buffer)
    }

    /// Creates a new `FdtMut` from a mutable byte slice without validation.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `slice` contains a valid Flattened Device
    /// Tree (FDT) blob. If the blob is invalid, methods on `FdtMut` and
    /// related types may panic.
    #[must_use]
    pub fn from_slice_unchecked(slice: &'a mut [u8]) -> Self {
        // SAFETY: The caller guarantees that the slice contains a valid device tree.
        let buffer = SliceBuffer::new_unchecked(slice);
        Self::new_unchecked(buffer)
    }

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
        let size = header.totalsize() as usize;
        // SAFETY: The caller must ensure that `data` is a valid pointer to a Flattened
        // Device Tree (FDT) blob. The caller must ensure the `data` spans
        // `totalsize` bytes (as specified in the FDT header).
        let slice = unsafe { core::slice::from_raw_parts_mut(data, size) };
        Self::from_slice_unchecked(slice)
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

impl<'a> From<FdtMut<SliceBuffer<'a>>> for Fdt<'a> {
    fn from(fdt: FdtMut<SliceBuffer<'a>>) -> Self {
        Self {
            data: fdt.data.slice,
        }
    }
}

fn token_chunk_len(fdt: Fdt<'_>, offset: usize, token: FdtToken) -> usize {
    match token {
        FdtToken::Nop | FdtToken::EndNode | FdtToken::End => FDT_TAGSIZE,
        FdtToken::BeginNode => {
            let name_start = offset + FDT_TAGSIZE;
            let name_end = fdt.find_string_end(name_start).expect("valid node name");
            Fdt::align_tag_offset(name_end) - offset
        }
        FdtToken::Prop => {
            let (val_len_bytes, _) = big_endian::U32::ref_from_prefix(
                fdt.data_at_offset(offset + FDT_TAGSIZE)
                    .expect("valid property length offset"),
            )
            .expect("valid property length");
            let val_len = val_len_bytes.get() as usize;
            let prop_end = offset + 3 * FDT_TAGSIZE + val_len;
            Fdt::align_tag_offset(prop_end) - offset
        }
    }
}
