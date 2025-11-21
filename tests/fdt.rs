// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use dtoolkit::fdt::Fdt;

#[test]
fn read_child_nodes() {
    let dtb = include_bytes!("dtb/test_children.dtb");
    let fdt = Fdt::new(dtb).unwrap();
    let root = fdt.root().unwrap();
    let mut children = root.children();

    let child1 = children.next().unwrap().unwrap();
    assert_eq!(child1.name().unwrap(), "child1");

    let child2 = children.next().unwrap().unwrap();
    assert_eq!(child2.name().unwrap(), "child2");

    assert!(children.next().is_none());
}

#[test]
fn get_child_by_name() {
    let dtb = include_bytes!("dtb/test_children.dtb");
    let fdt = Fdt::new(dtb).unwrap();
    let root = fdt.root().unwrap();

    let child1 = root.child("child1").unwrap().unwrap();
    assert_eq!(child1.name().unwrap(), "child1");

    let child2 = root.child("child2").unwrap().unwrap();
    assert_eq!(child2.name().unwrap(), "child2");

    assert!(root.child("non-existent-child").unwrap().is_none());
}

#[test]
fn find_node_by_path() {
    let dtb = include_bytes!("dtb/test_traversal.dtb");
    let fdt = Fdt::new(dtb).unwrap();

    let root = fdt.find_node("/").unwrap().unwrap();
    assert_eq!(root.name().unwrap(), "");

    let a = fdt.find_node("/a").unwrap().unwrap();
    assert_eq!(a.name().unwrap(), "a");

    let b = fdt.find_node("/a/b").unwrap().unwrap();
    assert_eq!(b.name().unwrap(), "b");

    let c = fdt.find_node("/a/b/c").unwrap().unwrap();
    assert_eq!(c.name().unwrap(), "c");

    let d = fdt.find_node("/d").unwrap().unwrap();
    assert_eq!(d.name().unwrap(), "d");

    assert!(fdt.find_node("/a/c").is_none());
    assert!(fdt.find_node("/x").is_none());
    assert!(fdt.find_node("").is_none());
}
