// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg(feature = "write")]

use dtoolkit::fdt::Fdt;
use dtoolkit::model::overlay::OverlayApplier;
use dtoolkit::model::{DeviceTree, DeviceTreeNode, DeviceTreeProperty};
use dtoolkit::{Node, Property};

macro_rules! apply_overlay {
    ($base_file:expr, $overlay_file:expr) => {{
        let base_dtb = include_bytes!(concat!("dtb/", $base_file));
        let base_fdt = Fdt::new(base_dtb).unwrap();
        let mut base = DeviceTree::from_fdt(&base_fdt);

        let overlay_dtb = include_bytes!(concat!("dtb/", $overlay_file));
        let overlay_fdt = Fdt::new(overlay_dtb).unwrap();

        let mut applier = OverlayApplier::new(&mut base);
        applier.apply_overlay(&overlay_fdt).unwrap();
        base
    }};
}

#[test]
fn apply_overlay_target_path() {
    let mut base = apply_overlay!(
        "overlay_target_path_base.dtb",
        "overlay_target_path_overlay.dtb"
    );

    let soc = base.find_node_mut("/soc").unwrap();
    assert_eq!(soc.property("status").unwrap().as_str(), Ok("okay"));
    assert_eq!(soc.property("new-prop").unwrap().as_str(), Ok("foo"));
    assert!(soc.child("serial@1000").is_some());

    // Verify round-trip through serialization
    let dtb = base.to_dtb();
    let fdt = Fdt::new(&dtb).unwrap();
    let fdt_soc = fdt.find_node("/soc").unwrap();
    assert_eq!(fdt_soc.property("status").unwrap().as_str(), Ok("okay"));
}

#[test]
fn apply_overlay_with_local_fixups() {
    let mut base = apply_overlay!(
        "overlay_local_fixups_base.dtb",
        "overlay_local_fixups_overlay.dtb"
    );

    // Base had max phandle 1, so overlay phandle 1 should be relocated to 2.
    let dev = base.find_node_mut("/dev@200").unwrap();
    assert_eq!(dev.property("phandle").unwrap().as_u32().unwrap(), 2);
    assert_eq!(dev.property("clocks").unwrap().value(), &[0, 0, 0, 2]);
}

#[test]
fn apply_overlay_with_external_fixups_and_symbols() {
    let mut base = apply_overlay!(
        "overlay_external_symbols_base.dtb",
        "overlay_external_symbols_overlay.dtb"
    );

    let uart = base.find_node_mut("/soc/uart@1000").unwrap();
    assert_eq!(uart.property("status").unwrap().as_str(), Ok("okay"));

    let base_sym = base.root.child("__symbols__").unwrap();
    assert_eq!(
        base_sym.property("uart0").unwrap().as_str(),
        Ok("/soc/uart@1000")
    );
}

#[test]
fn apply_overlay_dependent_fragments() {
    // Just verify it applies without error
    let _base = apply_overlay!(
        "overlay_dependent_base.dtb",
        "overlay_dependent_overlay.dtb"
    );
}

