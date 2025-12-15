// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A library for parsing and manipulating Flattened Device Tree (FDT) blobs.
//!
//! This library provides a comprehensive API for working with FDTs, including:
//!
//! - A read-only API for parsing and traversing FDTs without memory allocation.
//! - A read-write API for creating and modifying FDTs in memory.
//! - Support for applying device tree overlays.
//! - Outputting device trees in DTS source format.
//!
//! The library is written purely in Rust and is `#![no_std]` compatible. If
//! you don't need the Device Tree manipulation functionality, the library is
//! also no-`alloc`-compatible.
//!
//! ## Read-Only API
//!
//! The read-only API is centered around the [`Fdt`](fdt::Fdt) struct, which
//! provides a safe, zero-copy view of an FDT blob. You can use this API
//! to traverse the device tree, inspect nodes and properties, and read
//! property values.
//!
//! Note that because the [`Fdt`](fdt::Fdt) struct is zero-copy, certain
//! operations such as node or property lookups run in linear time. If you need
//! to perform these operations often, and you can spare extra memory, it might
//! be beneficial to convert from [`Fdt`](fdt::Fdt) to
//! [`DeviceTree`](model::DeviceTree) first.
//!
//! ## Read-Write API
//!
//! The read-write API is centered around the [`DeviceTree`](model::DeviceTree)
//! struct, which provides a mutable, in-memory representation of a device tree.
//! You can use this API to create new device trees from scratch, modify
//! existing ones, and serialize them back to an FDT blob.
//!
//! Internally it is built upon hash maps, meaning that most lookup and
//! modification operations run in constant time.
//!
//! # Examples
//!
//! ```
//! use dtoolkit::fdt::Fdt;
//! use dtoolkit::model::{DeviceTree, DeviceTreeNode, DeviceTreeProperty};
//! use dtoolkit::{Node, Property};
//!
//! // Create a new device tree from scratch.
//! let mut tree = DeviceTree::new();
//!
//! // Add a child node to the root.
//! let child = DeviceTreeNode::builder("child")
//!     .property(DeviceTreeProperty::new("my-property", "hello\0"))
//!     .build();
//! tree.root.add_child(child);
//!
//! // Serialize the device tree to a DTB.
//! let dtb = tree.to_dtb();
//!
//! // Parse the DTB with the read-only API.
//! let fdt = Fdt::new(&dtb).unwrap();
//!
//! // Find the child node and read its property.
//! let child_node = fdt.find_node("/child").unwrap();
//! let prop = child_node.property("my-property").unwrap();
//! assert_eq!(prop.as_str().unwrap(), "hello");
//!
//! // Display the DTS
//! println!("{}", fdt);
//! ```

#![cfg_attr(not(test), no_std)]
#![warn(missing_docs, rustdoc::missing_crate_level_docs)]
#![deny(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "write")]
extern crate alloc;

pub mod error;
pub mod fdt;
pub mod memreserve;
#[cfg(feature = "write")]
pub mod model;
pub mod standard;

use core::ffi::CStr;
use core::fmt::{self, Display, Formatter};
use core::ops::{BitOr, Shl};

use zerocopy::{FromBytes, big_endian};

use crate::error::{PropertyError, StandardError};
use crate::standard::{AddressSpaceProperties, DEFAULT_ADDRESS_CELLS, DEFAULT_SIZE_CELLS, Status};

