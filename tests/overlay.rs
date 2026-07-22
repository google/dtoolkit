// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg(feature = "write")]

use dtoolkit::fdt::Fdt;
use dtoolkit::model::DeviceTree;
use dtoolkit::model::overlay::OverlayApplier;
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
