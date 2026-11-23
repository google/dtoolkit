#[must_use]
pub(crate) fn name_without_address(name: &str) -> &str {
    if let Some((name, _)) = name.split_once('@') {
        name
    } else {
        name
    }
}

macro_rules! impl_read_only_node_methods {
    (
        node = $node:ty,
        property = $prop:ty,
        properties = $props:ty,
        children = $children:ty $(,)?
    ) => {
        /// Returns the name of this node.
        #[must_use]
        pub fn name(&self) -> &str {
            self.as_read_only().name()
        }

        /// Returns the name of this node without the unit address, if any.
        #[must_use]
        pub fn name_without_address(&self) -> &str {
            self.as_read_only().name_without_address()
        }

        /// Returns the property with the given name, if any.
        #[must_use]
        pub fn property(&self, name: &str) -> Option<$prop> {
            self.as_read_only().property(name)
        }

        /// Returns an iterator over the properties of this node.
        #[must_use]
        pub fn properties(&self) -> $props {
            self.as_read_only().properties()
        }

        /// Returns a child node by its name.
        #[must_use]
        pub fn child(&self, name: &str) -> Option<$node> {
            self.as_read_only().child(name)
        }

        /// Returns an iterator over the children of this node.
        #[must_use]
        pub fn children(&self) -> $children {
            self.as_read_only().children()
        }
    };
}

pub(crate) use impl_read_only_node_methods;
