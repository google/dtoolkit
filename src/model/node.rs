// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use alloc::borrow::ToOwned;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use indexmap::IndexMap;
use twox_hash::xxhash64;

use super::property::DeviceTreeProperty;
use crate::error::FdtParseError;
use crate::fdt::FdtNode;

/// A mutable, in-memory representation of a device tree node.
///
/// Children and properties are stored in [`IndexMap`]s, which provide O(1)
/// lookups by name while preserving insertion order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceTreeNode {
    name: String,
    pub(super) properties: IndexMap<String, DeviceTreeProperty, xxhash64::State>,
    pub(super) children: IndexMap<String, DeviceTreeNode, xxhash64::State>,
}

impl Default for DeviceTreeNode {
    fn default() -> Self {
        Self {
            name: String::new(),
            properties: IndexMap::with_hasher(default_hash_state()),
            children: IndexMap::with_hasher(default_hash_state()),
        }
    }
}

impl DeviceTreeNode {
    /// Creates a new [`DeviceTreeNode`] with the given name.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::model::DeviceTreeNode;
    /// let node = DeviceTreeNode::new("my-node");
    /// assert_eq!(node.name(), "my-node");
    /// ```
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Creates a new [`DeviceTreeNodeBuilder`] with the given name.
    #[must_use]
    pub fn builder(name: impl Into<String>) -> DeviceTreeNodeBuilder {
        DeviceTreeNodeBuilder::new(name)
    }

    /// Returns the name of this node.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::model::DeviceTreeNode;
    /// let node = DeviceTreeNode::new("my-node");
    /// assert_eq!(node.name(), "my-node");
    /// ```
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns an iterator over the properties of this node.
    pub fn properties(&self) -> impl Iterator<Item = &DeviceTreeProperty> {
        self.properties.values()
    }

    /// Returns a mutable iterator over the properties of this node.
    pub fn properties_mut(&mut self) -> impl Iterator<Item = &mut DeviceTreeProperty> {
        self.properties.values_mut()
    }

    /// Finds a property by its name and returns a reference to it.
    ///
    /// # Performance
    ///
    /// This is a constant-time operation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::model::{DeviceTreeNode, DeviceTreeProperty};
    /// let mut node = DeviceTreeNode::new("my-node");
    /// node.add_property(DeviceTreeProperty::new("my-prop", vec![1, 2, 3, 4]));
    /// let prop = node.property("my-prop").unwrap();
    /// assert_eq!(prop.value(), &[1, 2, 3, 4]);
    /// ```
    #[must_use]
    pub fn property(&self, name: &str) -> Option<&DeviceTreeProperty> {
        self.properties.get(name)
    }

    /// Finds a property by its name and returns a mutable reference to it.
    ///
    /// # Performance
    ///
    /// This is a constant-time operation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::model::{DeviceTreeNode, DeviceTreeProperty};
    /// let mut node = DeviceTreeNode::new("my-node");
    /// node.add_property(DeviceTreeProperty::new("my-prop", vec![1, 2, 3, 4]));
    /// let prop = node.property_mut("my-prop").unwrap();
    /// prop.set_value(vec![5, 6, 7, 8]);
    /// assert_eq!(prop.value(), &[5, 6, 7, 8]);
    /// ```
    #[must_use]
    pub fn property_mut(&mut self, name: &str) -> Option<&mut DeviceTreeProperty> {
        self.properties.get_mut(name)
    }

    /// Adds a property to this node.
    ///
    /// # Performance
    ///
    /// This is a constant-time operation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::model::{DeviceTreeNode, DeviceTreeProperty};
    /// let mut node = DeviceTreeNode::new("my-node");
    /// node.add_property(DeviceTreeProperty::new("my-prop", vec![1, 2, 3, 4]));
    /// assert_eq!(node.property("my-prop").unwrap().value(), &[1, 2, 3, 4]);
    /// ```
    pub fn add_property(&mut self, property: DeviceTreeProperty) {
        self.properties.insert(property.name().to_owned(), property);
    }

    /// Removes a property from this node by its name.
    ///
    /// # Performance
    ///
    /// This is a linear-time operation, as it needs to shift elements after
    /// the removed property.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::model::{DeviceTreeNode, DeviceTreeProperty};
    /// let mut node = DeviceTreeNode::new("my-node");
    /// node.add_property(DeviceTreeProperty::new("my-prop", vec![1, 2, 3, 4]));
    /// let prop = node.remove_property("my-prop").unwrap();
    /// assert_eq!(prop.value(), &[1, 2, 3, 4]);
    /// assert!(node.property("my-prop").is_none());
    /// ```
    pub fn remove_property(&mut self, name: &str) -> Option<DeviceTreeProperty> {
        self.properties.shift_remove(name)
    }

