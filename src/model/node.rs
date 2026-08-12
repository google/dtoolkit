// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use alloc::borrow::ToOwned;
use alloc::string::{String, ToString};

use indexmap::IndexMap;
use twox_hash::xxhash64;

use super::property::DeviceTreeProperty;
use crate::error::ModelError;
use crate::{Node, Property};

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

impl Node for DeviceTreeNode {
    type Property<'a>
        = &'a DeviceTreeProperty
    where
        Self: 'a;
    type Name<'a>
        = &'a str
    where
        Self: 'a;
    type Child<'a>
        = &'a DeviceTreeNode
    where
        Self: 'a;
    type Properties<'a>
        = private::DeviceTreePropertyRefIter<'a>
    where
        Self: 'a;
    type Children<'a>
        = private::DeviceTreeChildIter<'a>
    where
        Self: 'a;

    fn name(&self) -> &str {
        (&self).name()
    }

    fn name_without_address(&self) -> &str {
        (&self).name_without_address()
    }

    fn properties(&self) -> private::DeviceTreePropertyRefIter<'_> {
        private::DeviceTreePropertyRefIter {
            inner: self.properties.values(),
        }
    }

    /// Finds a child by its name and returns a reference to it.
    ///
    /// # Performance
    ///
    /// This is a constant-time operation if the `name` includes a unit-address,
    /// or a linear-time operation if not.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::Node;
    /// use dtoolkit::model::{DeviceTreeNode, DeviceTreeProperty};
    ///
    /// let mut node = DeviceTreeNode::new("my-node").unwrap();
    /// node.add_child(DeviceTreeNode::new("child").unwrap());
    /// let child = node.child("child");
    /// assert!(child.is_some());
    /// ```
    fn child(&self, name: &str) -> Option<&Self> {
        if name.contains('@') {
            self.children.get(name)
        } else {
            self.children()
                .find(|child| child.name_without_address() == name)
        }
    }

    fn children(&self) -> private::DeviceTreeChildIter<'_> {
        private::DeviceTreeChildIter {
            inner: self.children.values(),
        }
    }
}

impl<'a> Node for &'a DeviceTreeNode {
    type Property<'b>
        = &'b DeviceTreeProperty
    where
        Self: 'b;
    type Name<'b>
        = &'a str
    where
        Self: 'b;
    type Child<'b>
        = &'a DeviceTreeNode
    where
        Self: 'b;
    type Properties<'b>
        = private::DeviceTreePropertyRefIter<'b>
    where
        Self: 'b;
    type Children<'b>
        = private::DeviceTreeChildIter<'a>
    where
        Self: 'b;

    fn name(&self) -> &'a str {
        &self.name
    }

    fn name_without_address(&self) -> &'a str {
        crate::util::name_without_address(self.name())
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
    /// use dtoolkit::model::{DeviceTreeNode, DeviceTreeProperty};
    /// use dtoolkit::{Node, Property};
    ///
    /// let mut node = DeviceTreeNode::new("my-node").unwrap();
    /// node.add_property(DeviceTreeProperty::new("my-prop", vec![1u8, 2, 3, 4]).unwrap());
    /// let prop = (&node).property("my-prop").unwrap();
    /// assert_eq!(prop.value(), &[1, 2, 3, 4]);
    /// ```
    fn property(&self, name: &str) -> Option<&'a DeviceTreeProperty> {
        self.properties.get(name)
    }

    fn properties(&self) -> private::DeviceTreePropertyRefIter<'a> {
        private::DeviceTreePropertyRefIter {
            inner: self.properties.values(),
        }
    }

    /// Finds a child by its name and returns a reference to it.
    ///
    /// # Performance
    ///
    /// This is a constant-time operation if the `name` includes a unit-address,
    /// or a linear-time operation if not.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::Node;
    /// use dtoolkit::model::{DeviceTreeNode, DeviceTreeProperty};
    ///
    /// let mut node = DeviceTreeNode::new("my-node").unwrap();
    /// node.add_child(DeviceTreeNode::new("child").unwrap());
    /// let child = (&node).child("child");
    /// assert!(child.is_some());
    /// ```
    fn child(&self, name: &str) -> Option<&'a DeviceTreeNode> {
        (*self).child(name)
    }

    fn children(&self) -> private::DeviceTreeChildIter<'a> {
        private::DeviceTreeChildIter {
            inner: self.children.values(),
        }
    }
}

