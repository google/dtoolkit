// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Helper types and methods for inspecting Device Tree Overlays (DTBOs).
//!
//! This module is modeled after the [`standard`](crate::standard) module,
//! providing generic wrappers over any type implementing [`Node`] so that
//! overlay inspection works uniformly across both read-only (`FdtNode`) and
//! in-memory (`DeviceTreeNode`) representations.

use core::ops::Deref;

use crate::error::OverlayError;
use crate::{Node, Property};

/// The name of the `__symbols__` node.
pub const NODE_SYMBOLS: &str = "__symbols__";
/// The name of the `__fixups__` node.
pub const NODE_FIXUPS: &str = "__fixups__";
/// The name of the `__local_fixups__` node.
pub const NODE_LOCAL_FIXUPS: &str = "__local_fixups__";
/// The standard properties representing a phandle.
pub const PHANDLE_PROPS: [&str; 2] = ["phandle", "linux,phandle"];

/// Typed wrapper for a device tree overlay root node.
#[derive(Debug, Clone, Copy)]
pub struct Overlay<N> {
    pub(crate) node: N,
}

impl<N> Deref for Overlay<N> {
    type Target = N;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl<N: Node> Overlay<N> {
    /// Creates a new `Overlay` wrapper from a given node.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::fdt::Fdt;
    /// use dtoolkit::overlay::Overlay;
    ///
    /// # let overlay_dtb = include_bytes!("../tests/dtb/overlay_target_path_overlay.dtb");
    /// let fdt = Fdt::new(overlay_dtb).unwrap();
    /// let overlay = Overlay::new(fdt.root());
    /// ```
    #[must_use]
    pub fn new(node: N) -> Self {
        Self { node }
    }

    /// Returns an iterator over all overlay fragments in the DTBO.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::Node;
    /// use dtoolkit::fdt::Fdt;
    /// use dtoolkit::overlay::Overlay;
    ///
    /// # let overlay_dtb = include_bytes!("../tests/dtb/overlay_target_path_overlay.dtb");
    /// let fdt = Fdt::new(overlay_dtb).unwrap();
    /// let overlay = Overlay::new(fdt.root());
    /// for fragment in overlay.fragments() {
    ///     let name: &str = fragment.name_without_address().as_ref();
    ///     assert_eq!(name, "fragment");
    /// }
    /// ```
    pub fn fragments(&self) -> impl Iterator<Item = Fragment<N::Child<'_>>> {
        self.node.children().filter_map(|node| {
            if node.name_without_address().as_ref() == "fragment" {
                Some(Fragment { node })
            } else {
                None
            }
        })
    }
}

/// An overlay fragment within a DTBO blob or tree.
#[derive(Debug, Clone, Copy)]
pub struct Fragment<N> {
    pub(crate) node: N,
}

impl<N> Deref for Fragment<N> {
    type Target = N;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl<N: Node> Fragment<N> {
    /// Returns the target of this fragment (`target` phandle or `target-path`).
    ///
    /// # Errors
    ///
    /// Returns [`OverlayError::InvalidFragmentTarget`] if neither property is
    /// present or if both are present. Returns [`OverlayError::Property`] if
    /// the target property cannot be converted to a u32 or string.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::fdt::Fdt;
    /// use dtoolkit::overlay::{FragmentTarget, Overlay};
    ///
    /// # let overlay_dtb = include_bytes!("../tests/dtb/overlay_target_path_overlay.dtb");
    /// let fdt = Fdt::new(overlay_dtb).unwrap();
    /// let overlay = Overlay::new(fdt.root());
    /// let fragment = overlay.fragments().next().unwrap();
    ///
    /// match fragment.target().unwrap() {
    ///     FragmentTarget::Path(p) => {
    ///         let p_str: &str = p.as_ref();
    ///         assert_eq!(p_str, "/soc");
    ///     }
    ///     FragmentTarget::Phandle(p) => println!("Targeting phandle: {}", p),
    /// }
    /// ```
    pub fn target(
        &self,
    ) -> Result<FragmentTarget<<N::Property<'_> as Property>::Str>, OverlayError> {
        let target = self.node.property("target");
        let target_path = self.node.property("target-path");

        match (target, target_path) {
            (Some(prop), None) => Ok(FragmentTarget::Phandle(prop.as_u32()?)),
            (None, Some(prop)) => Ok(FragmentTarget::Path(prop.as_str()?)),
            _ => Err(OverlayError::InvalidFragmentTarget),
        }
    }
}

/// The target of an overlay fragment in the base tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FragmentTarget<S> {
    /// A 32-bit phandle pointing to a target node in the base tree.
    Phandle(u32),
    /// An absolute device tree path or alias/symbol pointing to a target node.
    Path(S),
}

/// A parsed location string from a `/__fixups__` property value.
///
/// Each location string specifies where a target phandle needs to be patched
/// in the format `"/path/to/node:property_name:offset"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FixupLocation<'a> {
    /// The path to the target node inside the overlay fragment.
    pub node_path: &'a str,
    /// The property name inside the target node.
    pub property_name: &'a str,
    /// The byte offset within the property value where the 32-bit big-endian
    /// phandle resides.
    pub offset: usize,
}

