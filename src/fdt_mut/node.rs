// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::fmt;
use core::fmt::{Display, Formatter};

use zerocopy::IntoBytes;

use crate::fdt::node::InnerChildIter;
use crate::fdt::node::private::FdtChildIter;
use crate::fdt::property::{FdtPropIter, InnerPropIter};
use crate::fdt::{FDT_TAGSIZE, Fdt, FdtNode, FdtPropertyHeader};
use crate::fdt_mut::buffer::FdtBuffer;
use crate::fdt_mut::property::FdtPropMutIter;
use crate::fdt_mut::{FdtMut, FdtPropertyMut};
use crate::standard::{AddressSpaceProperties, NodeStandard};
use crate::{Node, Property};

/// A mutable device tree node.
#[derive(Debug)]
pub struct FdtNodeMut<'a, B: FdtBuffer> {
    pub(crate) data: &'a mut FdtMut<B>,
    pub(crate) offset: usize,
    pub(crate) parent_address_space: AddressSpaceProperties,
}

impl<B: FdtBuffer> FdtNodeMut<'_, B> {
    /// Returns a mutable property by its name.
    pub fn property_mut(&mut self, name: &str) -> Option<FdtPropertyMut<'_, B>> {
        let mut props = self.properties_mut();
        while let Some(prop) = props.next() {
            if prop.as_read_only().name() == name {
                return Some(FdtPropertyMut {
                    prop_offset: prop.prop_offset,
                    value_offset: prop.value_offset,
                    len: prop.len,
                    nameoff: prop.nameoff,
                    data: self.data,
                });
            }
        }
        None
    }

    /// Returns a mutable child node by its name.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::Node;
    /// use dtoolkit::fdt_mut::FdtMut;
    ///
    /// # let mut dtb = include_bytes!("../../tests/dtb/test_traversal.dtb").to_vec();
    /// let mut fdt = FdtMut::from_slice(&mut dtb).unwrap();
    /// let mut parent = fdt.find_node_mut("/a/b").unwrap();
    /// let child = parent.child_mut("c").unwrap();
    ///
    /// assert_eq!(child.name(), "c");
    /// ```
    pub fn child_mut(&mut self, name: &str) -> Option<FdtNodeMut<'_, B>> {
        let mut children = self.children_mut();
        while let Some(child) = children.next() {
            if child.name() == name {
                return Some(FdtNodeMut {
                    offset: child.offset,
                    parent_address_space: child.parent_address_space,
                    data: self.data,
                });
            }
        }
        None
    }

    /// Adds a new property to this node.
    ///
    /// # Errors
    ///
    /// Returns an error if resizing the buffer fails.
    ///
    /// # Panics
    ///
    /// Panics if the node name or existing structure is invalid, or if lengths
    /// exceed `u32::MAX`.
    pub fn add_property(
        &mut self,
        name: &str,
        value: &[u8],
    ) -> Result<FdtPropertyMut<'_, B>, crate::error::FdtMutError> {
        let nameoff = self.data.add_string(name)? as usize;
        let fdt = self.as_read_only().fdt;
        let mut offset = self.offset + FDT_TAGSIZE;
        let name_end = fdt.find_string_end(offset).expect("valid node name");
        offset = Fdt::align_tag_offset(name_end);
        let insert_offset = fdt.skip_props(offset, false).expect("valid dt");

        let padded_val_len = Fdt::align_tag_offset(value.len());
        let required_space = size_of::<FdtPropertyHeader>() + padded_val_len;

        self.data.shift_dt_struct(insert_offset, required_space)?;

        let data = self.data.data_mut();
        let header = FdtPropertyHeader::new(
            u32::try_from(value.len()).expect("len fits in u32"),
            u32::try_from(nameoff).expect("nameoff fits in u32"),
        );
        data[insert_offset..insert_offset + size_of::<FdtPropertyHeader>()]
            .copy_from_slice(header.as_bytes());

        let val_offset = insert_offset + size_of::<FdtPropertyHeader>();
        self.data
            .copy_data_with_padding(value, padded_val_len, val_offset);

        Ok(FdtPropertyMut {
            prop_offset: insert_offset,
            value_offset: val_offset,
            len: value.len(),
            nameoff,
            data: self.data,
        })
    }

    /// Removes a property from this node by its name.
    ///
    /// This is a convenience method that finds a property by name and calls
    /// [`FdtPropertyMut::remove`] on it.
    ///
    /// Returns `true` if the property was present and successfully removed,
    /// `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::fdt_mut::FdtMut;
    /// use dtoolkit::{Node, Property};
    ///
    /// # let mut dtb = include_bytes!("../../tests/dtb/test_traversal.dtb").to_vec();
    /// let mut fdt = FdtMut::from_slice(&mut dtb).unwrap();
    /// let mut node = fdt.find_node_mut("/a/b/c").unwrap();
    /// assert!(node.property("prop").is_some());
    /// assert!(node.remove_property("prop"));
    /// assert!(node.property("prop").is_none());
    /// assert!(!node.remove_property("prop"));
    /// ```
    pub fn remove_property(&mut self, name: &str) -> bool {
        if let Some(prop) = self.property_mut(name) {
            prop.remove();
            true
        } else {
            false
        }
    }

    /// Removes a child node by its name.
    ///
    /// This is a convenience method that searches for a child node by name and
    /// calls [`FdtNodeMut::remove`] on it if found.
    ///
    /// Returns `true` if the child was present and successfully removed,
    /// `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::Node;
    /// use dtoolkit::fdt_mut::FdtMut;
    ///
    /// # let mut dtb = include_bytes!("../../tests/dtb/test_traversal.dtb").to_vec();
    /// let mut fdt = FdtMut::from_slice(&mut dtb).unwrap();
    /// let mut node = fdt.find_node_mut("/a/b").unwrap();
    /// assert!(node.child("c").is_some());
    /// assert!(node.remove_child("c"));
    /// assert!(node.child("c").is_none());
    /// assert!(!node.remove_child("c"));
    /// ```
    pub fn remove_child(&mut self, name: &str) -> bool {
        if let Some(child) = self.child_mut(name) {
            child.remove();
            true
        } else {
            false
        }
    }

    /// Removes the node by overwriting its structure with `NOP` tags.
    ///
    /// The memory previously occupied by this node, including its name,
    /// properties, and all nested child nodes, will be replaced with `NOP`
    /// tags, rendering it invisible to Device Tree iterators without
    /// requiring data to be shifted.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::Node;
    /// use dtoolkit::fdt_mut::FdtMut;
    ///
    /// # let mut dtb = include_bytes!("../../tests/dtb/test_traversal.dtb").to_vec();
    /// let mut fdt = FdtMut::from_slice(&mut dtb).unwrap();
    /// let mut parent = fdt.find_node_mut("/a/b").unwrap();
    /// let child = parent.child_mut("c").unwrap();
    /// child.remove();
    ///
    /// assert!(parent.child("c").is_none());
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the [`FdtMut`] structure was constructed using
    /// [`FdtMut::new_unchecked`] or [`FdtMut::from_raw_unchecked`] and the FDT
    /// is not valid.
    pub fn remove(self) {
        let fdt = self.data.as_read_only();
        let start = self.offset;
        let end = fdt
            .next_sibling_offset(self.offset)
            .expect("Fdt should be valid");

        self.data.replace_with_nops(start, end);
    }

    /// Returns a mutable iterator over the properties of this node.
    pub fn properties_mut(&mut self) -> FdtPropMutIter<'_, B> {
        FdtPropMutIter {
            inner: InnerPropIter::new(self.offset),
            data: self.data,
        }
    }

    /// Returns a mutable iterator over the children of this node.
    pub fn children_mut(&mut self) -> private::FdtChildMutIter<'_, B> {
        let address_space = self.as_read_only().address_space();
        private::FdtChildMutIter {
            inner: InnerChildIter::new(self.offset),
            parent_address_space: address_space,
            data: self.data,
        }
    }

    /// Returns a read only view of this node.
    #[must_use]
    pub fn as_read_only(&self) -> FdtNode<'_> {
        let fdt = self.data.as_read_only();
        FdtNode {
            fdt,
            offset: self.offset,
            parent_address_space: self.parent_address_space,
        }
    }
}