mod private {
    use alloc::string::String;

    use crate::model::{DeviceTreeNode, DeviceTreeProperty};

    /// An iterator over the properties of a device tree node (by reference).
    #[derive(Debug, Clone)]
    pub struct DeviceTreePropertyRefIter<'a> {
        pub inner: indexmap::map::Values<'a, String, DeviceTreeProperty>,
    }

    impl<'a> Iterator for DeviceTreePropertyRefIter<'a> {
        type Item = &'a DeviceTreeProperty;

        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next()
        }
    }

    /// An iterator over the children of a device tree node.
    #[derive(Debug, Clone)]
    pub struct DeviceTreeChildIter<'a> {
        pub inner: indexmap::map::Values<'a, String, DeviceTreeNode>,
    }

    impl<'a> Iterator for DeviceTreeChildIter<'a> {
        type Item = &'a DeviceTreeNode;

        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next()
        }
    }
}

impl DeviceTreeNode {
    /// Creates a new [`DeviceTreeNode`] with the given name.
    ///
    /// # Errors
    ///
    /// Returns a [`ModelError::InvalidNodeName`] if the node name is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::Node;
    /// use dtoolkit::model::DeviceTreeNode;
    ///
    /// let node = DeviceTreeNode::new("my-node").unwrap();
    /// assert_eq!(node.name(), "my-node");
    /// ```
    pub fn new(name: impl Into<String>) -> Result<Self, ModelError> {
        let name = name.into();
        if !crate::validate::is_valid_node_name(&name) {
            return Err(ModelError::InvalidNodeName(name));
        }
        Ok(Self::new_unchecked(name))
    }

