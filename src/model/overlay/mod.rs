// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use alloc::borrow::ToOwned;
use alloc::vec::Vec;

use crate::model::overlay::error::{OverlayError, OverlayErrorKind};
use crate::model::{DeviceTree, DeviceTreeNode};

mod error;

impl DeviceTree {
    /// Applies a device tree overlay to this device tree.
    ///
    /// # Errors
    ///
    /// Returns an error if the overlay is malformed, e.g. a fragment is missing
    /// a `target-path` or `__overlay__` node.
    pub fn apply_overlay(&mut self, overlay: DeviceTree) -> Result<(), OverlayError> {
        let overlay_root = overlay.root;
        let mut phandle_map = PhandleMap::new(self)?;

        for fragment in overlay_root.children.into_values() {
            let target_path = fragment
                .property("target-path")
                .ok_or_else(|| {
                    OverlayError::new(
                        OverlayErrorKind::TargetPathNotFound,
                        fragment.name().to_owned(),
                    )
                })?
                .as_str()
                .map_err(|_err| {
                    OverlayError::new(
                        OverlayErrorKind::TargetPathInvalid,
                        fragment.name().to_owned(),
                    )
                })?;
            let target_node = self.find_node_mut(target_path).ok_or_else(|| {
                OverlayError::new(
                    OverlayErrorKind::TargetNodeNotFound,
                    fragment.name().to_owned(),
                )
            })?;

            let overlay_node = fragment
                .children
                .clone()
                .into_values()
                .find(|c| c.name() == "__overlay__")
                .ok_or_else(|| {
                    OverlayError::new(
                        OverlayErrorKind::SourceNodeNotFound,
                        fragment.name().to_owned(),
                    )
                })?;

            merge_nodes(&mut phandle_map, target_node, overlay_node)?;
        }

        Ok(())
    }
}

fn merge_nodes(
    phandle_map: &mut PhandleMap,
    existing: &mut DeviceTreeNode,
    new: DeviceTreeNode,
) -> Result<(), OverlayError> {
    for mut prop in new.properties.into_values() {
        phandle_map.fixup_property(&mut prop, existing.name())?;

        if let Some(existing_prop) = existing.property_mut(prop.name()) {
            *existing_prop = prop;
        } else {
            existing.add_property(prop);
        }
    }

    for mut child in new.children.into_values() {
        phandle_map.fixup_node(&mut child)?;

        if let Some(existing_child) = existing.child_mut(child.name()) {
            merge_nodes(phandle_map, existing_child, child)?;
        } else {
            existing.add_child(child);
        }
    }
    Ok(())
}

struct PhandleMap {
    next_phandle: u32,
    map: Vec<(u32, u32)>,
}

impl PhandleMap {
    fn new(base: &DeviceTree) -> Result<Self, OverlayError> {
        let mut max_phandle = 0;
        let mut error = Ok(());
        base.root().for_each_node(&mut |node| {
            if let Some(phandle) = node.property("phandle") {
                let Ok(phandle) = phandle.as_u32() else {
                    error = Err(OverlayError::new(
                        OverlayErrorKind::CorruptedPhandle,
                        node.name().to_owned(),
                    ));
                    return;
                };
                if phandle > max_phandle {
                    max_phandle = phandle;
                }
            }
        });
        error?;

        Ok(Self {
            next_phandle: max_phandle + 1,
            map: Vec::new(),
        })
    }

    fn fixup_node(&mut self, node: &mut DeviceTreeNode) -> Result<(), OverlayError> {
        if let Some(phandle) = node.property("phandle") {
            let phandle = phandle.as_u32().map_err(|_| {
                OverlayError::new(OverlayErrorKind::CorruptedPhandle, node.name().to_owned())
            })?;
            let new_phandle = self.next_phandle;
            self.next_phandle += 1;
            self.map.push((phandle, new_phandle));
            let node_name = node.name().to_owned();
            node.property_mut("phandle")
                .ok_or_else(|| OverlayError::new(OverlayErrorKind::PhandleNotFound, node_name))?
                .set_value(new_phandle.to_be_bytes());
        }

        let node_name = node.name().to_owned();
        for prop in node.properties_mut() {
            self.fixup_property(prop, &node_name)?;
        }

        for child in node.children_mut() {
            self.fixup_node(child)?;
        }
        Ok(())
    }

    fn fixup_property(
        &mut self,
        prop: &mut crate::model::DeviceTreeProperty,
        node_name: &str,
    ) -> Result<(), OverlayError> {
        if !prop.value().len().is_multiple_of(4) {
            return Ok(());
        }

        let mut new_value = prop.value().to_vec();
        for (old, new) in &self.map {
            for chunk in new_value.chunks_mut(4) {
                let value = u32::from_be_bytes(chunk.try_into().map_err(|_| {
                    OverlayError::new(OverlayErrorKind::CorruptedPhandle, node_name.to_owned())
                })?);
                if value == *old {
                    chunk.copy_from_slice(&new.to_be_bytes());
                }
            }
        }
        prop.set_value(new_value);
        Ok(())
    }
}

impl DeviceTreeNode {
    fn for_each_node<F>(&self, f: &mut F)
    where
        F: FnMut(&DeviceTreeNode),
    {
        f(self);
        for child in self.children() {
            child.for_each_node(f);
        }
    }
}
