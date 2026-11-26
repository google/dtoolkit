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
use zerocopy::{FromBytes, IntoBytes, big_endian};

use crate::error::{BufferError, FdtParseError};
use crate::fdt::{
    FDT_BEGIN_NODE, FDT_END, FDT_END_NODE, FDT_LAST_COMP_VERSION, FDT_MAGIC, FDT_NOP, FDT_TAGSIZE,
    FDT_VERSION, Fdt, FdtHeader, FdtOptions, FdtToken,
};
use crate::memreserve::RawMemoryReservation;
use crate::{ToPropertyValue, Tree};

const EMPTY_FDT_MEMRSV_OFFSET: usize = size_of::<FdtHeader>();
const EMPTY_FDT_STRUCT_OFFSET: usize = EMPTY_FDT_MEMRSV_OFFSET + size_of::<RawMemoryReservation>();
const EMPTY_FDT_STRUCT_SIZE: usize = size_of_val(&EMPTY_FDT_TOKENS);

/// Size in bytes of an empty FDT blob created by [`FdtMut::create`].
pub const EMPTY_FDT_SIZE: usize = EMPTY_FDT_STRUCT_OFFSET + EMPTY_FDT_STRUCT_SIZE;

const EMPTY_FDT_HEADER: FdtHeader = {
    const fn to_u32(val: usize) -> big_endian::U32 {
        assert!(val <= u32::MAX as usize);
        #[expect(clippy::cast_possible_truncation, reason = "asserted above")]
        big_endian::U32::new(val as u32)
    }

    FdtHeader {
        magic: big_endian::U32::new(FDT_MAGIC),
        totalsize: to_u32(EMPTY_FDT_SIZE),
        off_dt_struct: to_u32(EMPTY_FDT_STRUCT_OFFSET),
        off_dt_strings: to_u32(EMPTY_FDT_SIZE),
        off_mem_rsvmap: to_u32(EMPTY_FDT_MEMRSV_OFFSET),
        version: big_endian::U32::new(FDT_VERSION),
        last_comp_version: big_endian::U32::new(FDT_LAST_COMP_VERSION),
        boot_cpuid_phys: big_endian::U32::new(0),
        size_dt_strings: big_endian::U32::new(0),
        size_dt_struct: to_u32(EMPTY_FDT_STRUCT_SIZE),
    }
};

const EMPTY_FDT_TOKENS: [big_endian::U32; 4] = [
    big_endian::U32::new(FDT_BEGIN_NODE),
    big_endian::U32::new(0),
    big_endian::U32::new(FDT_END_NODE),
    big_endian::U32::new(FDT_END),
];

/// A mutable flattened device tree.
///
/// This is the read-write counterpart to [`Fdt`], providing in-place access to
/// nodes ([`FdtNodeMut`]) and properties ([`FdtPropertyMut`]) backed by an
/// [`FdtBuffer`].
///
/// # Backing buffer
///
/// The backing storage is abstracted by [`FdtBuffer`]. Fixed-size buffers such
/// as `&mut [u8]` and `[u8; N]` use the FDT header's `totalsize` as the active
/// length, treat any trailing bytes as spare capacity for growth, and never
/// reallocate.
///
/// Growable containers such as `Vec<u8>` instead track the active length
/// themselves and resize as needed. Because of this, passing `&mut vec` and
/// passing `&mut vec[..]` (or `vec.as_mut_slice()`) behave differently:
/// `&mut vec` grows or shrinks the vector, while `&mut vec[..]` uses the
/// fixed-capacity `[u8]` implementation.
///
/// The `from_raw*` constructors ([`FdtMut::from_raw`],
/// [`FdtMut::from_raw_with_options`], and [`FdtMut::from_raw_unchecked`])
/// conservatively take `totalsize` as the maximum buffer size, leaving no spare
/// capacity for growth. If you know that more writable memory is available,
/// construct the slice yourself (e.g. with [`core::slice::from_raw_parts_mut`])
/// and pass it to [`FdtMut::new`] (or [`FdtMut::with_options`] /
/// [`FdtMut::new_unchecked`]).
pub struct FdtMut<B> {
    pub(crate) data: B,
}

