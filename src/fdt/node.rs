// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A read-only API for inspecting a device tree node.

use core::fmt::{self, Display, Formatter};

use super::{FDT_TAGSIZE, Fdt, FdtToken};
use crate::fdt::property::{FdtPropIter, FdtProperty};
use crate::standard::AddressSpaceProperties;

/// A node in a flattened device tree.
#[derive(Debug, Clone, Copy)]
pub struct FdtNode<'a> {
    pub(crate) fdt: Fdt<'a>,
    pub(crate) offset: usize,
    /// The `#address-cells` and `#size-cells` properties of this node's parent
    /// node.
    pub(crate) parent_address_space: AddressSpaceProperties,
}

impl<'a> FdtNode<'a> {
    pub(crate) fn new(fdt: Fdt<'a>, offset: usize) -> Self {
        Self {
            fdt,
            offset,
            parent_address_space: AddressSpaceProperties::default(),
        }
    }

    /// Returns the name of this node.
    ///
    /// # Panics
    ///
    /// Panics if the [`Fdt`] structure was constructed using
    /// [`Fdt::new_unchecked`] or [`Fdt::from_raw_unchecked`] and the FDT is not
    /// valid.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::fdt::Fdt;
    /// # let dtb = include_bytes!("../../tests/dtb/test_children.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let root = fdt.root();
    /// let child = root.child("child1").unwrap();
    /// assert_eq!(child.name(), "child1");
    /// ```
    #[must_use]
    pub fn name(&self) -> &'a str {
        let name_offset = self.offset + FDT_TAGSIZE;
        self.fdt
            .string_at_offset(name_offset, None)
            .expect("Fdt should be valid")
    }

    /// Returns the name of this node without the unit address, if any.
    ///
    /// # Panics
    ///
    /// Panics if the [`Fdt`] structure was constructed using
    /// [`Fdt::new_unchecked`] or [`Fdt::from_raw_unchecked`] and the FDT is not
    /// valid.
    #[must_use]
    pub fn name_without_address(&self) -> &'a str {
        let name = self.name();
        if let Some((name, _)) = name.split_once('@') {
            name
        } else {
            name
        }
    }

    /// Returns a property by its name.
    ///
    /// # Performance
    ///
    /// This method iterates through all properties of the node.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::fdt::Fdt;
    /// # let dtb = include_bytes!("../../tests/dtb/test_props.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let node = fdt.find_node("/test-props").unwrap();
    /// let prop = node.property("u32-prop").unwrap();
    /// assert_eq!(prop.name(), "u32-prop");
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the [`Fdt`] structure was constructed using
    /// [`Fdt::new_unchecked`] or [`Fdt::from_raw_unchecked`] and the FDT is not
    /// valid.
    #[must_use]
    pub fn property(&self, name: &str) -> Option<FdtProperty<'a>> {
        self.properties().find(|property| property.name() == name)
    }

    /// Returns an iterator over the properties of this node.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::fdt::Fdt;
    /// # let dtb = include_bytes!("../../tests/dtb/test_props.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let node = fdt.find_node("/test-props").unwrap();
    /// let mut props = node.properties();
    /// assert_eq!(props.next().unwrap().name(), "u32-prop");
    /// assert_eq!(props.next().unwrap().name(), "u64-prop");
    /// assert_eq!(props.next().unwrap().name(), "str-prop");
    /// ```
    pub fn properties(&self) -> impl Iterator<Item = FdtProperty<'a>> + use<'a> {
        FdtPropIter::Start {
            fdt: self.fdt,
            offset: self.offset,
        }
    }

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
    /// # Performance
    ///
    /// This method's performance is linear in the number of children of this
    /// node because it iterates through the children. If you need to call this
    /// often, consider converting to a
    /// [`DeviceTreeNode`](crate::model::DeviceTreeNode) first. Child lookup
    /// on a [`DeviceTreeNode`](crate::model::DeviceTreeNode) is a
    /// constant-time operation.
    ///
    /// # Panics
    ///
    /// Panics if the [`Fdt`] structure was constructed using
    /// [`Fdt::new_unchecked`] or [`Fdt::from_raw_unchecked`] and the FDT is not
    /// valid.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::fdt::Fdt;
    /// # let dtb = include_bytes!("../../tests/dtb/test_children.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let root = fdt.root();
    /// let child = root.child("child1").unwrap();
    /// assert_eq!(child.name(), "child1");
    /// ```
    ///
    /// ```
    /// # use dtoolkit::fdt::Fdt;
    /// # let dtb = include_bytes!("../../tests/dtb/test_children.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let root = fdt.root();
    /// let child = root.child("child2").unwrap();
    /// assert_eq!(child.name(), "child2@42");
    /// let child = root.child("child2@42").unwrap();
    /// assert_eq!(child.name(), "child2@42");
    /// ```
    #[must_use]
    pub fn child(&self, name: &str) -> Option<FdtNode<'a>> {
        let include_address = name.contains('@');
        self.children().find(|&child| {
            if include_address {
                child.name() == name
            } else {
                child.name_without_address() == name
            }
        })
    }

    /// Returns an iterator over the children of this node.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::fdt::Fdt;
    /// # let dtb = include_bytes!("../../tests/dtb/test_children.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let root = fdt.root();
    /// let mut children = root.children();
    /// assert_eq!(children.next().unwrap().name(), "child1");
    /// assert_eq!(children.next().unwrap().name(), "child2@42");
    /// assert!(children.next().is_none());
    /// ```
    pub fn children(&self) -> impl Iterator<Item = FdtNode<'a>> + use<'a> {
        FdtChildIter::Start { node: *self }
    }

    pub(crate) fn fmt_recursive(&self, f: &mut Formatter, indent: usize) -> fmt::Result {
        let name = self.name();
        if name.is_empty() {
            writeln!(f, "{:indent$}/ {{", "", indent = indent)?;
        } else {
            writeln!(f, "{:indent$}{} {{", "", name, indent = indent)?;
        }

        let mut has_properties = false;
        for prop in self.properties() {
            has_properties = true;
            prop.fmt(f, indent + 4)?;
        }

        let mut first_child = true;
        for child in self.children() {
            if !first_child || has_properties {
                writeln!(f)?;
            }

            first_child = false;
            child.fmt_recursive(f, indent + 4)?;
        }

        writeln!(f, "{:indent$}}};", "", indent = indent)
    }
}

