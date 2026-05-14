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
//!     .unwrap()
//!     .property(DeviceTreeProperty::new("my-property", "hello\0").unwrap())
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
//! let val: &str = prop.as_str().unwrap().as_ref();
//! assert_eq!(val, "hello");
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
mod util;
mod validate;
mod values;

use core::fmt::{self, Display, Formatter};
use core::ops::{BitOr, Shl};

use zerocopy::big_endian;

use crate::error::{PropertyError, StandardError};

#[doc(hidden)]
#[macro_export]
macro_rules! impl_property_methods {
    (get_value = |$self:ident| $get_value:expr) => {
        fn as_cells(&$self) -> Result<$crate::Cells<'a>, $crate::error::PropertyError> {
            Ok($crate::Cells(
                <[zerocopy::big_endian::U32]>::ref_from_bytes($get_value)
                    .map_err(|_| $crate::error::PropertyError::InvalidLength)?,
            ))
        }

        fn as_str(&$self) -> Result<&'a str, $crate::error::PropertyError> {
            let cstr =
                core::ffi::CStr::from_bytes_with_nul($get_value).map_err(|_| $crate::error::PropertyError::InvalidString)?;
            cstr.to_str().map_err(|_| $crate::error::PropertyError::InvalidString)
        }

        fn as_str_list(&$self) -> $crate::values::FdtStringListIterator<'a> {
            $crate::values::FdtStringListIterator { value: $get_value }
        }

        fn as_prop_encoded_array<const N: usize>(
            &$self,
            fields_cells: [usize; N],
        ) -> Result<$crate::values::PropEncodedArrayIterator<'a, N>, $crate::error::PropertyError> {
            $crate::values::PropEncodedArrayIterator::new($get_value, fields_cells)
        }
    };
}

/// A device tree node.
pub trait Node: Sized {
    /// The type used for properties of the node.
    type Property<'a>: Property + 'a
    where
        Self: 'a;

    /// The type used for child nodes.
    type Child<'a>: Node + 'a
    where
        Self: 'a;

    /// The type used for the properties iterator.
    type Properties<'a>: Iterator<Item = Self::Property<'a>> + 'a
    where
        Self: 'a;

    /// The type used for the children iterator.
    type Children<'a>: Iterator<Item = Self::Child<'a>> + 'a
    where
        Self: 'a;

    /// The type used for the name of the node.
    type Name<'a>: AsRef<str> + Copy + 'a
    where
        Self: 'a;

    /// Returns the name of this node.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::Node;
    /// use dtoolkit::fdt::Fdt;
    ///
    /// # let dtb = include_bytes!("../tests/dtb/test_children.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let root = fdt.root();
    /// let child = root.child("child1").unwrap();
    /// assert_eq!(child.name(), "child1");
    /// ```
    #[must_use]
    fn name(&self) -> Self::Name<'_>;

    /// Returns the name of this node without the unit address, if any.
    #[must_use]
    fn name_without_address(&self) -> Self::Name<'_>;

    /// Returns the property with the given name, if any.
    ///
    /// # Performance
    ///
    /// This default implementation iterates through all properties of the node.
    fn property(&self, name: &str) -> Option<Self::Property<'_>> {
        self.properties().find(|property| property.name() == name)
    }

    /// Returns an iterator over the properties of this node.
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
    /// let mut props = node.properties();
    /// assert_eq!(props.next().unwrap().name(), "u32-prop");
    /// assert_eq!(props.next().unwrap().name(), "u64-prop");
    /// assert_eq!(props.next().unwrap().name(), "str-prop");
    /// ```
    fn properties(&self) -> Self::Properties<'_>;

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
    fn child(&self, name: &str) -> Option<Self::Child<'_>> {
        let include_address = name.contains('@');
        self.children().find(|child| {
            if include_address {
                child.name().as_ref() == name
            } else {
                child.name_without_address().as_ref() == name
            }
        })
    }

    /// Returns an iterator over the children of this node.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::Node;
    /// use dtoolkit::fdt::Fdt;
    ///
    /// # let dtb = include_bytes!("../tests/dtb/test_children.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let root = fdt.root();
    /// let mut children = root.children();
    /// assert_eq!(children.next().unwrap().name(), "child1");
    /// assert_eq!(children.next().unwrap().name(), "child2@42");
    /// assert!(children.next().is_none());
    /// ```
    fn children(&self) -> Self::Children<'_>;
}

/// A property of a device tree node.
pub trait Property: Sized {
    /// The type used for strings in the property.
    type Str: AsRef<str>;

    /// The type used for the strings iterator.
    type StrList: Iterator<Item = Self::Str>;

    /// The type used for the prop-encoded-array iterator.
    type PropEncodedArray<const N: usize>: Iterator<Item = [Self::CellsItem; N]>;

    /// The type used for the cells.
    type CellsItem: Copy + ToCellInt;

    /// Returns the name of this property.
    #[must_use]
    fn name(&self) -> &str;

    /// Returns the value of this property.
    #[must_use]
    fn value(&self) -> &[u8];

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

    /// Returns the value of this property as a slide of 32-bit cells.
    ///
    /// # Errors
    ///
    /// Returns an error if the value of the property isn't a multiple of 4
    /// bytes long.
    fn as_cells(&self) -> Result<Self::CellsItem, PropertyError>;

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
    fn as_str(&self) -> Result<Self::Str, PropertyError>;

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
    fn as_str_list(&self) -> Self::StrList;

    /// Returns an iterator over the elements of the property interpreted as a
    /// `prop-encoded-array`.
    ///
    /// Each element of the array will have will have the same number of fields,
    /// where each field has the number of cells specified by the corresponding
    /// entry in `fields_cells`.
    ///
    /// # Errors
    ///
    /// Returns an error if the property's value's length can't be divided into
    /// a multiple of the given cells.
    fn as_prop_encoded_array<const N: usize>(
        &self,
        fields_cells: [usize; N],
    ) -> Result<Self::PropEncodedArray<N>, PropertyError>;
}

/// An integer value split into several big-endian u32 parts.
///
/// This is generally used in prop-encoded-array properties.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Cells<'a>(pub(crate) &'a [big_endian::U32]);

/// Trait for converting cells to integers.
pub trait ToCellInt {
    /// Converts the value to the given integer type.
    ///
    /// # Errors
    ///
    /// Returns [`StandardError::TooManyCells`] if the value has too many cells
    /// to fit in the given type.
    fn to_int<T: Default + From<u32> + Shl<usize, Output = T> + BitOr<Output = T>>(
        self,
    ) -> Result<T, StandardError>;
}

impl ToCellInt for Cells<'_> {
    fn to_int<T: Default + From<u32> + Shl<usize, Output = T> + BitOr<Output = T>>(
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

impl AsRef<[big_endian::U32]> for Cells<'_> {
    fn as_ref(&self) -> &[big_endian::U32] {
        self.0
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
