// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A read-write, in-memory representation of a device tree.
//!
//! This module provides the [`DeviceTree`], [`DeviceTreeNode`], and
//! [`DeviceTreeProperty`] structs, which can be used to create or modify a
//! device tree in memory. The [`DeviceTree`] can then be serialized to a
//! flattened device tree blob.

use alloc::vec::Vec;
use core::fmt::Display;

use crate::error::FdtError;
use crate::fdt::Fdt;
use crate::memreserve::MemoryReservation;
use crate::writer;
mod node;
mod overlay;
mod property;
pub use node::{DeviceTreeNode, DeviceTreeNodeBuilder};
pub use property::DeviceTreeProperty;

/// A mutable, in-memory representation of a device tree.
///
/// This struct provides a high-level API for creating and modifying a device
/// tree. It can be created from scratch or by parsing an existing FDT blob.
///
/// # Examples
///
/// ```
/// # use dtoolkit::model::{DeviceTree, DeviceTreeNode};
/// let root = DeviceTreeNode::new("/");
/// let mut tree = DeviceTree::new(root);
/// tree.root_mut().add_child(DeviceTreeNode::new("child"));
/// let child = tree.find_node_mut("/child").unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceTree {
    pub(self) root: DeviceTreeNode,
    /// The memory reservations for this device tree.
    pub memory_reservations: Vec<MemoryReservation>,
}

impl DeviceTree {
    /// Creates a new `DeviceTree` with the given root node.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::model::{DeviceTree, DeviceTreeNode};
    /// let root = DeviceTreeNode::new("/");
    /// let tree = DeviceTree::new(root);
    /// ```
    #[must_use]
    pub fn new(root: DeviceTreeNode) -> Self {
        Self {
            root,
            memory_reservations: Vec::new(),
        }
    }

    /// Creates a new `DeviceTree` from a `Fdt`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::{fdt::Fdt, model::DeviceTree};
    /// # let dtb = include_bytes!("../../tests/dtb/test.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let tree = DeviceTree::from_fdt(&fdt).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the root node of the `Fdt` cannot be parsed.
    pub fn from_fdt(fdt: &Fdt<'_>) -> Result<Self, FdtError> {
        let root = DeviceTreeNode::try_from(fdt.root()?)?;
        Ok(DeviceTree {
            root,
            memory_reservations: fdt.memory_reservations().collect(),
        })
    }

    /// Serializes the `DeviceTree` to a flattened device tree blob.
    ///
    /// # Panics
    ///
    /// This may panic if any of the lengths written to the DTB (block sizes,
    /// property value length, etc.) exceed [`u32::MAX`].
    #[must_use]
    pub fn to_dtb(&self) -> Vec<u8> {
        writer::to_bytes(self)
    }

    /// Returns a reference to the root node of the device tree.
    #[must_use]
    pub fn root(&self) -> &DeviceTreeNode {
        &self.root
    }

    /// Returns a mutable reference to the root node of the device tree.
    pub fn root_mut(&mut self) -> &mut DeviceTreeNode {
        &mut self.root
    }

    /// Finds a node by its path and returns a mutable reference to it.
    ///
    /// # Performance
    ///
    /// This method traverses the device tree, but since child lookup is a
    /// constant-time operation, performance is linear in the number of path
    /// segments.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::model::{DeviceTree, DeviceTreeNode};
    /// let mut tree = DeviceTree::new(DeviceTreeNode::new("/"));
    /// tree.root_mut().add_child(DeviceTreeNode::new("child"));
    /// let child = tree.find_node_mut("/child").unwrap();
    /// assert_eq!(child.name(), "child");
    /// ```
    pub fn find_node_mut(&mut self, path: &str) -> Option<&mut DeviceTreeNode> {
        if !path.starts_with('/') {
            return None;
        }
        let mut current_node = &mut self.root;
        if path == "/" {
            return Some(current_node);
        }
        for component in path.split('/').filter(|s| !s.is_empty()) {
            match current_node.child_mut(component) {
                Some(node) => current_node = node,
                None => return None,
            }
        }
        Some(current_node)
    }
}

impl Display for DeviceTree {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Fdt::new(&self.to_dtb())
            .expect("DeviceTree::to_dtb() should always generate a valid FDT")
            .fmt(f)
    }
}