#[test]
fn phandles_from_local_and_external_fixups_do_not_collide() {
    // Base tree:
    //   /soc { phandle = 5; }
    //   /dev                  (no phandle, exported as symbol "foo")
    //   /__symbols__ { foo = "/dev"; }
    let mut base = DeviceTree::new();
    let mut soc = DeviceTreeNode::new("soc").unwrap();
    soc.add_property(DeviceTreeProperty::new("phandle", 5u32.to_be_bytes().to_vec()).unwrap());
    base.root.add_child(soc);
    base.root.add_child(DeviceTreeNode::new("dev").unwrap());
    let mut symbols = DeviceTreeNode::new("__symbols__").unwrap();
    symbols.add_property(DeviceTreeProperty::new("foo", b"/dev\0".to_vec()).unwrap());
    base.root.add_child(symbols);

    // Overlay with both a local phandle (relocated by 5 to 6) and an external
    // fixup that requires a brand new phandle to be allocated in the base.
    let mut overlay = DeviceTree::new();
    let mut frag = DeviceTreeNode::new("fragment@0").unwrap();
    frag.add_property(DeviceTreeProperty::new("target-path", b"/soc\0".to_vec()).unwrap());
    let mut overlay_node = DeviceTreeNode::new("__overlay__").unwrap();
    let mut newnode = DeviceTreeNode::new("newnode").unwrap();
    newnode.add_property(DeviceTreeProperty::new("phandle", 1u32.to_be_bytes().to_vec()).unwrap());
    newnode.add_property(DeviceTreeProperty::new("ref", 1u32.to_be_bytes().to_vec()).unwrap());
    newnode.add_property(DeviceTreeProperty::new("clocks", 0u32.to_be_bytes().to_vec()).unwrap());
    overlay_node.add_child(newnode);
    frag.add_child(overlay_node);
    overlay.root.add_child(frag);

    let mut local_fixups = DeviceTreeNode::new("__local_fixups__").unwrap();
    let mut lf_frag = DeviceTreeNode::new("fragment@0").unwrap();
    let mut lf_overlay = DeviceTreeNode::new("__overlay__").unwrap();
    let mut lf_newnode = DeviceTreeNode::new("newnode").unwrap();
    lf_newnode.add_property(DeviceTreeProperty::new("ref", 0u32.to_be_bytes().to_vec()).unwrap());
    lf_overlay.add_child(lf_newnode);
    lf_frag.add_child(lf_overlay);
    local_fixups.add_child(lf_frag);
    overlay.root.add_child(local_fixups);

    let mut fixups = DeviceTreeNode::new("__fixups__").unwrap();
    fixups.add_property(
        DeviceTreeProperty::new(
            "foo",
            b"/fragment@0/__overlay__/newnode:clocks:0\0".to_vec(),
        )
        .unwrap(),
    );
    overlay.root.add_child(fixups);

    let mut applier = OverlayApplier::new(&mut base);
    applier.apply_overlay_tree(overlay).unwrap();

    // The relocated local phandle lands at 1 + 5 = 6, and the locally
    // referenced phandle value was updated to match.
    let newnode = base.find_node("/soc/newnode").unwrap();
    assert_eq!(newnode.property("phandle").unwrap().as_u32().unwrap(), 6);
    assert_eq!(newnode.property("ref").unwrap().value(), &[0, 0, 0, 6]);
    // The phandle allocated for the external symbol must not collide with it.
    let dev = base.find_node("/dev").unwrap();
    assert_eq!(dev.property("phandle").unwrap().as_u32().unwrap(), 7);
}

#[test]
fn phandle_added_to_existing_node_is_resolvable_by_later_overlay() {
    // Base: /soc { phandle = 5; }
    let mut base = DeviceTree::new();
    let mut soc = DeviceTreeNode::new("soc").unwrap();
    soc.add_property(DeviceTreeProperty::new("phandle", 5u32.to_be_bytes().to_vec()).unwrap());
    base.root.add_child(soc);

    // Overlay A merges a new phandle (relocated from 1 to 6) into the
    // existing /soc node.
    let mut overlay_a = DeviceTree::new();
    let mut frag_a = DeviceTreeNode::new("fragment@0").unwrap();
    frag_a.add_property(DeviceTreeProperty::new("target-path", b"/soc\0".to_vec()).unwrap());
    let mut overlay_node_a = DeviceTreeNode::new("__overlay__").unwrap();
    overlay_node_a
        .add_property(DeviceTreeProperty::new("phandle", 1u32.to_be_bytes().to_vec()).unwrap());
    frag_a.add_child(overlay_node_a);
    overlay_a.root.add_child(frag_a);

    let mut applier = OverlayApplier::new(&mut base);
    applier.apply_overlay_tree(overlay_a).unwrap();

    // Overlay B targets /soc by its newly assigned phandle.
    let mut overlay_b = DeviceTree::new();
    let mut frag_b = DeviceTreeNode::new("fragment@0").unwrap();
    frag_b.add_property(DeviceTreeProperty::new("target", 6u32.to_be_bytes().to_vec()).unwrap());
    let mut overlay_node_b = DeviceTreeNode::new("__overlay__").unwrap();
    overlay_node_b.add_property(DeviceTreeProperty::new("status", b"okay\0".to_vec()).unwrap());
    frag_b.add_child(overlay_node_b);
    overlay_b.root.add_child(frag_b);

    // Overlay B resolves /soc by the phandle (6) that overlay A merged into it.
    applier.apply_overlay_tree(overlay_b).unwrap();

    assert_eq!(
        base.find_node("/soc")
            .unwrap()
            .property("phandle")
            .unwrap()
            .as_u32()
            .unwrap(),
        6
    );
    assert_eq!(
        base.find_node("/soc")
            .unwrap()
            .property("status")
            .unwrap()
            .as_str(),
        Ok("okay")
    );
}