/// A device tree node.
pub trait Node<'a>: Sized {
    /// The type used for properties of the node.
    type Property: Property<'a>;

    /// Returns the name of this node.
    ///
    /// # Errors
    ///
    /// Returns an error if there was a problem parsing the device tree.
    fn name(&self) -> &'a str;

    /// Returns the name of this node without the unit address, if any.
    ///
    /// # Errors
    ///
    /// Returns an
    /// [`FdtErrorKind::InvalidOffset`](crate::error::FdtErrorKind::InvalidOffset)
    /// if the name offset is invalid or an
    /// [`FdtErrorKind::InvalidString`](crate::error::FdtErrorKind::InvalidString) if the string at the offset is not null-terminated
    /// or contains invalid UTF-8.
    fn name_without_address(&self) -> &'a str {
        let name = self.name();
        if let Some((name, _)) = name.split_once('@') {
            name
        } else {
            name
        }
    }

    /// Returns the property with the given name, if any.
    ///
    /// # Performance
    ///
    /// This method iterates through all properties of the node.
    ///
    /// # Panics
    ///
    /// Panics if the [`Fdt`] structure was constructed using
    /// [`Fdt::new_unchecked`] or [`Fdt::from_raw_unchecked`] and the FDT is not
    /// valid.
    fn property(&self, name: &str) -> Option<Self::Property> {
        self.properties().find(|property| property.name() == name)
    }

    /// Returns an iterator over the properties of this node.
    fn properties(&self) -> impl Iterator<Item = Self::Property> + use<'a, Self>;

    /// Returns a child node by its name.
    ///
    /// If the given name contains a _unit-address_ (the part after the `@`
    /// sign) then both the _node-name_ and _unit-address_ must match. If it
    /// doesn't have a _unit-address_, then nodes with any _unit-address_ or
    /// none will be allowed.
    ///
    /// For example, searching for `memory` as a child of `/` would match either
    /// `/memory` or `/memory@4000`, while `memory@4000` would match only the
    /// latter.
    ///
    /// # Errors
    ///
    /// Returns an error if a child node's name cannot be read.
    fn child(&self, name: &str) -> Option<Self> {
        let include_address = name.contains('@');
        self.children().find(|child| {
            if include_address {
                child.name() == name
            } else {
                child.name_without_address() == name
            }
        })
    }

    /// Returns an iterator over the children of this node.
    fn children(&self) -> impl Iterator<Item = Self> + use<'a, Self>;

    /// Returns the value of the standard `compatible` property.
    #[must_use]
    fn compatible(&self) -> Option<impl Iterator<Item = &'a str> + use<'a, Self>> {
        self.property("compatible")
            .map(|property| property.as_str_list())
    }

    /// Returns whether this node has a `compatible` properties containing the
    /// given string.
    #[must_use]
    fn is_compatible(&self, compatible_filter: &str) -> bool {
        if let Some(mut compatible) = self.compatible() {
            compatible.any(|c| c == compatible_filter)
        } else {
            false
        }
    }

    /// Finds all child nodes with a `compatible` property containing the given
    /// string.
    fn find_compatible<'f>(
        &self,
        compatible_filter: &'f str,
    ) -> impl Iterator<Item = Self> + use<'a, 'f, Self> {
        self.children()
            .filter(move |child| child.is_compatible(compatible_filter))
    }

    /// Returns the value of the standard `model` property.
    ///
    /// # Errors
    ///
    /// Returns an error if the value isn't a valid UTF-8 string.
    fn model(&self) -> Result<Option<&'a str>, StandardError> {
        if let Some(model) = self.property("model") {
            Ok(Some(model.as_str()?))
        } else {
            Ok(None)
        }
    }

    /// Returns the value of the standard `phandle` property.
    ///
    /// # Errors
    ///
    /// Returns an error if the value isn't a valid u32.
    fn phandle(&self) -> Result<Option<u32>, StandardError> {
        if let Some(property) = self.property("phandle") {
            Ok(Some(property.as_u32()?))
        } else {
            Ok(None)
        }
    }

    /// Returns the value of the standard `status` property.
    ///
    /// If there is no `status` property then `okay` is assumed.
    ///
    /// # Errors
    ///
    /// Returns an error if the value isn't a valid status.
    fn status(&self) -> Result<Status, StandardError> {
        if let Some(status) = self.property("status") {
            Ok(status.as_str()?.parse()?)
        } else {
            Ok(Status::Okay)
        }
    }

    /// Returns the value of the standard `#address-cells` property.
    ///
    /// # Errors
    ///
    /// Returns an error if the value isn't a valid u32.
    fn address_cells(&self) -> Result<u32, StandardError> {
        if let Some(property) = self.property("#address-cells") {
            Ok(property.as_u32()?)
        } else {
            Ok(DEFAULT_ADDRESS_CELLS)
        }
    }

    /// Returns the value of the standard `#size-cells` property.
    ///
    /// # Errors
    ///
    /// Returns an error if the value isn't a valid u32.
    fn size_cells(&self) -> Result<u32, StandardError> {
        if let Some(model) = self.property("#size-cells") {
            Ok(model.as_u32()?)
        } else {
            Ok(DEFAULT_SIZE_CELLS)
        }
    }

    /// Returns the values of the standard `#address-cells` and `#size_cells`
    /// properties.
    #[must_use]
    fn address_space(&self) -> AddressSpaceProperties {
        AddressSpaceProperties {
            address_cells: self.address_cells().unwrap_or(DEFAULT_ADDRESS_CELLS),
            size_cells: self.size_cells().unwrap_or(DEFAULT_SIZE_CELLS),
        }
    }

    /// Returns the value of the standard `virtual-reg` property.
    ///
    /// # Errors
    ///
    /// Returns an error if the value isn't a valid u32.
    fn virtual_reg(&self) -> Result<Option<u32>, StandardError> {
        if let Some(property) = self.property("virtual-reg") {
            Ok(Some(property.as_u32()?))
        } else {
            Ok(None)
        }
    }

    /// Returns whether the standard `dma-coherent` property is present.
    #[must_use]
    fn dma_coherent(&self) -> bool {
        self.property("dma-coherent").is_some()
    }
}

