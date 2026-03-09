// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use dtoolkit::fdt::Fdt;
use dtoolkit::fdt_mut::FdtMut;
use dtoolkit::{Node, Property};

#[test]
fn modify_property_in_place() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let mut data = dtb.to_vec();

    let mut fdt_mut = FdtMut::new(&mut data).unwrap();
    let mut node_mut = fdt_mut.find_node_mut("/test-props").unwrap();
    let mut prop_mut = node_mut.property_mut("str-prop").unwrap();

    // change "hello world" to "hello there" which has the same length
    let new_val = b"hello there\0";
    assert_eq!((&prop_mut).value().len(), 12);
    assert_eq!(new_val.len(), 12);

    prop_mut.set_value(new_val).unwrap();

    let fdt = Fdt::new(&data).unwrap();
    let node = fdt.find_node("/test-props").unwrap();
    let prop = node.property("str-prop").unwrap();
    assert_eq!(prop.as_str().unwrap(), "hello there");
}
