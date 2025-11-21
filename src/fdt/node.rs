// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A read-only API for inspecting a device tree node.

use super::{FDT_TAGSIZE, Fdt, FdtToken};
use crate::error::FdtError;
use crate::fdt::property::{FdtPropIter, FdtProperty};

/// A node in a flattened device tree.
#[derive(Debug, Clone, Copy)]
pub struct FdtNode<'a> {
    pub(crate) fdt: &'a Fdt<'a>,
    pub(crate) offset: usize,
}

impl<'a> FdtNode<'a> {
    /// Returns the name of this node.
    ///
    /// # Examples
    ///
    /// # Errors
    ///
    /// Returns an
    /// [`FdtErrorKind::InvalidOffset`](crate::error::FdtErrorKind::InvalidOffset)
    /// if the name offset is invalid or an
    /// [`FdtErrorKind::InvalidString`](crate::error::FdtErrorKind::InvalidString) if the string at the offset is not null-terminated
    /// or contains invalid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::fdt::Fdt;
    /// # let dtb = include_bytes!("../../tests/dtb/test_children.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let root = fdt.root().unwrap();
    /// let child = root.child("child1").unwrap().unwrap();
    /// assert_eq!(child.name().unwrap(), "child1");
    /// ```
    pub fn name(&self) -> Result<&'a str, FdtError> {
        let name_offset = self.offset + FDT_TAGSIZE;
        self.fdt.string_at_offset(name_offset, None)
    }

    /// Returns a property by its name.
    ///
    /// # Performance
    ///
    /// This method iterates through all properties of the node.
    /// If you need to call this often, consider using
    /// [`DeviceTree::from_fdt`](crate::model::DeviceTree::from_fdt) or
    /// [`DeviceTreeNode::try_from`](crate::model::DeviceTreeNode::try_from)
    /// first. [`DeviceTreeNode`](crate::model::DeviceTreeNode) stores
    /// the properties in a hash map for constant-time lookup.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::fdt::Fdt;
    /// # let dtb = include_bytes!("../../tests/dtb/test_props.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let node = fdt.find_node("/test-props").unwrap().unwrap();
    /// let prop = node.property("u32-prop").unwrap().unwrap();
    /// assert_eq!(prop.name(), "u32-prop");
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if a property's name or value cannot be read.
    pub fn property(&self, name: &str) -> Result<Option<FdtProperty<'a>>, FdtError> {
        for property in self.properties() {
            let property = property?;
            if property.name() == name {
                return Ok(Some(property));
            }
        }
        Ok(None)
    }

    /// Returns an iterator over the properties of this node.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::fdt::Fdt;
    /// # let dtb = include_bytes!("../../tests/dtb/test_props.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let node = fdt.find_node("/test-props").unwrap().unwrap();
    /// let mut props = node.properties();
    /// assert_eq!(props.next().unwrap().unwrap().name(), "u32-prop");
    /// assert_eq!(props.next().unwrap().unwrap().name(), "u64-prop");
    /// assert_eq!(props.next().unwrap().unwrap().name(), "str-prop");
    /// ```
    pub fn properties(&self) -> impl Iterator<Item = Result<FdtProperty<'a>, FdtError>> + use<'a> {
        FdtPropIter::Start {
            fdt: self.fdt,
            offset: self.offset,
        }
    }

    /// Returns a child node by its name.
    ///
    /// # Performance
    ///
    /// This method's performance is linear in the number of children of this
    /// node because it iterates through the children.
    ///
    /// # Errors
    ///
    /// Returns an error if a child node's name cannot be read.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::fdt::Fdt;
    /// # let dtb = include_bytes!("../../tests/dtb/test_children.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let root = fdt.root().unwrap();
    /// let child = root.child("child1").unwrap().unwrap();
    /// assert_eq!(child.name().unwrap(), "child1");
    /// ```
    pub fn child(&self, name: &str) -> Result<Option<FdtNode<'a>>, FdtError> {
        for child in self.children() {
            let child = child?;
            if child.name()? == name {
                return Ok(Some(child));
            }
        }
        Ok(None)
    }

    /// Returns an iterator over the children of this node.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::fdt::Fdt;
    /// # let dtb = include_bytes!("../../tests/dtb/test_children.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let root = fdt.root().unwrap();
    /// let mut children = root.children();
    /// assert_eq!(children.next().unwrap().unwrap().name().unwrap(), "child1");
    /// assert_eq!(children.next().unwrap().unwrap().name().unwrap(), "child2");
    /// assert!(children.next().is_none());
    /// ```
    pub fn children(&self) -> impl Iterator<Item = Result<FdtNode<'a>, FdtError>> + use<'a> {
        FdtChildIter::Start {
            fdt: self.fdt,
            offset: self.offset,
        }
    }
}

/// An iterator over the children of a device tree node.
enum FdtChildIter<'a> {
    Start { fdt: &'a Fdt<'a>, offset: usize },
    Running { fdt: &'a Fdt<'a>, offset: usize },
    Error,
}

impl<'a> Iterator for FdtChildIter<'a> {
    type Item = Result<FdtNode<'a>, FdtError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Start { fdt, offset } => {
                let mut offset = *offset;
                offset += FDT_TAGSIZE; // Skip FDT_BEGIN_NODE
                offset = match fdt.find_string_end(offset) {
                    Ok(offset) => offset,
                    Err(e) => {
                        *self = Self::Error;
                        return Some(Err(e));
                    }
                };
                offset = Fdt::align_tag_offset(offset);
                *self = Self::Running { fdt, offset };
                self.next()
            }
            Self::Running { fdt, offset } => match Self::try_next(fdt, offset) {
                Some(Ok(val)) => Some(Ok(val)),
                Some(Err(e)) => {
                    *self = Self::Error;
                    Some(Err(e))
                }
                None => None,
            },
            Self::Error => None,
        }
    }
}

impl<'a> FdtChildIter<'a> {
    fn try_next(fdt: &'a Fdt<'a>, offset: &mut usize) -> Option<Result<FdtNode<'a>, FdtError>> {
        loop {
            let token = match fdt.read_token(*offset) {
                Ok(token) => token,
                Err(e) => return Some(Err(e)),
            };
            match token {
                FdtToken::BeginNode => {
                    let node_offset = *offset;
                    *offset = match fdt.next_sibling_offset(*offset) {
                        Ok(offset) => offset,
                        Err(e) => return Some(Err(e)),
                    };
                    return Some(Ok(FdtNode {
                        fdt,
                        offset: node_offset,
                    }));
                }
                FdtToken::Prop => {
                    *offset = match fdt.next_property_offset(*offset + FDT_TAGSIZE) {
                        Ok(offset) => offset,
                        Err(e) => return Some(Err(e)),
                    };
                }
                FdtToken::EndNode | FdtToken::End => return None,
                FdtToken::Nop => *offset += FDT_TAGSIZE,
            }
        }
    }
}