    /// Creates a new [`DeviceTreeNode`] with the given name without validation.
    #[must_use]
    pub fn new_unchecked(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Creates a new [`DeviceTreeNodeBuilder`] with the given name.
    ///
    /// # Errors
    ///
    /// Returns a [`ModelError::InvalidNodeName`] if the node name is invalid.
    pub fn builder(name: impl Into<String>) -> Result<DeviceTreeNodeBuilder, ModelError> {
        DeviceTreeNodeBuilder::new(name)
    }

    /// Returns a mutable iterator over the properties of this node.
    pub fn properties_mut(&mut self) -> impl Iterator<Item = &mut DeviceTreeProperty> {
        self.properties.values_mut()
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
    /// use dtoolkit::Property;
    /// use dtoolkit::model::{DeviceTreeNode, DeviceTreeProperty};
    ///
    /// let mut node = DeviceTreeNode::new("my-node").unwrap();
    /// node.add_property(DeviceTreeProperty::new("my-prop", vec![1u8, 2, 3, 4]).unwrap());
    /// let prop = node.property_mut("my-prop").unwrap();
    /// prop.set_value(vec![5u8, 6, 7, 8]);
    /// assert_eq!((&*prop).value(), &[5, 6, 7, 8]);
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
    /// use dtoolkit::model::{DeviceTreeNode, DeviceTreeProperty};
    /// use dtoolkit::{Node, Property};
    ///
    /// let mut node = DeviceTreeNode::new("my-node").unwrap();
    /// node.add_property(DeviceTreeProperty::new("my-prop", vec![1u8, 2, 3, 4]).unwrap());
    /// assert_eq!((&node).property("my-prop").unwrap().value(), &[1, 2, 3, 4]);
    /// ```
    pub fn add_property(&mut self, property: DeviceTreeProperty) {
        self.properties
            .insert((&property).name().to_owned(), property);
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
    /// use dtoolkit::model::{DeviceTreeNode, DeviceTreeProperty};
    /// use dtoolkit::{Node, Property};
    ///
    /// let mut node = DeviceTreeNode::new("my-node").unwrap();
    /// node.add_property(DeviceTreeProperty::new("my-prop", vec![1u8, 2, 3, 4]).unwrap());
    /// let prop = node.remove_property("my-prop").unwrap();
    /// assert_eq!((&prop).value(), &[1, 2, 3, 4]);
    /// assert!(node.property("my-prop").is_none());
    /// ```
    pub fn remove_property(&mut self, name: &str) -> Option<DeviceTreeProperty> {
        self.properties.shift_remove(name)
    }

    /// Returns a mutable iterator over the children of this node.
    pub fn children_mut(&mut self) -> impl Iterator<Item = &mut DeviceTreeNode> {
        self.children.values_mut()
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
    /// use dtoolkit::model::{DeviceTreeNode, DeviceTreeProperty};
    /// use dtoolkit::{Node, Property};
    ///
    /// let mut node = DeviceTreeNode::new("my-node").unwrap();
    /// node.add_child(DeviceTreeNode::new("child").unwrap());
    /// let child = node.child_mut("child").unwrap();
    /// child.add_property(DeviceTreeProperty::new("my-prop", vec![1u8, 2, 3, 4]).unwrap());
    /// assert_eq!(
    ///     (&*child).property("my-prop").unwrap().value(),
    ///     &[1, 2, 3, 4]
    /// );
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
    /// use dtoolkit::Node;
    /// use dtoolkit::model::DeviceTreeNode;
    ///
    /// let mut node = DeviceTreeNode::new("my-node").unwrap();
    /// node.add_child(DeviceTreeNode::new("child").unwrap());
    /// assert_eq!((&node).child("child").unwrap().name(), "child");
    /// ```
    pub fn add_child(&mut self, child: DeviceTreeNode) {
        self.children.insert(child.name.clone(), child);
    }

    /// Adds a child to this node, returning a reference to it.
    ///
    /// # Performance
    ///
    /// This is a constant-time operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::model::{DeviceTreeNode, DeviceTreeProperty};
    /// use dtoolkit::{Node, Property};
    ///
    /// let mut node = DeviceTreeNode::new("my-node").unwrap();
    /// let mut child = node.add_child_mut(DeviceTreeNode::new("child").unwrap());
    /// child.add_property(DeviceTreeProperty::new("test", "value").unwrap());
    /// assert_eq!(
    ///     (&node)
    ///         .child("child")
    ///         .unwrap()
    ///         .property("test")
    ///         .unwrap()
    ///         .value(),
    ///     b"value\0"
    /// );
    /// ```
    #[must_use]
    pub fn add_child_mut(&mut self, child: DeviceTreeNode) -> &mut DeviceTreeNode {
        self.children.entry(child.name.clone()).or_insert(child)
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
    /// use dtoolkit::Node;
    /// use dtoolkit::model::DeviceTreeNode;
    ///
    /// let mut node = DeviceTreeNode::new("my-node").unwrap();
    /// node.add_child(DeviceTreeNode::new("child").unwrap());
    /// let child = node.remove_child("child").unwrap();
    /// assert_eq!(child.name(), "child");
    /// assert!(node.child("child").is_none());
    /// ```
    pub fn remove_child(&mut self, name: &str) -> Option<DeviceTreeNode> {
        self.children.shift_remove(name)
    }
}

impl DeviceTreeNode {
    /// Creates a new [`DeviceTreeNode`] from any type that implements [`Node`].
    pub fn from_node<T: Node>(node: &T) -> Self {
        let name = node.name().as_ref().to_string();
        let mut properties = IndexMap::with_hasher(default_hash_state());
        properties.extend(node.properties().map(|prop| {
            let name = prop.name().to_owned();
            (name, DeviceTreeProperty::from_property(&prop))
        }));

        let mut children = IndexMap::with_hasher(default_hash_state());
        children.extend(node.children().map(|child| {
            let name = child.name().as_ref().to_owned();
            (name, DeviceTreeNode::from_node(&child))
        }));

        Self {
            name,
            properties,
            children,
        }
    }
}

/// A builder for creating [`DeviceTreeNode`]s.
#[derive(Debug, Default)]
pub struct DeviceTreeNodeBuilder {
    node: DeviceTreeNode,
}

impl DeviceTreeNodeBuilder {
    fn new(name: impl Into<String>) -> Result<Self, ModelError> {
        Ok(Self {
            node: DeviceTreeNode::new(name)?,
        })
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