impl<B: FdtBuffer> FdtMut<B> {
    /// Initializes an empty device tree (containing only the root node `/`) in
    /// `data`.
    ///
    /// Any existing contents of `data` are overwritten. The buffer must be able
    /// to hold at least [`EMPTY_FDT_SIZE`] bytes.
    ///
    /// # Errors
    ///
    /// Returns a [`BufferError`] if resizing `data` to [`EMPTY_FDT_SIZE`]
    /// fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::fdt_mut::{EMPTY_FDT_SIZE, FdtMut};
    /// use dtoolkit::{Node, Property, Tree};
    ///
    /// let mut fdt = FdtMut::create([0u8; EMPTY_FDT_SIZE + 64]).unwrap();
    /// fdt.root_mut().add_property("model", "example").unwrap();
    /// let ro = fdt.as_read_only();
    /// let prop = ro.root().property("model").unwrap();
    /// assert_eq!(prop.value_as::<&str>().unwrap(), "example");
    /// ```
    pub fn create(mut data: B) -> Result<Self, BufferError> {
        data.try_resize(EMPTY_FDT_SIZE)?;

        let bytes = data.data_mut();
        bytes[..EMPTY_FDT_MEMRSV_OFFSET].copy_from_slice(EMPTY_FDT_HEADER.as_bytes());
        bytes[EMPTY_FDT_MEMRSV_OFFSET..EMPTY_FDT_STRUCT_OFFSET]
            .copy_from_slice(RawMemoryReservation::TERMINATOR.as_bytes());
        bytes[EMPTY_FDT_STRUCT_OFFSET..EMPTY_FDT_SIZE].copy_from_slice(EMPTY_FDT_TOKENS.as_bytes());

        let fdt = Self { data };
        debug_assert!(Fdt::new(fdt.as_read_only().data()).is_ok());
        Ok(fdt)
    }

    /// Creates a new mutable FDT from a buffer.
    ///
    /// # Errors
    ///
    /// Returns an [`FdtParseError`] if the data is not a valid device tree.
    pub fn new(data: B) -> Result<Self, FdtParseError> {
        Self::with_options(data, FdtOptions::new())
    }

    /// Creates a new mutable FDT from a buffer using the specified
    /// [`FdtOptions`].
    ///
    /// # Errors
    ///
    /// Returns an [`FdtParseError`] if the data is not a valid device tree
    /// according to `options`.
    pub fn with_options(data: B, options: FdtOptions) -> Result<Self, FdtParseError> {
        Fdt::with_options(data.data(), options)?;
        Ok(Self { data })
    }

    /// Creates a new `FdtMut` from the given buffer without validation.
    ///
    /// The caller must ensure that `data` contains a valid Flattened Device
    /// Tree (FDT) blob. If the blob is invalid, methods on `FdtMut` and
    /// related types may panic.
    #[must_use]
    pub fn new_unchecked(data: B) -> Self {
        Self { data }
    }

    /// Returns the data of this FDT as a mutable slice.
    ///
    /// Modifying the returned bytes directly bypasses all FDT validation.
    /// Corrupting the FDT header (including `totalsize`), structure block,
    /// strings block, or memory reservation block may cause subsequent calls on
    /// this `FdtMut` and any views derived from it to panic or produce invalid
    /// results.
    pub fn data_mut(&mut self) -> &mut [u8] {
        self.data.data_mut()
    }

    /// Returns a read-only view of this FDT.
    #[must_use]
    pub fn as_read_only(&self) -> Fdt<'_> {
        Fdt {
            data: self.data.data(),
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
    /// # let dtb = *include_bytes!("../tests/dtb/test_traversal.dtb");
    /// let mut fdt = FdtMut::new(dtb).unwrap();
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

    /// Returns a mutable node at the given byte offset relative to the start of
    /// the `dt_struct` block, or `None` if `offset` does not point to a node's
    /// `FDT_BEGIN_NODE` tag.
    ///
    /// Note that mutating the FDT in a way that removes the node or shifts,
    /// grows, or compacts the `dt_struct` block may invalidate previously saved
    /// offsets.
    ///
    /// For more details, refer to [`Fdt::node_at_offset`].
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::fdt_mut::FdtMut;
    ///
    /// # let dtb = *include_bytes!("../tests/dtb/test_traversal.dtb");
    /// let mut fdt = FdtMut::new(dtb).unwrap();
    /// let offset = fdt.find_node_mut("/a/b/c").unwrap().offset();
    /// let node = fdt.node_at_offset_mut(offset).unwrap();
    /// assert_eq!(node.name(), "c");
    /// ```
    #[must_use]
    pub fn node_at_offset_mut(&mut self, offset: usize) -> Option<FdtNodeMut<'_, B>> {
        let node = self.as_read_only().node_at_offset(offset)?;

        Some(FdtNodeMut {
            offset: node.offset,
            parent_address_space: node.parent_address_space,
            data: self,
        })
    }

