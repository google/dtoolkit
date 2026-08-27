// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use dtoolkit::error::{BufferError, FdtMutError};
use dtoolkit::fdt::Fdt;
use dtoolkit::fdt_mut::{FdtBuffer, FdtMut, SliceBuffer};
use dtoolkit::{Node, Property};
#[cfg(feature = "alloc")]
use zerocopy::FromBytes;

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
    assert_eq!(prop.value_as::<&str>().unwrap(), "hello there");
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
    assert_eq!(prop.value_as::<&str>().unwrap(), "hi");

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
    assert_eq!(prop.value_as::<&str>().unwrap(), "hello");

    // Growing beyond the original space should fail because there are no NOPs
    let mut fdt_mut = FdtMut::from_slice(&mut data).unwrap();
    let mut node_mut = fdt_mut.find_node_mut("/test-props").unwrap();
    let mut prop_mut = node_mut.property_mut("str-prop").unwrap();

    let long_val = b"this is too long\0";
    let err = prop_mut.set_value(long_val).unwrap_err();
    assert!(matches!(err, FdtMutError::Resize(_)));
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
    assert_eq!(prop.value_as::<&str>().unwrap(), "hello there");
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
    assert_eq!(prop.value_as::<&str>().unwrap(), "hello there");
}

#[cfg(feature = "heapless09")]
#[test]
fn modify_property_heapless_owned() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let data = heapless::Vec::<u8, 2048>::from_slice(dtb).unwrap();

    let mut fdt_mut = FdtMut::new(data).unwrap();
    let mut node_mut = fdt_mut.find_node_mut("/test-props").unwrap();
    let mut prop_mut = node_mut.property_mut("str-prop").unwrap();

    let new_val = b"hello there\0";
    prop_mut.set_value(new_val).unwrap();

    let fdt = fdt_mut.as_read_only();
    let node = fdt.find_node("/test-props").unwrap();
    let prop = node.property("str-prop").unwrap();
    assert_eq!(prop.value_as::<&str>().unwrap(), "hello there");
}

#[test]
fn modify_property_grow_slice() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let mut data = dtb.to_vec();

    let (_fdt_mut, result) = modify_property_grow(SliceBuffer::new(data.as_mut_slice()).unwrap());
    let err = result.unwrap_err();

    assert!(matches!(
        err,
        FdtMutError::Resize(BufferError::OutOfSpace { .. })
    ));
}

#[test]
fn modify_property_grow_slice_with_capacity() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let mut data = [0u8; 2048];
    data[..dtb.len()].copy_from_slice(dtb);

    let (fdt_mut, result) = modify_property_grow(SliceBuffer::new(&mut data).unwrap());
    result.unwrap();

    let prop = fdt_mut
        .as_read_only()
        .find_node("/test-props")
        .unwrap()
        .property("str-prop")
        .unwrap();
    assert_eq!(prop.value(), TEST_GROW_VAL);
}

#[cfg(feature = "alloc")]
#[test]
fn modify_property_grow_vec() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let data = dtb.to_vec();

    let (fdt_mut, result) = modify_property_grow(data);
    result.unwrap();

    let prop = fdt_mut
        .as_read_only()
        .find_node("/test-props")
        .unwrap()
        .property("str-prop")
        .unwrap();
    assert_eq!(prop.value(), TEST_GROW_VAL);
}

#[cfg(feature = "arrayvec07")]
#[test]
fn modify_property_grow_arrayvec_success() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let mut data = arrayvec::ArrayVec::<u8, 2048>::new();
    data.try_extend_from_slice(dtb).unwrap();

    let (fdt_mut, result) = modify_property_grow(data);
    result.unwrap();

    let prop = fdt_mut
        .as_read_only()
        .find_node("/test-props")
        .unwrap()
        .property("str-prop")
        .unwrap();
    assert_eq!(prop.value(), TEST_GROW_VAL);
}

#[cfg(feature = "arrayvec07")]
#[test]
fn modify_property_grow_arrayvec_failure() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let mut data = arrayvec::ArrayVec::<u8, 650>::new();
    data.try_extend_from_slice(dtb).unwrap();

    let (_fdt_mut, result) = modify_property_grow(data);
    let err = result.unwrap_err();

    assert!(matches!(
        err,
        FdtMutError::Resize(BufferError::OutOfSpace {
            requested: 683,
            capacity: 650
        })
    ));
}

