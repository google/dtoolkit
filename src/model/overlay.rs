// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Implementation of applying the Device Tree Overlays (DTBO).

use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::error::OverlayError;
use crate::fdt::Fdt;
use crate::model::{DeviceTree, DeviceTreeNode};
use crate::overlay::{Fragment, FragmentTarget, NODE_SYMBOLS, Overlay, PHANDLE_PROPS, get_phandle};
use crate::{Node, Property};

/// A utility struct for applying device tree overlays efficiently.
///
/// It caches phandle mappings to avoid repeatedly traversing the entire base
/// tree.
#[derive(Debug)]
#[allow(unused)]
pub struct OverlayApplier<'a> {
    base: &'a mut DeviceTree,
    phandles: BTreeMap<u32, String>,
    max_phandle: u32,
}

impl<'a> OverlayApplier<'a> {
    /// Creates a new `OverlayApplier` for the given base device tree.
    pub fn new(base: &'a mut DeviceTree) -> Self {
        let mut phandles = BTreeMap::new();
        let mut max_ph = 0;
        let mut current_path = String::new();
        Self::build_cache(&base.root, &mut current_path, &mut phandles, &mut max_ph);
        Self {
            base,
            phandles,
            max_phandle: max_ph,
        }
    }

    fn build_cache(
        node: &DeviceTreeNode,
        current_path: &mut String,
        phandles: &mut BTreeMap<u32, String>,
        max_ph: &mut u32,
    ) {
        if let Some(p) = get_phandle(node) {
            *max_ph = (*max_ph).max(p);
            let p_str = if current_path.is_empty() {
                "/".to_string()
            } else {
                current_path.clone()
            };
            phandles.insert(p, p_str);
        }
        for child in node.children() {
            let old_len = current_path.len();
            if !current_path.ends_with('/') {
                current_path.push('/');
            }
            current_path.push_str(child.name());
            Self::build_cache(child, current_path, phandles, max_ph);
            current_path.truncate(old_len);
        }
    }

    /// Applies a read-only Device Tree Overlay ([`Fdt`]) to the base
    /// [`DeviceTree`].
    ///
    /// # Errors
    ///
    /// Returns an error when applying the overlay fails because of a malformed
    /// overlay data.
    pub fn apply_overlay(&mut self, overlay: &Fdt<'_>) -> Result<(), OverlayError> {
        let overlay_tree = DeviceTree::from_fdt(overlay);
        self.apply_overlay_tree(overlay_tree)
    }

    /// Applies an in-memory Device Tree Overlay ([`DeviceTree`]) to the base
    /// [`DeviceTree`].
    ///
    /// # Errors
    ///
    /// Returns an error when applying the overlay fails because of a malformed
    /// overlay data.
    pub fn apply_overlay_tree(&mut self, mut overlay: DeviceTree) -> Result<(), OverlayError> {
        let overlay_view = Overlay {
            node: &overlay.root,
        };
        let mut fragment_targets = Vec::new();

        for frag in overlay_view.fragments() {
            let frag_name = frag.name().to_string();
            let target_path = self.resolve_fragment_target_path(frag)?;
            fragment_targets.push((frag_name, target_path));
        }

        for (frag_name, target_path) in &fragment_targets {
            self.merge_fragment(&mut overlay, frag_name, target_path)?;
        }

        Ok(())
    }

    fn resolve_fragment_target_path(
        &self,
        fragment: Fragment<&DeviceTreeNode>,
    ) -> Result<String, OverlayError> {
        match fragment.target()? {
            FragmentTarget::Phandle(phandle) => {
                if let Some(path) = self.phandles.get(&phandle) {
                    Ok(path.clone())
                } else {
                    Err(OverlayError::TargetNotFound(format!(
                        "phandle 0x{phandle:x}"
                    )))
                }
            }
            FragmentTarget::Path(path) => {
                if path.starts_with('/') {
                    self.base
                        .find_node(path)
                        .map(|_| path.to_string())
                        .ok_or_else(|| OverlayError::TargetNotFound(path.to_string()))
                } else {
                    if let Some(sym_node) = self.base.root.child(NODE_SYMBOLS)
                        && let Some(sym_prop) = sym_node.property(path)
                    {
                        let abs_path = sym_prop.as_str()?;
                        return Ok(abs_path.to_string());
                    }
                    if let Some(aliases_node) = self.base.root.child("aliases")
                        && let Some(alias_prop) = aliases_node.property(path)
                    {
                        let abs_path = alias_prop.as_str()?;
                        return Ok(abs_path.to_string());
                    }
                    Err(OverlayError::TargetNotFound(path.to_string()))
                }
            }
        }
    }

    fn merge_fragment(
        &mut self,
        overlay: &mut DeviceTree,
        frag_name: &str,
        target_path: &str,
    ) -> Result<(), OverlayError> {
        if let Some(mut frag_node) = overlay.root.remove_child(frag_name)
            && let Some(overlay_subnode) = frag_node.remove_child("__overlay__")
        {
            let target_node = self
                .base
                .find_node_mut(target_path)
                .ok_or_else(|| OverlayError::TargetNotFound(target_path.to_owned()))?;
            merge_node(
                target_node,
                overlay_subnode,
                target_path,
                &mut self.phandles,
                &mut self.max_phandle,
            );
        }

        Ok(())
    }
}

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