impl<B: FdtBuffer> Display for FdtNodeMut<'_, B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.as_read_only())
    }
}

impl<B: FdtBuffer> Node for FdtNodeMut<'_, B> {
    type Property<'b>
        = crate::fdt::FdtProperty<'b>
    where
        Self: 'b;
    type Child<'b>
        = FdtNode<'b>
    where
        Self: 'b;
    type Properties<'b>
        = FdtPropIter<'b>
    where
        Self: 'b;
    type Children<'b>
        = FdtChildIter<'b>
    where
        Self: 'b;
    type Name<'b>
        = &'b str
    where
        Self: 'b;

    fn name(&self) -> &str {
        self.as_read_only().name()
    }

    fn name_without_address(&self) -> &str {
        self.as_read_only().name_without_address()
    }

    fn properties(&self) -> FdtPropIter<'_> {
        self.as_read_only().properties()
    }

    fn children(&self) -> FdtChildIter<'_> {
        self.as_read_only().children()
    }
}

pub(crate) mod private {
    use crate::fdt::node::InnerChildIter;
    use crate::fdt_mut::buffer::FdtBuffer;
    use crate::fdt_mut::{FdtMut, FdtNodeMut};
    use crate::standard::AddressSpaceProperties;

    /// A mutable iterator over the children of a device tree node.
    #[derive(Debug)]
    pub struct FdtChildMutIter<'a, B: FdtBuffer> {
        pub(crate) data: &'a mut FdtMut<B>,
        pub(crate) parent_address_space: AddressSpaceProperties,
        pub(crate) inner: InnerChildIter,
    }

    impl<B: FdtBuffer> FdtChildMutIter<'_, B> {
        /// Returns the next mutable child.
        ///
        /// # Panics
        ///
        /// Panics if the underlying device tree data is invalid.
        pub fn next(&mut self) -> Option<FdtNodeMut<'_, B>> {
            let fdt = self.data.as_read_only();
            let node_offset = self.inner.next(fdt)?;
            Some(FdtNodeMut {
                offset: node_offset,
                parent_address_space: self.parent_address_space,
                data: self.data,
            })
        }
    }
}
