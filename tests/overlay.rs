// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![cfg(feature = "write")]

use dtoolkit::fdt::Fdt;
use dtoolkit::model::DeviceTree;

#[test]
fn test_apply_overlay() {
    let base_dtb = include_bytes!("dtb/test_overlay_base.dtb");
    let overlay_dtb = include_bytes!("dtb/test_overlay.dtb");
    let merged_dtb = include_bytes!("dtb/test_overlay_merged.dtb");

    let base_fdt = Fdt::new(base_dtb).unwrap();
    let mut base_tree = DeviceTree::from_fdt(&base_fdt).unwrap();

    let overlay_fdt = Fdt::new(overlay_dtb).unwrap();
    let overlay_tree = DeviceTree::from_fdt(&overlay_fdt).unwrap();

    base_tree.apply_overlay(overlay_tree).unwrap();

    let merged_fdt = Fdt::new(merged_dtb).unwrap();
    let merged_tree = DeviceTree::from_fdt(&merged_fdt).unwrap();

    assert_eq!(base_tree, merged_tree);
}