    /// Returns an iterator over the children of this node.
    pub fn children(&self) -> impl Iterator<Item = &DeviceTreeNode> {
        self.children.values()
    }

    /// Returns a mutable iterator over the children of this node.
    pub fn children_mut(&mut self) -> impl Iterator<Item = &mut DeviceTreeNode> {
        self.children.values_mut()
    }

    /// Finds a child by its name and returns a reference to it.
    ///
    /// # Performance
    ///
    /// This is a constant-time operation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::model::{DeviceTreeNode, DeviceTreeProperty};
    /// let mut node = DeviceTreeNode::new("my-node");
    /// node.add_child(DeviceTreeNode::new("child"));
    /// let child = node.child("child");
    /// assert!(child.is_some());
    /// ```
    #[must_use]
    pub fn child(&self, name: &str) -> Option<&DeviceTreeNode> {
        self.children.get(name)
    }

    /// Finds a child by its name and returns a mutable reference to it.
    ///
    /// # Performance
    ///
    /// This is a constant-time operation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::model::{DeviceTreeNode, DeviceTreeProperty};
    /// let mut node = DeviceTreeNode::new("my-node");
    /// node.add_child(DeviceTreeNode::new("child"));
    /// let child = node.child_mut("child").unwrap();
    /// child.add_property(DeviceTreeProperty::new("my-prop", vec![1, 2, 3, 4]));
    /// assert_eq!(child.property("my-prop").unwrap().value(), &[1, 2, 3, 4]);
    /// ```
    #[must_use]
    pub fn child_mut(&mut self, name: &str) -> Option<&mut DeviceTreeNode> {
        self.children.get_mut(name)
    }

    /// Adds a child to this node.
    ///
    /// # Performance
    ///
    /// This is a constant-time operation.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::model::DeviceTreeNode;
    /// let mut node = DeviceTreeNode::new("my-node");
    /// node.add_child(DeviceTreeNode::new("child"));
    /// assert_eq!(node.child("child").unwrap().name(), "child");
    /// ```
    pub fn add_child(&mut self, child: DeviceTreeNode) {
        self.children.insert(child.name().to_owned(), child);
    }

    /// Removes a child from this node by its name.
    ///
    /// # Performance
    ///
    /// This is a linear-time operation, as it needs to shift elements after
    /// the removed child.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::model::DeviceTreeNode;
    /// let mut node = DeviceTreeNode::new("my-node");
    /// node.add_child(DeviceTreeNode::new("child"));
    /// let child = node.remove_child("child").unwrap();
    /// assert_eq!(child.name(), "child");
    /// assert!(node.child("child").is_none());
    /// ```
    pub fn remove_child(&mut self, name: &str) -> Option<DeviceTreeNode> {
        self.children.shift_remove(name)
    }
}

impl<'a> TryFrom<FdtNode<'a>> for DeviceTreeNode {
    type Error = FdtParseError;

    fn try_from(node: FdtNode<'a>) -> Result<Self, Self::Error> {
        let name = node.name()?.to_string();
        let properties = node
            .properties()
            .map(|property| property?.try_into())
            .collect::<Result<Vec<DeviceTreeProperty>, _>>()?;
        let mut property_map =
            IndexMap::with_capacity_and_hasher(properties.len(), default_hash_state());
        for property in properties {
            property_map.insert(property.name().to_owned(), property);
        }

        let children_vec: Vec<DeviceTreeNode> = node
            .children()
            .map(|child| child?.try_into())
            .collect::<Result<Vec<_>, _>>()?;
        let mut children =
            IndexMap::with_capacity_and_hasher(children_vec.len(), default_hash_state());
        for child in children_vec {
            children.insert(child.name().to_owned(), child);
        }

        Ok(DeviceTreeNode {
            name,
            properties: property_map,
            children,
        })
    }
}

/// A builder for creating [`DeviceTreeNode`]s.
#[derive(Debug, Default)]
pub struct DeviceTreeNodeBuilder {
    node: DeviceTreeNode,
}

impl DeviceTreeNodeBuilder {
    fn new(name: impl Into<String>) -> Self {
        Self {
            node: DeviceTreeNode::new(name),
        }
    }

    /// Adds a property to the node.
    #[must_use]
    pub fn property(mut self, property: DeviceTreeProperty) -> Self {
        self.node.add_property(property);
        self
    }

    /// Adds a child to the node.
    #[must_use]
    pub fn child(mut self, child: DeviceTreeNode) -> Self {
        self.node.add_child(child);
        self
    }

    /// Builds the `DeviceTreeNode`.
    #[must_use]
    pub fn build(self) -> DeviceTreeNode {
        self.node
    }
}

fn default_hash_state() -> xxhash64::State {
    xxhash64::State::with_seed(0xC001_C0DE)
}