#[cfg(feature = "heapless09")]
#[test]
fn modify_property_grow_heapless_success() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let data = heapless::Vec::<u8, 2048>::from_slice(dtb).unwrap();

    let (fdt_mut, result) = modify_property_grow(data);
    result.unwrap();

    let prop = fdt_mut
        .as_read_only()
        .find_node("/test-props")
        .unwrap()
        .property("str-prop")
        .unwrap();
    assert_eq!(prop.value(), TEST_GROW_VAL);
}

#[cfg(feature = "heapless09")]
#[test]
fn modify_property_grow_heapless_failure() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let data = heapless::Vec::<u8, 650>::from_slice(dtb).unwrap();

    let (_fdt_mut, result) = modify_property_grow(data);
    let err = result.unwrap_err();

    assert!(matches!(
        err,
        FdtMutError::Resize(BufferError::OutOfSpace {
            requested: 683,
            capacity: 650
        })
    ));
}

const TEST_GROW_VAL: &[u8] = b"this is a much longer string than the original one\0";

fn modify_property_grow<B: FdtBuffer>(data: B) -> (FdtMut<B>, Result<(), FdtMutError>) {
    let mut fdt_mut = FdtMut::new(data).expect("FDT should be valid");
    let mut node_mut = fdt_mut
        .find_node_mut("/test-props")
        .expect("/test-props should exist");
    let mut prop_mut = node_mut
        .property_mut("str-prop")
        .expect("str-prop should exist");

    let result = prop_mut.set_value(TEST_GROW_VAL);
    (fdt_mut, result)
}

#[cfg(feature = "alloc")]
#[test]
fn compact_vec() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let data = dtb.to_vec();

    let fdt_mut = FdtMut::new(data).unwrap();

    test_compact(fdt_mut);
}

#[test]
fn compact_slice() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let mut data = dtb.to_vec();

    let fdt_mut = FdtMut::from_slice(&mut data[..]).unwrap();

    test_compact(fdt_mut);
}

#[cfg(feature = "arrayvec07")]
#[test]
fn compact_arrayvec() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let mut data = arrayvec::ArrayVec::<u8, 2048>::new();
    data.try_extend_from_slice(dtb).unwrap();

    let fdt_mut = FdtMut::new(data).unwrap();

    test_compact(fdt_mut);
}

#[cfg(feature = "heapless09")]
#[test]
fn compact_heapless() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let data = heapless::Vec::<u8, 2048>::from_slice(dtb).unwrap();

    let fdt_mut = FdtMut::new(data).unwrap();

    test_compact(fdt_mut);
}

fn test_compact<B: FdtBuffer>(mut fdt_mut: FdtMut<B>) {
    // Remove some properties to create NOPs
    let mut node = fdt_mut
        .find_node_mut("/test-props")
        .expect("the node should exist in the test data");
    assert!(node.remove_property("str-prop"));
    assert!(node.remove_property("u64-prop"));

    let size_before = fdt_mut.as_read_only().data().len();

    // Compact
    fdt_mut.compact();

    let size_after = fdt_mut.as_read_only().data().len();
    assert!(size_after < size_before);

    // Verify it still parses and remaining properties are there
    let fdt = fdt_mut.as_read_only();
    let node = fdt
        .find_node("/test-props")
        .expect("the node should exist in the test data");
    assert!(node.property("str-prop").is_none());
    assert!(node.property("u64-prop").is_none());
    assert!(node.property("u32-prop").is_some());
}

#[cfg(feature = "alloc")]
#[test]
fn compact_strips_trailing_padding() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let mut data = dtb.to_vec();

    // Manually append 50 padding bytes to the end of the blob and update totalsize
    data.extend(core::iter::repeat_n(0xff, 50));

    let (totalsize, _) = zerocopy::big_endian::U32::mut_from_prefix(&mut data[4..8]).unwrap();
    *totalsize += 50;

    let mut fdt_mut = FdtMut::new(data).unwrap();
    let size_before_compact = fdt_mut.as_read_only().data().len();

    // Remove a property to ensure the struct shrinks
    let mut node = fdt_mut.find_node_mut("/test-props").unwrap();
    assert!(node.remove_property("str-prop"));

    fdt_mut.compact();

    let fdt = fdt_mut.as_read_only();
    let size_after_compact = fdt.data().len();

    // Compact should strip the 50 byte padding AND the NOPs left by remove_property
    assert!(size_after_compact <= size_before_compact - 50);

    let node = fdt.find_node("/test-props").unwrap();
    assert!(node.property("str-prop").is_none());
    assert!(node.property("u64-prop").is_some());
}