    /// Makes room in `dt_struct` and adds `name` to the `dt_strings` block.
    ///
    /// If the string is already present, it returns the existing offset without
    /// modifying the block to deduplicate data. Leaves the FDT unchanged on
    /// error.
    pub(crate) fn insert_dt_struct_and_string(
        &mut self,
        insert_offset: usize,
        struct_growth: u64,
        name: &str,
    ) -> Result<u32, BufferError> {
        let existing = self.find_existing_string(name);
        let extra_strings = if existing.is_some() {
            0
        } else {
            name.len() as u64 + 1
        };
        self.shift_dt_struct_and_reserve_strings(insert_offset, struct_growth, extra_strings)?;
        Ok(existing.unwrap_or_else(|| self.append_raw_string(name)))
    }

    fn find_existing_string(&self, name: &str) -> Option<u32> {
        let header = self.as_read_only().header();
        let str_block_offset = header.off_dt_strings() as usize;
        let str_block_size = header.size_dt_strings() as usize;
        let str_block = &self.data.data()[str_block_offset..str_block_offset + str_block_size];

        // Comparing raw bytes to avoid decoding UTF-8 (validation only covers
        // strings referenced by properties, so the string block may contain
        // strings that are not valid UTF-8).
        let mut curr_offset = 0;
        for s in str_block.split_inclusive(|&b| b == b'\0') {
            if s.strip_suffix(b"\0") == Some(name.as_bytes()) {
                return Some(u32::try_from(curr_offset).expect("string offset fits in u32"));
            }
            curr_offset += s.len();
        }

        None
    }

    fn append_raw_string(&mut self, name: &str) -> u32 {
        let header = self.as_read_only().header();
        let str_block_offset = header.off_dt_strings() as usize;
        let str_block_size = header.size_dt_strings() as usize;

        let name_bytes = name.as_bytes();
        let name_len = name_bytes.len() + 1; // plus null terminator
        let target = str_block_offset + str_block_size;

        self.data.data_mut()[target..target + name_bytes.len()].copy_from_slice(name_bytes);
        self.data.data_mut()[target + name_bytes.len()] = 0;

        let header = self.header_mut();
        header.size_dt_strings.set(
            u32::try_from(str_block_size).expect("size_dt_strings fits in u32")
                + u32::try_from(name_len).expect("name_len fits in u32"),
        );

        u32::try_from(str_block_size).expect("str_block_size fits in u32")
    }

    /// Shifts data starting from `offset` by `amount` bytes to make space.
    ///
    /// This method assumes that `offset` is within the device tree structure
    /// block. It grows the structure block by `amount` and shifts the
    /// offset of the strings block, which comes after the structure block.
    fn shift_dt_struct(&mut self, offset: usize, amount: u64) -> Result<(), BufferError> {
        self.shift_dt_struct_and_reserve_strings(offset, amount, 0)
    }

    fn shift_dt_struct_and_reserve_strings(
        &mut self,
        offset: usize,
        amount: u64,
        extra_strings: u64,
    ) -> Result<(), BufferError> {
        debug_assert!(
            {
                let header = self.as_read_only().header();
                let struct_start = header.off_dt_struct() as usize;
                let struct_end = struct_start + header.size_dt_struct() as usize;
                (struct_start..=struct_end).contains(&offset)
            },
            "offset must be within the structure block"
        );

        let header = self.as_read_only().header();
        let old_size_dt_struct = header.size_dt_struct.get();
        let old_off_dt_strings = header.off_dt_strings.get();
        let strings_end = u64::from(old_off_dt_strings)
            .saturating_add(u64::from(header.size_dt_strings()))
            .saturating_add(extra_strings);

        let old_size = self.data.data().len();
        let new_size_u64 = (old_size as u64).max(strings_end).saturating_add(amount);
        self.ensure_totalsize(new_size_u64)?;

        let amount_u32 = u32::try_from(amount).expect("amount should fit in u32");
        self.data
            .data_mut()
            .copy_within(offset..old_size, offset + amount_u32 as usize);

        let header = self.header_mut();
        header.size_dt_struct.set(old_size_dt_struct + amount_u32);
        header.off_dt_strings.set(old_off_dt_strings + amount_u32);

        Ok(())
    }

