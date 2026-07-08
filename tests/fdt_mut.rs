// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use dtoolkit::error::FdtMutError;
use dtoolkit::fdt::Fdt;
use dtoolkit::fdt_mut::FdtMut;
use dtoolkit::{Node, Property};

#[test]
fn modify_property_in_place() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let mut data = dtb.to_vec();

    let mut fdt_mut = FdtMut::from_slice(&mut data).unwrap();
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

#[test]
fn modify_property_shrink_and_grow() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let mut data = dtb.to_vec();

    let mut fdt_mut = FdtMut::from_slice(&mut data).unwrap();
    let mut node_mut = fdt_mut.find_node_mut("/test-props").unwrap();
    let mut prop_mut = node_mut.property_mut("str-prop").unwrap();

    let orig_val = b"hello world\0";
    assert_eq!((&prop_mut).value(), orig_val);

    // Shrink the value
    let short_val = b"hi\0";
    prop_mut.set_value(short_val).unwrap();
    assert_eq!((&prop_mut).value(), short_val);

    // Check it correctly parses back
    let fdt = Fdt::new(&data).unwrap();
    let node = fdt.find_node("/test-props").unwrap();
    let prop = node.property("str-prop").unwrap();
    assert_eq!(prop.as_str().unwrap(), "hi");

    // Now grow it back, since the space is now NOPs
    let mut fdt_mut = FdtMut::from_slice(&mut data).unwrap();
    let mut node_mut = fdt_mut.find_node_mut("/test-props").unwrap();
    let mut prop_mut = node_mut.property_mut("str-prop").unwrap();

    let medium_val = b"hello\0";
    prop_mut.set_value(medium_val).unwrap();
    assert_eq!((&prop_mut).value(), medium_val);

    let fdt = Fdt::new(&data).unwrap();
    let node = fdt.find_node("/test-props").unwrap();
    let prop = node.property("str-prop").unwrap();
    assert_eq!(prop.as_str().unwrap(), "hello");

    // Growing beyond the original space should fail because there are no NOPs
    let mut fdt_mut = FdtMut::from_slice(&mut data).unwrap();
    let mut node_mut = fdt_mut.find_node_mut("/test-props").unwrap();
    let mut prop_mut = node_mut.property_mut("str-prop").unwrap();

    let long_val = b"this is too long\0";
    let err = prop_mut.set_value(long_val).unwrap_err();
    assert_eq!(err, FdtMutError::ShiftingRequired);
}

#[test]
fn remove_property_via_handle() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let mut data = dtb.to_vec();

    let mut fdt_mut = FdtMut::from_slice(&mut data).unwrap();
    let mut node_mut = fdt_mut.find_node_mut("/test-props").unwrap();
    let prop_mut = node_mut.property_mut("str-prop").unwrap();
    prop_mut.remove();

    let fdt = Fdt::new(&data).unwrap();
    let node = fdt.find_node("/test-props").unwrap();
    assert!(node.property("str-prop").is_none());
    // Verify other properties remain
    assert!(node.property("u32-prop").is_some());
}

#[test]
fn remove_property_via_node() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let mut data = dtb.to_vec();

    let mut fdt_mut = FdtMut::from_slice(&mut data).unwrap();
    let mut node_mut = fdt_mut.find_node_mut("/test-props").unwrap();

    assert!(node_mut.remove_property("str-prop"));
    assert!(!node_mut.remove_property("str-prop")); // Idempotent check

    let fdt = Fdt::new(&data).unwrap();
    let node = fdt.find_node("/test-props").unwrap();
    assert!(node.property("str-prop").is_none());
    assert!(node.property("u32-prop").is_some());
}

#[cfg(feature = "alloc")]
#[test]
fn modify_property_vec_owned() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let data = dtb.to_vec();

    let mut fdt_mut = FdtMut::new(data).unwrap();
    let mut node_mut = fdt_mut.find_node_mut("/test-props").unwrap();
    let mut prop_mut = node_mut.property_mut("str-prop").unwrap();

    let new_val = b"hello there\0";
    prop_mut.set_value(new_val).unwrap();

    let fdt = fdt_mut.as_read_only();
    let node = fdt.find_node("/test-props").unwrap();
    let prop = node.property("str-prop").unwrap();
    assert_eq!(prop.as_str().unwrap(), "hello there");
}

#[cfg(feature = "arrayvec07")]
#[test]
fn modify_property_arrayvec_owned() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let mut data = arrayvec::ArrayVec::<u8, 2048>::new();
    data.try_extend_from_slice(dtb).unwrap();

    let mut fdt_mut = FdtMut::new(data).unwrap();
    let mut node_mut = fdt_mut.find_node_mut("/test-props").unwrap();
    let mut prop_mut = node_mut.property_mut("str-prop").unwrap();

    let new_val = b"hello there\0";
    prop_mut.set_value(new_val).unwrap();

    let fdt = fdt_mut.as_read_only();
    let node = fdt.find_node("/test-props").unwrap();
    let prop = node.property("str-prop").unwrap();
    assert_eq!(prop.as_str().unwrap(), "hello there");
}
