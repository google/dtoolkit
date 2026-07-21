// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Implementation of applying the Device Tree Overlays (DTBO).

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;

use crate::error::OverlayError;
use crate::model::DeviceTreeNode;
use crate::overlay::{PHANDLE_PROPS, get_phandle};
use crate::{Node, Property};

#[allow(unused)]
fn merge_node(
    target: &mut DeviceTreeNode,
    overlay: DeviceTreeNode,
    current_path: &str,
    phandles: &mut BTreeMap<u32, String>,
    max_phandle: &mut u32,
) {
    for (_, prop) in overlay.properties {
        target.add_property(prop);
    }
    for (child_name, child) in overlay.children {
        let child_path = if current_path == "/" {
            format!("/{child_name}")
        } else {
            format!("{current_path}/{child_name}")
        };

        if let Some(target_child) = target.child_mut(&child_name) {
            merge_node(target_child, child, &child_path, phandles, max_phandle);
        } else {
            // update phandles cache for the new sub-tree being attached
            let mut tree_path = child_path.clone();
            update_phandles_recursive(&child, &mut tree_path, phandles, max_phandle);
            target.add_child(child);
        }
    }
}

#[allow(unused)]
fn update_phandles_recursive(
    node: &DeviceTreeNode,
    current_path: &mut String,
    phandles: &mut BTreeMap<u32, String>,
    max_ph: &mut u32,
) {
    if let Some(p) = get_phandle(node) {
        *max_ph = (*max_ph).max(p);
        phandles.insert(p, current_path.clone());
    }
    for child in node.children() {
        let old_len = current_path.len();
        if !current_path.ends_with('/') {
            current_path.push('/');
        }
        current_path.push_str(child.name());
        update_phandles_recursive(child, current_path, phandles, max_ph);
        current_path.truncate(old_len);
    }
}

/// Adds `offset` to all phandle properties in the given `node` and its
/// descendants.
#[allow(unused)]
fn offset_node_phandles(node: &mut DeviceTreeNode, offset: u32) -> Result<(), OverlayError> {
    for prop_name in PHANDLE_PROPS {
        if let Some(prop) = node.property_mut(prop_name)
            && let Ok(val) = (&*prop).as_u32()
        {
            let new_val = val
                .checked_add(offset)
                .filter(|x| *x != 0 && *x != u32::MAX)
                .ok_or(OverlayError::PhandleOverflow)?;
            prop.set_value(new_val.to_be_bytes().to_vec());
        }
    }
    for child in node.children_mut() {
        offset_node_phandles(child, offset)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;
    use crate::model::{DeviceTreeNode, DeviceTreeProperty};

    #[test]
    fn merge_node_works() {
        let mut target = DeviceTreeNode::builder("target")
            .unwrap()
            .property(DeviceTreeProperty::new("a", vec![1]).unwrap())
            .child(
                DeviceTreeNode::builder("child")
                    .unwrap()
                    .property(DeviceTreeProperty::new("c", vec![3]).unwrap())
                    .build(),
            )
            .build();

        let overlay = DeviceTreeNode::builder("overlay")
            .unwrap()
            .property(DeviceTreeProperty::new("b", vec![4]).unwrap())
            .child(
                DeviceTreeNode::builder("child")
                    .unwrap()
                    .property(DeviceTreeProperty::new("d", vec![5]).unwrap())
                    .build(),
            )
            .child(
                DeviceTreeNode::builder("new_child")
                    .unwrap()
                    .property(DeviceTreeProperty::new("e", vec![6]).unwrap())
                    .build(),
            )
            .build();

        let current_path = "/target".to_string();
        let mut phandles = BTreeMap::new();
        let mut max_phandle = 0;
        merge_node(
            &mut target,
            overlay,
            &current_path,
            &mut phandles,
            &mut max_phandle,
        );

        let expected = DeviceTreeNode::builder("target")
            .unwrap()
            .property(DeviceTreeProperty::new("a", vec![1]).unwrap())
            .property(DeviceTreeProperty::new("b", vec![4]).unwrap())
            .child(
                DeviceTreeNode::builder("child")
                    .unwrap()
                    .property(DeviceTreeProperty::new("c", vec![3]).unwrap())
                    .property(DeviceTreeProperty::new("d", vec![5]).unwrap())
                    .build(),
            )
            .child(
                DeviceTreeNode::builder("new_child")
                    .unwrap()
                    .property(DeviceTreeProperty::new("e", vec![6]).unwrap())
                    .build(),
            )
            .build();

        assert_eq!(target, expected);
    }

    #[test]
    fn update_phandles_recursive_works() {
        let node = DeviceTreeNode::builder("root")
            .unwrap()
            .property(DeviceTreeProperty::new("phandle", 42u32.to_be_bytes().to_vec()).unwrap())
            .child(
                DeviceTreeNode::builder("child")
                    .unwrap()
                    .property(
                        DeviceTreeProperty::new("phandle", 43u32.to_be_bytes().to_vec()).unwrap(),
                    )
                    .child(
                        DeviceTreeNode::builder("grandchild")
                            .unwrap()
                            .property(
                                DeviceTreeProperty::new("phandle", 100u32.to_be_bytes().to_vec())
                                    .unwrap(),
                            )
                            .build(),
                    )
                    .build(),
            )
            .build();

        let mut current_path = "/root".to_string();
        let mut phandles = BTreeMap::new();
        let mut max_ph = 0;

        update_phandles_recursive(&node, &mut current_path, &mut phandles, &mut max_ph);

        assert_eq!(max_ph, 100);
        assert_eq!(phandles.len(), 3);
        assert_eq!(phandles.get(&42).unwrap(), "/root");
        assert_eq!(phandles.get(&43).unwrap(), "/root/child");
        assert_eq!(phandles.get(&100).unwrap(), "/root/child/grandchild");
    }

    #[test]
    fn offset_node_phandles_works() {
        let mut node = DeviceTreeNode::builder("root")
            .unwrap()
            .property(DeviceTreeProperty::new("phandle", 42u32.to_be_bytes().to_vec()).unwrap())
            .child(
                DeviceTreeNode::builder("child")
                    .unwrap()
                    .property(
                        DeviceTreeProperty::new("linux,phandle", 43u32.to_be_bytes().to_vec())
                            .unwrap(),
                    )
                    .build(),
            )
            .build();

        offset_node_phandles(&mut node, 100).unwrap();

        let expected = DeviceTreeNode::builder("root")
            .unwrap()
            .property(DeviceTreeProperty::new("phandle", 142u32.to_be_bytes().to_vec()).unwrap())
            .child(
                DeviceTreeNode::builder("child")
                    .unwrap()
                    .property(
                        DeviceTreeProperty::new("linux,phandle", 143u32.to_be_bytes().to_vec())
                            .unwrap(),
                    )
                    .build(),
            )
            .build();

        assert_eq!(node, expected);
    }

    #[test]
    fn offset_node_phandles_overflow() {
        let mut node = DeviceTreeNode::builder("root")
            .unwrap()
            .property(DeviceTreeProperty::new("phandle", 42u32.to_be_bytes().to_vec()).unwrap())
            .build();

        offset_node_phandles(&mut node, 100).unwrap();

        // Test overflow
        let result = offset_node_phandles(&mut node, u32::MAX - 100);
        assert!(matches!(result, Err(OverlayError::PhandleOverflow)));
    }
}
