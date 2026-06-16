// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::fmt;
use core::fmt::{Display, Formatter};

use crate::fdt::FdtNode;
use crate::fdt::node::InnerChildIter;
use crate::fdt::node::private::FdtChildIter;
use crate::fdt::property::{FdtPropIter, InnerPropIter};
use crate::fdt_mut::property::FdtPropMutIter;
use crate::fdt_mut::{FdtMut, FdtPropertyMut};
use crate::standard::{AddressSpaceProperties, NodeStandard};
use crate::{Node, Property};

/// A mutable device tree node.
#[derive(Debug)]
pub struct FdtNodeMut<'a> {
    pub(crate) data: FdtMut<'a>,
    pub(crate) offset: usize,
    pub(crate) parent_address_space: AddressSpaceProperties,
}

impl FdtNodeMut<'_> {
    /// Returns a mutable property by its name.
    pub fn property_mut(&mut self, name: &str) -> Option<FdtPropertyMut<'_>> {
        let mut props = self.properties_mut();
        while let Some(prop) = props.next() {
            if prop.as_read_only().name() == name {
                return Some(FdtPropertyMut {
                    prop_offset: prop.prop_offset,
                    value_offset: prop.value_offset,
                    len: prop.len,
                    nameoff: prop.nameoff,
                    data: self.data.reborrow(),
                });
            }
        }
        None
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
    /// let mut fdt = FdtMut::new(&mut dtb).unwrap();
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

    /// Returns a mutable iterator over the properties of this node.
    pub fn properties_mut(&mut self) -> FdtPropMutIter<'_> {
        FdtPropMutIter {
            inner: InnerPropIter::new(self.offset),
            data: self.data.reborrow(),
        }
    }

    /// Returns a mutable iterator over the children of this node.
    pub fn children_mut(&mut self) -> private::FdtChildMutIter<'_> {
        let address_space = self.as_read_only().address_space();
        private::FdtChildMutIter {
            inner: InnerChildIter::new(self.offset),
            parent_address_space: address_space,
            data: self.data.reborrow(),
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

impl Display for FdtNodeMut<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.as_read_only())
    }
}

impl Node for FdtNodeMut<'_> {
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

mod private {
    use crate::fdt::node::InnerChildIter;
    use crate::fdt_mut::{FdtMut, FdtNodeMut};
    use crate::standard::AddressSpaceProperties;

    /// A mutable iterator over the children of a device tree node.
    #[derive(Debug)]
    pub struct FdtChildMutIter<'a> {
        pub(crate) data: FdtMut<'a>,
        pub(crate) parent_address_space: AddressSpaceProperties,
        pub(crate) inner: InnerChildIter,
    }

    impl FdtChildMutIter<'_> {
        /// Returns the next mutable child.
        ///
        /// # Panics
        ///
        /// Panics if the underlying device tree data is invalid.
        pub fn next(&mut self) -> Option<FdtNodeMut<'_>> {
            let fdt = self.data.as_read_only();
            let node_offset = self.inner.next(fdt)?;
            Some(FdtNodeMut {
                offset: node_offset,
                parent_address_space: self.parent_address_space,
                data: self.data.reborrow(),
            })
        }
    }
}