    /// Ensures the underlying buffer and `totalsize` are large enough to hold
    /// at least `required_size` bytes, dynamically expanding and updating
    /// the header where necessary.
    fn ensure_totalsize(&mut self, required_size: u64) -> Result<(), BufferError> {
        let old_size = self.data.data().len() as u64;
        if required_size > old_size {
            let required_u32 =
                u32::try_from(required_size).map_err(|_| BufferError::FdtLimitExceeded {
                    requested: required_size,
                })?;
            self.data.try_resize(required_u32 as usize)?;
            let header = self.header_mut();
            header.totalsize.set(required_u32);
        }
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
    /// # let dtb = *include_bytes!("../tests/dtb/test_props.dtb");
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
                        .data_mut()
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
            .data_mut()
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
            FdtHeader::mut_from_prefix(self.data.data_mut()).expect("Fdt should be valid");
        header
    }

    fn replace_with_nops(&mut self, start: usize, end: usize) {
        let nop_bytes = FDT_NOP.to_be_bytes();

        let mut offset = start;
        while offset < end {
            self.data.data_mut()[offset..offset + FDT_TAGSIZE].copy_from_slice(&nop_bytes);
            offset += FDT_TAGSIZE;
        }
    }

    fn write_property_value_with_padding<T: ToPropertyValue>(
        &mut self,
        value: &T,
        val_len: usize,
        padded_val_len: usize,
        val_offset: usize,
    ) {
        let data = self.data_mut();
        value.write_property_value(&mut data[val_offset..val_offset + val_len]);
        data[val_offset + val_len..val_offset + padded_val_len].fill(0);
    }
}