impl Display for FdtNode<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.fmt_recursive(f, 0)
    }
}

/// An iterator over the children of a device tree node.
enum FdtChildIter<'a> {
    Start {
        node: FdtNode<'a>,
    },
    Running {
        fdt: Fdt<'a>,
        offset: usize,
        address_space: AddressSpaceProperties,
    },
}

impl<'a> Iterator for FdtChildIter<'a> {
    type Item = FdtNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Start { node } => {
                let address_space = node.address_space();
                let mut offset = node.offset;
                offset += FDT_TAGSIZE; // Skip FDT_BEGIN_NODE
                offset = node
                    .fdt
                    .find_string_end(offset)
                    .expect("Fdt should be valid");
                offset = Fdt::align_tag_offset(offset);
                *self = Self::Running {
                    fdt: node.fdt,
                    offset,
                    address_space,
                };
                self.next()
            }
            Self::Running {
                fdt,
                offset,
                address_space,
            } => Self::try_next(*fdt, offset, *address_space),
        }
    }
}

impl<'a> FdtChildIter<'a> {
    fn try_next(
        fdt: Fdt<'a>,
        offset: &mut usize,
        parent_address_space: AddressSpaceProperties,
    ) -> Option<FdtNode<'a>> {
        loop {
            let token = fdt.read_token(*offset).expect("Fdt should be valid");
            match token {
                FdtToken::BeginNode => {
                    let node_offset = *offset;
                    *offset = fdt
                        .next_sibling_offset(*offset)
                        .expect("Fdt should be valid");
                    return Some(FdtNode {
                        fdt,
                        offset: node_offset,
                        parent_address_space,
                    });
                }
                FdtToken::Prop => {
                    *offset = fdt
                        .next_property_offset(*offset + FDT_TAGSIZE, false)
                        .expect("Fdt should be valid");
                }
                FdtToken::EndNode | FdtToken::End => return None,
                FdtToken::Nop => *offset += FDT_TAGSIZE,
            }
        }
    }
}