/// A property of a device tree node.
pub trait Property<'a>: Sized {
    /// Returns the name of this property.
    #[must_use]
    fn name(&self) -> &'a str;

    /// Returns the value of this property.
    #[must_use]
    fn value(&self) -> &'a [u8];

    /// Returns the value of this property as a `u32`.
    ///
    /// # Errors
    ///
    /// Returns an [`PropertyError::InvalidLength`] if the property's value is
    /// not 4 bytes long.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::fdt::Fdt;
    /// use dtoolkit::{Node, Property};
    ///
    /// # let dtb = include_bytes!("../tests/dtb/test_props.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let node = fdt.find_node("/test-props").unwrap();
    /// let prop = node.property("u32-prop").unwrap();
    /// assert_eq!(prop.as_u32().unwrap(), 0x12345678);
    /// ```
    fn as_u32(&self) -> Result<u32, PropertyError> {
        self.value()
            .try_into()
            .map(u32::from_be_bytes)
            .map_err(|_| PropertyError::InvalidLength)
    }

    /// Returns the value of this property as a `u64`.
    ///
    /// # Errors
    ///
    /// Returns an [`PropertyError::InvalidLength`] if the property's value is
    /// not 8 bytes long.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::fdt::Fdt;
    /// use dtoolkit::{Node, Property};
    ///
    /// # let dtb = include_bytes!("../tests/dtb/test_props.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let node = fdt.find_node("/test-props").unwrap();
    /// let prop = node.property("u64-prop").unwrap();
    /// assert_eq!(prop.as_u64().unwrap(), 0x1122334455667788);
    /// ```
    fn as_u64(&self) -> Result<u64, PropertyError> {
        self.value()
            .try_into()
            .map(u64::from_be_bytes)
            .map_err(|_| PropertyError::InvalidLength)
    }

    /// Returns the value of this property as a string.
    ///
    /// # Errors
    ///
    /// Returns an [`PropertyError::InvalidString`] if the property's value is
    /// not a null-terminated string or contains invalid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::fdt::Fdt;
    /// use dtoolkit::{Node, Property};
    ///
    /// # let dtb = include_bytes!("../tests/dtb/test_props.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let node = fdt.find_node("/test-props").unwrap();
    /// let prop = node.property("str-prop").unwrap();
    /// assert_eq!(prop.as_str().unwrap(), "hello world");
    /// ```
    fn as_str(&self) -> Result<&'a str, PropertyError> {
        let cstr =
            CStr::from_bytes_with_nul(self.value()).map_err(|_| PropertyError::InvalidString)?;
        cstr.to_str().map_err(|_| PropertyError::InvalidString)
    }

    /// Returns an iterator over the strings in this property.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::fdt::Fdt;
    /// use dtoolkit::{Node, Property};
    ///
    /// # let dtb = include_bytes!("../tests/dtb/test_props.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let node = fdt.find_node("/test-props").unwrap();
    /// let prop = node.property("str-list-prop").unwrap();
    /// let mut str_list = prop.as_str_list();
    /// assert_eq!(str_list.next(), Some("first"));
    /// assert_eq!(str_list.next(), Some("second"));
    /// assert_eq!(str_list.next(), Some("third"));
    /// assert_eq!(str_list.next(), None);
    /// ```
    fn as_str_list(&self) -> impl Iterator<Item = &'a str> + use<'a, Self> {
        FdtStringListIterator {
            value: self.value(),
        }
    }

    /// Returns an iterator over the elements of the property interpreted as a
    /// `prop-encoded-array`.
    ///
    /// Each element of the array will have will have the same number of fields,
    /// where each field has the number of cells specified by the corresponding
    /// entry in `fields_cells`.
    fn as_prop_encoded_array<const N: usize>(
        &self,
        fields_cells: [usize; N],
    ) -> Result<impl Iterator<Item = [Cells<'a>; N]> + use<'a, N, Self>, StandardError> {
        let chunk_cells = fields_cells.iter().sum();
        let chunk_bytes = chunk_cells * size_of::<u32>();
        if !self.value().len().is_multiple_of(chunk_bytes) {
            return Err(StandardError::PropEncodedArraySizeMismatch {
                size: self.value().len(),
                chunk: chunk_cells,
            });
        }
        Ok(self.value().chunks_exact(chunk_bytes).map(move |chunk| {
            let mut cells = <[big_endian::U32]>::ref_from_bytes(chunk)
                .expect("chunk should be a multiple of 4 bytes because of chunks_exact");
            fields_cells.map(|field_cells| {
                let field;
                (field, cells) = cells.split_at(field_cells);
                Cells(field)
            })
        }))
    }
}