#[expect(
    clippy::needless_lifetimes,
    reason = "named lifetime is referenced in safety documentation"
)]
impl<'a> FdtMut<&'a mut [u8]> {
    /// Creates a new `FdtMut` from the given pointer without validation.
    ///
    /// The backing buffer of the returned `FdtMut` is sized to `totalsize`
    /// bytes, leaving no spare capacity for in-place expansion. Callers with a
    /// larger writable buffer that want to use the remaining space should
    /// construct a slice manually (e.g. with
    /// [`core::slice::from_raw_parts_mut`]) and pass it to
    /// [`FdtMut::new_unchecked`] or [`FdtMut::new`].
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
        // SAFETY: The caller guarantees that `data` is a valid pointer to a
        // Flattened Device Tree (FDT) blob. We are reading an
        // `FdtHeader` from this pointer, which is a `#[repr(C,
        // packed)]` struct. The `totalsize` field of this header is
        // then used to determine the total size of the FDT
        // blob. The caller must ensure that the memory at `data` is valid for
        // at least `size_of::<FdtHeader>()` bytes.
        let header = unsafe { ptr::read_unaligned(data.cast::<FdtHeader>()) };
        let size = header.totalsize() as usize;
        // SAFETY: The caller must ensure that `data` is a valid pointer to a
        // Flattened Device Tree (FDT) blob. The caller must ensure the
        // `data` spans `totalsize` bytes (as specified in the FDT
        // header).
        let slice = unsafe { core::slice::from_raw_parts_mut(data, size) };
        Self::new_unchecked(slice)
    }

    /// Creates a new `FdtMut` from the given pointer using the specified
    /// [`FdtOptions`], verifying that the header and `totalsize` fit within
    /// `options.max_size` bytes before reading the rest of the blob.
    ///
    /// The backing buffer of the returned `FdtMut` is sized to `totalsize`
    /// bytes (not `options.max_size`), leaving no spare capacity for in-place
    /// expansion. Callers with a larger writable buffer that want to use the
    /// remaining space should construct a slice manually (e.g. with
    /// [`core::slice::from_raw_parts_mut`]) and pass it to
    /// [`FdtMut::with_options`] or [`FdtMut::new`].
    ///
    /// # Safety
    ///
    /// If `options.max_size >= size_of::<FdtHeader>()` (40 bytes), `data` must
    /// be valid and accessible for reading at least 8 bytes (`magic` and
    /// `totalsize`), and if the header contains a valid FDT magic number and
    /// `40 <= totalsize <= options.max_size`, the memory region starting at
    /// `data` and spanning `totalsize` bytes must be valid and accessible for
    /// reading and writing for the lifetime `'a`.
    ///
    /// # Errors
    ///
    /// This function can return the same errors as
    /// [`Fdt::from_raw_with_options`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::fdt::FdtOptions;
    /// # use dtoolkit::fdt_mut::FdtMut;
    /// # let mut dtb = include_bytes!("../tests/dtb/test.dtb").to_vec();
    /// let options = FdtOptions::new().max_size(dtb.len());
    /// let ptr = dtb.as_mut_ptr();
    /// let fdt = unsafe { FdtMut::from_raw_with_options(ptr, options).unwrap() };
    /// ```
    #[expect(
        unsafe_code,
        reason = "Having a method that reads a Device Tree from a raw pointer is useful for \
        embedded applications, where the binary only gets a pointer to DT from the firmware or \
        a bootloader. The user must ensure it trusts the data."
    )]
    pub unsafe fn from_raw_with_options(
        data: *mut u8,
        options: FdtOptions,
    ) -> Result<Self, FdtParseError> {
        // SAFETY: The caller guarantees that `data` points to a readable FDT
        // header prefix (when `options.max_size >= size_of::<FdtHeader>()`) and
        // spans `totalsize` writable bytes when the FDT is valid and
        // `40 <= totalsize <= options.max_size`.
        let size = unsafe { Fdt::from_raw_with_options(data.cast_const(), options)? }
            .data()
            .len();
        // SAFETY: `Fdt::from_raw_with_options` validated the entire `size`-byte
        // FDT blob and verified `size <= options.max_size` (and
        // `size <= isize::MAX`).
        let slice = unsafe { core::slice::from_raw_parts_mut(data, size) };
        Ok(Self::new_unchecked(slice))
    }

    /// Creates a new `FdtMut` from the given pointer.
    ///
    /// The backing buffer of the returned `FdtMut` is sized to `totalsize`
    /// bytes, leaving no spare capacity for in-place expansion. Callers with a
    /// larger writable buffer that want to use the remaining space should
    /// construct a slice manually (e.g. with
    /// [`core::slice::from_raw_parts_mut`]) and pass it to [`FdtMut::new`].
    ///
    /// # Safety
    ///
    /// The `data` pointer must be valid and accessible for reading at least
    /// 8 bytes (`magic` and `totalsize`), and if the header contains a valid
    /// FDT magic number and `totalsize >= 40`, the memory region starting at
    /// `data` and spanning `totalsize` bytes must be valid and accessible for
    /// reading and writing for the lifetime `'a`.
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
        // SAFETY: The caller guarantees that `data` points to a readable FDT
        // header and spans `totalsize` writable bytes when the header is valid.
        unsafe { Self::from_raw_with_options(data, FdtOptions::new()) }
    }
}

impl<B: FdtBuffer> Debug for FdtMut<B> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let slice = self.data.data();
        write!(
            f,
            "FdtMut {{ data: {} bytes at {:?} }}",
            slice.len(),
            slice.as_ptr()
        )
    }
}

/// Formats the device tree as DTS source.
///
/// The exact output may change between minor versions of this library, but it
/// should always be valid DTS.
impl<B: FdtBuffer> Display for FdtMut<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.as_read_only())
    }
}

impl<'a> From<FdtMut<&'a mut [u8]>> for Fdt<'a> {
    fn from(fdt: FdtMut<&'a mut [u8]>) -> Self {
        let len = fdt.data.data().len();
        Self::new_unchecked(&fdt.data[..len])
    }
}

impl<'a, const N: usize> From<FdtMut<&'a mut [u8; N]>> for Fdt<'a> {
    fn from(fdt: FdtMut<&'a mut [u8; N]>) -> Self {
        let len = fdt.data.data().len();
        Self::new_unchecked(&fdt.data[..len])
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