impl<'a> FixupLocation<'a> {
    /// Parses a location string from `/__fixups__`.
    ///
    /// # Errors
    ///
    /// Returns [`OverlayError::InvalidFixupLocation`] if the string does not
    /// have exactly two colons separating non-empty path/property
    /// components and a 4-byte aligned numeric offset.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::overlay::FixupLocation;
    ///
    /// let loc = FixupLocation::parse("/fragment@0/__overlay__/dev@1000:clocks:0").unwrap();
    /// assert_eq!(loc.node_path, "/fragment@0/__overlay__/dev@1000");
    /// assert_eq!(loc.property_name, "clocks");
    /// assert_eq!(loc.offset, 0);
    /// ```
    pub fn parse(raw: &'a str) -> Result<Self, OverlayError> {
        let mut parts = raw.rsplitn(3, ':');
        let offset_str = parts.next().ok_or(OverlayError::InvalidFixupLocation)?;
        let property_name = parts.next().ok_or(OverlayError::InvalidFixupLocation)?;
        let node_path = parts.next().ok_or(OverlayError::InvalidFixupLocation)?;

        if node_path.is_empty() || property_name.is_empty() {
            return Err(OverlayError::InvalidFixupLocation);
        }

        let offset = offset_str
            .parse::<usize>()
            .map_err(|_| OverlayError::InvalidFixupLocation)?;
        if offset % 4 != 0 {
            return Err(OverlayError::InvalidFixupLocation);
        }

        Ok(Self {
            node_path,
            property_name,
            offset,
        })
    }
}

/// Returns the phandle of a node, if present.
///
/// # Examples
///
/// ```
/// use dtoolkit::fdt::Fdt;
/// use dtoolkit::overlay::get_phandle;
///
/// # let overlay_dtb = include_bytes!("../tests/dtb/overlay_target_path_overlay.dtb");
/// let fdt = Fdt::new(overlay_dtb).unwrap();
/// let phandle = get_phandle(&fdt.root());
/// assert!(phandle.is_none());
/// ```
pub fn get_phandle<N: Node>(node: &N) -> Option<u32> {
    for prop in PHANDLE_PROPS {
        if let Some(p) = node.property(prop)
            && let Ok(val) = p.as_u32()
        {
            return Some(val);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fdt::Fdt;
    #[cfg(feature = "write")]
    use crate::model::{DeviceTreeNode, DeviceTreeProperty};

    #[test]
    fn test_parse_fixup_location() {
        let loc = FixupLocation::parse("/fragment@0/__overlay__/dev@1000:clocks:0").unwrap();
        assert_eq!(loc.node_path, "/fragment@0/__overlay__/dev@1000");
        assert_eq!(loc.property_name, "clocks");
        assert_eq!(loc.offset, 0);

        assert_eq!(
            FixupLocation::parse("invalid").unwrap_err(),
            OverlayError::InvalidFixupLocation
        );
        assert_eq!(
            FixupLocation::parse("/path:prop:2").unwrap_err(),
            OverlayError::InvalidFixupLocation
        );
    }

    #[test]
    fn read_overlay_fragments_and_targets() {
        let overlay_dtb = include_bytes!("../tests/dtb/overlay_target_path_overlay.dtb");
        let overlay_fdt = Fdt::new(overlay_dtb).unwrap();
        let overlay = Overlay {
            node: overlay_fdt.root(),
        };

        let mut fragments = overlay.fragments();
        let fragment = fragments.next().expect("Expected a fragment");
        assert!(fragments.next().is_none());

        let target = fragment.target().unwrap();
        assert_eq!(target, FragmentTarget::Path("/soc"));
    }

    #[test]
    fn read_overlay_fixups() {
        let overlay_dtb = include_bytes!("../tests/dtb/overlay_local_fixups_overlay.dtb");
        let overlay_fdt = Fdt::new(overlay_dtb).unwrap();
        let overlay = Overlay {
            node: overlay_fdt.root(),
        };

        assert!(overlay.node.child(NODE_LOCAL_FIXUPS).is_some());

        let mut fragments = overlay.fragments();
        let fragment = fragments.next().expect("Expected a fragment");
        let target = fragment.target().unwrap();
        assert_eq!(target, FragmentTarget::Path("/"));
    }

    #[test]
    fn read_overlay_symbols_and_external_fixups() {
        let overlay_dtb = include_bytes!("../tests/dtb/overlay_external_symbols_overlay.dtb");
        let overlay_fdt = Fdt::new(overlay_dtb).unwrap();
        let overlay = Overlay {
            node: overlay_fdt.root(),
        };

        assert!(overlay.node.child(NODE_SYMBOLS).is_some());
        assert!(overlay.node.child(NODE_FIXUPS).is_some());
        assert!(overlay.node.child(NODE_LOCAL_FIXUPS).is_none());

        let mut fragments = overlay.fragments();
        let fragment = fragments.next().expect("Expected a fragment");
        let target = fragment.target().unwrap();
        // This overlay uses a phandle target pointing to a resolved symbol
        assert!(matches!(target, FragmentTarget::Phandle(_)));
    }

    #[test]
    #[cfg(feature = "write")]
    fn test_get_phandle() {
        let mut node = DeviceTreeNode::new("test").unwrap();
        assert_eq!(get_phandle(&node), None);

        node.add_property(
            DeviceTreeProperty::new("linux,phandle", 42u32.to_be_bytes().to_vec()).unwrap(),
        );
        assert_eq!(get_phandle(&node), Some(42));

        // phandle should take precedence if both are present
        node.add_property(
            DeviceTreeProperty::new("phandle", 100u32.to_be_bytes().to_vec()).unwrap(),
        );
        assert_eq!(get_phandle(&node), Some(100));
    }
}
