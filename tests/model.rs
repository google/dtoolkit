// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg(feature = "write")]

use dtoolkit::model::{DeviceTree, DeviceTreeNode, DeviceTreeProperty};

#[test]
fn tree_creation() {
    let mut tree = DeviceTree::new();
    tree.root
        .add_property(DeviceTreeProperty::new("compatible", "test"));
    tree.root
        .add_property(DeviceTreeProperty::new("prop-u32", 1u32.to_be_bytes()));
    tree.root.add_child(
        DeviceTreeNode::builder("child-a")
            .property(DeviceTreeProperty::new("child-prop", "a"))
            .build(),
    );
    tree.root.add_child(
        DeviceTreeNode::builder("child-b")
            .property(DeviceTreeProperty::new("child-prop", "b"))
            .build(),
    );

    let root = &tree.root;
    assert_eq!(root.name(), "/");
    assert_eq!(root.properties().count(), 2);
    assert_eq!(root.children().count(), 2);

    let child_a = root.children().find(|c| c.name() == "child-a").unwrap();
    assert_eq!(child_a.property("child-prop").unwrap().as_str(), Ok("a"));

    let child_b = root.children().find(|c| c.name() == "child-b").unwrap();
    assert_eq!(child_b.property("child-prop").unwrap().as_str(), Ok("b"));
}

#[test]
fn tree_modification() {
    let mut tree = DeviceTree::new();

    // Add a child
    let child = DeviceTreeNode::new("child");
    tree.root.add_child(child);
    assert_eq!(tree.root.children().count(), 1);

    // Add a property to the child
    let child = tree.root.child_mut("child").unwrap();
    child.add_property(DeviceTreeProperty::new("prop", "value"));
    assert_eq!(child.properties().count(), 1);

    // Find and modify the property
    let prop = tree
        .root
        .child_mut("child")
        .unwrap()
        .property_mut("prop")
        .unwrap();
    prop.set_value("new-value".as_bytes());

    // Verify the modification
    let child = tree.root.children().find(|c| c.name() == "child").unwrap();
    assert_eq!(child.property("prop").unwrap().as_str(), Ok("new-value"));

    // Remove the property
    let child = tree.root.child_mut("child").unwrap();
    let removed_prop = child.remove_property("prop");
    assert!(removed_prop.is_some());
    assert_eq!(child.properties().count(), 0);

    // Remove the child
    let removed_child = tree.root.remove_child("child");
    assert!(removed_child.is_some());
    assert_eq!(tree.root.children().count(), 0);
}

#[test]
fn find_node_mut() {
    let mut tree = DeviceTree::new();
    tree.root.add_child(
        DeviceTreeNode::builder("child-a")
            .child(DeviceTreeNode::builder("child-a-a").build())
            .build(),
    );
    tree.root
        .add_child(DeviceTreeNode::builder("child-b").build());

    // Find a nested child and modify it
    let child_a_a = tree.find_node_mut("/child-a/child-a-a").unwrap();
    child_a_a.add_property(DeviceTreeProperty::new("prop", "value"));

    // Verify the modification
    let child_a = tree
        .root
        .children()
        .find(|c| c.name() == "child-a")
        .unwrap();
    let child_a_a = child_a
        .children()
        .find(|c| c.name() == "child-a-a")
        .unwrap();
    assert_eq!(child_a_a.property("prop").unwrap().as_str(), Ok("value"));

    // Find a non-existent node
    assert!(tree.find_node_mut("/child-a/child-c").is_none());
}

#[test]
fn device_tree_format() {
    let tree = DeviceTree::new(
        DeviceTreeNode::builder("")
            .child(
                DeviceTreeNode::builder("child-a")
                    .child(DeviceTreeNode::builder("child-a-a").build())
                    .build(),
            )
            .child(DeviceTreeNode::builder("child-b").build())
            .build(),
    );

    let fds = tree.to_string();

    assert_eq!(
        fds,
        r#"/dts-v1/;

/ {
    child-a {
        child-a-a {
        };
    };

    child-b {
    };
};
"#
    );
}