struct FdtStringListIterator<'a> {
    value: &'a [u8],
}

impl<'a> Iterator for FdtStringListIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.value.is_empty() {
            return None;
        }
        let cstr = CStr::from_bytes_until_nul(self.value).ok()?;
        let s = cstr.to_str().ok()?;
        self.value = &self.value[s.len() + 1..];
        Some(s)
    }
}

/// An integer value split into several big-endian u32 parts.
///
/// This is generally used in prop-encoded-array properties.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Cells<'a>(pub(crate) &'a [big_endian::U32]);

impl Cells<'_> {
    /// Converts the value to the given integer type.
    ///
    /// # Errors
    ///
    /// Returns `FdtError::TooManyCells` if the value has too many cells to fit
    /// in the given type.
    pub fn to_int<T: Default + From<u32> + Shl<usize, Output = T> + BitOr<Output = T>>(
        self,
    ) -> Result<T, StandardError> {
        if size_of::<T>() < self.0.len() * size_of::<u32>() {
            Err(StandardError::TooManyCells {
                cells: self.0.len(),
            })
        } else if let [size] = self.0 {
            Ok(size.get().into())
        } else {
            let mut value = Default::default();
            for cell in self.0 {
                value = value << 32 | cell.get().into();
            }
            Ok(value)
        }
    }
}

impl Display for Cells<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("0x")?;
        for part in self.0 {
            write!(f, "{part:08x}")?;
        }
        Ok(())
    }
}