#[test]
fn compact_slice_noop() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let mut data = dtb.to_vec();

    let mut fdt_mut = FdtMut::from_slice(&mut data[..]).unwrap();

    let size_before = fdt_mut.as_read_only().data().len();

    // Compact should succeed because there are no NOPs to remove, so no resize is
    // needed
    fdt_mut.compact();

    let size_after = fdt_mut.as_read_only().data().len();
    assert_eq!(size_after, size_before);
}

#[test]
fn iter_invalidation() {
    let dtb = include_bytes!("dtb/test_props.dtb");

    let mut fdt = FdtMut::new(dtb.to_vec()).unwrap();
    let mut node = fdt.find_node_mut("/test-props").unwrap();

    let mut props = node.properties_mut();
    let mut found = false;
    while let Some(mut prop) = props.next() {
        if (&prop).name() == "str-prop" {
            prop.set_value(b"this value is much longer than original\0")
                .unwrap();
            found = true;
            break;
        }
    }
    assert!(found);

    // ensure this doesn't panic after mutating the property
    let prop = props.next().unwrap();
    let _ = (&prop).name();
}

#[test]
fn remove_node_via_handle() {
    let dtb = include_bytes!("dtb/test_traversal.dtb");
    let mut data = dtb.to_vec();

    let mut fdt_mut = FdtMut::from_slice(&mut data).unwrap();

    assert!(fdt_mut.as_read_only().find_node("/a/b").is_some());
    assert!(fdt_mut.as_read_only().find_node("/a/b/c").is_some());

    let mut parent = fdt_mut.find_node_mut("/a/b").unwrap();
    let child = parent.child_mut("c").unwrap();
    child.remove();

    let fdt = Fdt::new(&data).unwrap();

    let node_b = fdt.find_node("/a/b").unwrap();
    assert_eq!(node_b.name(), "b");

    assert!(fdt.find_node("/a/b/c").is_none());
    assert!(node_b.child("c").is_none());
}

#[test]
fn remove_child_by_name() {
    let dtb = include_bytes!("dtb/test_traversal.dtb");
    let mut data = dtb.to_vec();

    let mut fdt_mut = FdtMut::from_slice(&mut data).unwrap();
    let mut parent = fdt_mut.find_node_mut("/a/b").unwrap();

    assert!(parent.remove_child("c"));
    assert!(!parent.remove_child("c")); // Already removed

    let fdt = Fdt::new(&data).unwrap();
    let node_b = fdt.find_node("/a/b").unwrap();
    assert!(node_b.child("c").is_none());
}

#[cfg(feature = "alloc")]
#[test]
fn add_property() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let mut fdt_mut = FdtMut::new(dtb.to_vec()).unwrap();
    let initial_size = fdt_mut.as_read_only().data().len();

    let mut node = fdt_mut.find_node_mut("/test-props").unwrap();

    let prop = node.property("str-prop").unwrap();
    let val = prop.value().to_vec();

    node.remove_property("str-prop");
    fdt_mut.compact(); // Remove NOPs to actually shrink DTB

    let mut node = fdt_mut.find_node_mut("/test-props").unwrap();
    node.add_property("str-prop", &val).unwrap();

    let final_size = fdt_mut.as_read_only().data().len();
    assert_eq!(
        initial_size, final_size,
        "size should be exactly identical after removing and adding the same property"
    );

    let node = fdt_mut.as_read_only().find_node("/test-props").unwrap();
    let str_prop = node.property("str-prop").unwrap();
    assert_eq!(str_prop.value(), val.as_slice());
}

#[cfg(feature = "alloc")]
#[test]
fn add_string_deduplication() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let mut fdt_mut = FdtMut::new(dtb.to_vec()).unwrap();
    let initial_size = fdt_mut.as_read_only().data().len();

    let mut node = fdt_mut.find_node_mut("/test-props").unwrap();
    // adding property with an existing name should not grow the dt_strings block
    node.add_property("str-prop", [1u8, 2, 3, 4]).unwrap();

    // DTB should grow exactly by the property block size (FDT_TAGSIZE * 3 + value
    // length)
    let mid_size = fdt_mut.as_read_only().data().len();
    assert_eq!(
        initial_size + 16,
        mid_size,
        "string block should not grow when reusing existing name, only structural block shifts"
    );

    // adding property with a new string name should grow the dt_strings block
    let new_name = "brand-new-property-name";
    let mut node = fdt_mut.find_node_mut("/test-props").unwrap();
    node.add_property(new_name, [1u8, 2, 3, 4]).unwrap();

    let final_size = fdt_mut.as_read_only().data().len();
    assert_eq!(
        mid_size + 16 + new_name.len() + 1,
        final_size,
        "total size must grow to align with dtb block + string length"
    );
}
