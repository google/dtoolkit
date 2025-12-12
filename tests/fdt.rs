// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use dtoolkit::fdt::Fdt;
#[cfg(feature = "write")]
use dtoolkit::model::DeviceTree;
use dtoolkit::standard::{InitialMappedArea, Reg, Status};

#[test]
fn read_child_nodes() {
    let dtb = include_bytes!("dtb/test_children.dtb");
    let fdt = Fdt::new(dtb).unwrap();
    let root = fdt.root().unwrap();
    let mut children = root.children();

    let child1 = children.next().unwrap().unwrap();
    assert_eq!(child1.name().unwrap(), "child1");
    assert_eq!(child1.name_without_address().unwrap(), "child1");

    let child3 = children.next().unwrap().unwrap();
    assert_eq!(child3.name().unwrap(), "child2@42");
    assert_eq!(child3.name_without_address().unwrap(), "child2");

    assert!(children.next().is_none());
}

#[test]
fn name_outlives_fdt_and_node() {
    let dtb = include_bytes!("dtb/test_children.dtb");
    let name = {
        let fdt = Fdt::new(dtb).unwrap();
        let child1 = fdt.find_node("/child1").unwrap().unwrap();
        child1.name().unwrap()
    };

    assert_eq!(name, "child1");
}

#[test]
fn read_prop_values() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let fdt = Fdt::new(dtb).unwrap();
    let root = fdt.root().unwrap();
    let mut children = root.children();
    let node = children.next().unwrap().unwrap();
    assert_eq!(node.name().unwrap(), "test-props");

    let mut props = node.properties();

    let prop = props.next().unwrap().unwrap();
    assert_eq!(prop.name(), "u32-prop");
    assert_eq!(prop.as_u32().unwrap(), 0x1234_5678);

    let prop = props.next().unwrap().unwrap();
    assert_eq!(prop.name(), "u64-prop");
    assert_eq!(prop.as_u64().unwrap(), 0x1122_3344_5566_7788);

    let prop = props.next().unwrap().unwrap();
    assert_eq!(prop.name(), "str-prop");
    assert_eq!(prop.as_str().unwrap(), "hello world");

    let prop = props.next().unwrap().unwrap();
    assert_eq!(prop.name(), "str-list-prop");
    let mut str_list = prop.as_str_list();
    assert_eq!(str_list.next(), Some("first"));
    assert_eq!(str_list.next(), Some("second"));
    assert_eq!(str_list.next(), Some("third"));
    assert_eq!(str_list.next(), None);

    assert!(props.next().is_none());
}

#[test]
fn get_property_by_name() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let fdt = Fdt::new(dtb).unwrap();
    let root = fdt.root().unwrap();
    let node = root.child("test-props").unwrap().unwrap();

    let prop = node.property("u32-prop").unwrap().unwrap();
    assert_eq!(prop.name(), "u32-prop");
    assert_eq!(prop.as_u32().unwrap(), 0x1234_5678);

    let prop = node.property("str-prop").unwrap().unwrap();
    assert_eq!(prop.name(), "str-prop");
    assert_eq!(prop.as_str().unwrap(), "hello world");

    assert!(node.property("non-existent-prop").unwrap().is_none());
}

#[test]
fn standard_properties() {
    let dtb = include_bytes!("dtb/test_props.dtb");
    let fdt = Fdt::new(dtb).unwrap();
    let root = fdt.root().unwrap();
    let test_props_node = root.child("test-props").unwrap().unwrap();
    let standard_props_node = root.child("standard-props").unwrap().unwrap();

    // Default values.
    assert_eq!(test_props_node.address_cells().unwrap(), 2);
    assert_eq!(test_props_node.size_cells().unwrap(), 1);
    assert_eq!(test_props_node.status().unwrap(), Status::Okay);
    assert_eq!(test_props_node.model().unwrap(), None);
    assert!(!test_props_node.dma_coherent().unwrap());
    assert_eq!(test_props_node.phandle().unwrap(), None);
    assert_eq!(test_props_node.virtual_reg().unwrap(), None);
    assert!(test_props_node.compatible().unwrap().is_none());

    // Explicit values.
    assert_eq!(standard_props_node.address_cells().unwrap(), 8);
    assert_eq!(standard_props_node.size_cells().unwrap(), 4);
    assert_eq!(standard_props_node.status().unwrap(), Status::Fail);
    assert_eq!(standard_props_node.model().unwrap(), Some("Some Model"));
    assert!(standard_props_node.dma_coherent().unwrap());
    assert_eq!(standard_props_node.phandle().unwrap(), Some(0x1234));
    assert_eq!(standard_props_node.virtual_reg().unwrap(), Some(0xabcd));
    assert_eq!(
        standard_props_node
            .compatible()
            .unwrap()
            .unwrap()
            .collect::<Vec<_>>(),
        vec!["abc,def", "some,other"]
    );
    let reg = standard_props_node
        .reg()
        .unwrap()
        .unwrap()
        .collect::<Vec<_>>();
    assert_eq!(
        reg,
        vec![
            Reg {
                address: &[0x1234_5678.into(), 0x3000.into()],
                size: &[0.into(), 32.into()],
            },
            Reg {
                address: &[0.into(), 0xfe00.into()],
                size: &[0.into(), 256.into()],
            },
        ]
    );
    assert_eq!(reg[0].address::<u64>().unwrap(), 0x1234_5678_0000_3000);
    assert_eq!(reg[0].size::<u64>().unwrap(), 32);
    assert_eq!(reg[1].address::<u64>().unwrap(), 0xfe00);
    assert_eq!(reg[1].size::<u64>().unwrap(), 256);
}

#[test]
fn get_child_by_name() {
    let dtb = include_bytes!("dtb/test_children.dtb");
    let fdt = Fdt::new(dtb).unwrap();
    let root = fdt.root().unwrap();

    let child1 = root.child("child1").unwrap().unwrap();
    assert_eq!(child1.name().unwrap(), "child1");

    let child2 = root.child("child2").unwrap().unwrap();
    assert_eq!(child2.name().unwrap(), "child2@42");

    let child2_with_address = root.child("child2@42").unwrap().unwrap();
    assert_eq!(child2_with_address.name().unwrap(), "child2@42");

    assert!(root.child("non-existent-child").unwrap().is_none());
}

#[test]
fn children_nested() {
    let dtb = include_bytes!("dtb/test_children_nested.dtb");
    let fdt = Fdt::new(dtb).unwrap();
    let root = fdt.root().unwrap();

    for child in root.children() {
        println!("{}", child.unwrap().name().unwrap());
    }

    let children_names: Vec<_> = root
        .children()
        .map(|child| child.unwrap().name().unwrap())
        .collect();
    assert_eq!(children_names, vec!["child1", "child3"]);

    let child1 = root.child("child1").unwrap().unwrap();
    let child2 = child1.child("child2").unwrap().unwrap();
    let nested_properties: Vec<_> = child2
        .properties()
        .map(|prop| prop.unwrap().name().to_owned())
        .collect();
    assert_eq!(nested_properties, vec!["prop2"]);
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

    assert!(fdt.find_node("/a/c").unwrap().is_none());
    assert!(fdt.find_node("/x").unwrap().is_none());
    assert!(fdt.find_node("").unwrap().is_none());
}

#[test]
fn memory() {
    let dtb = include_bytes!("dtb/test_pretty_print.dtb");
    let fdt = Fdt::new(dtb).unwrap();

    let memory = fdt.memory().unwrap();
    let reg = memory.reg().unwrap().unwrap().collect::<Vec<_>>();
    assert_eq!(reg.len(), 1);
    assert_eq!(reg[0].address::<u32>().unwrap(), 0x8000_0000);
    assert_eq!(reg[0].size::<u32>().unwrap(), 0x2000_0000);
    assert_eq!(
        reg,
        vec![Reg {
            address: &[0x8000_0000.into()],
            size: &[0x2000_0000.into()]
        }]
    );
    assert!(memory.hotpluggable().unwrap());
    assert_eq!(
        memory
            .initial_mapped_area()
            .unwrap()
            .unwrap()
            .collect::<Vec<_>>(),
        vec![InitialMappedArea {
            effective_address: 0x1234,
            physical_address: 0x4321,
            size: 0x1000,
        }]
    );
}

#[macro_export]
macro_rules! load_dtb_dts_pair {
    ($name:expr) => {
        (
            include_bytes!(concat!("dtb/", $name, ".dtb")).as_slice(),
            include_str!(concat!("dts/", $name, ".dts")),
            $name,
        )
    };
}

const ALL_DT_FILES: &[(&[u8], &str, &str)] = &[
    load_dtb_dts_pair!("test_children_nested"),
    load_dtb_dts_pair!("test_children"),
    load_dtb_dts_pair!("test_memreserve"),
    load_dtb_dts_pair!("test_pretty_print"),
    load_dtb_dts_pair!("test_props"),
    load_dtb_dts_pair!("test_traversal"),
    load_dtb_dts_pair!("test"),
];

#[test]
fn pretty_print() {
    for (dtb, expected_dts, name) in ALL_DT_FILES {
        let fdt = Fdt::new(dtb).unwrap();
        let s = fdt.to_string();
        let expected = expected_dts
            // account for Windows newlines, if needed
            .replace("\r\n", "\n");
        assert_eq!(s, expected, "Mismatch for {name}");
    }
}

#[test]
#[cfg(feature = "write")]
fn round_trip() {
    for (dtb, _dts, name) in ALL_DT_FILES {
        let fdt = Fdt::new(dtb).unwrap();
        let ir = DeviceTree::from_fdt(&fdt).unwrap();
        let new_dtb = ir.to_dtb();
        assert_eq!(dtb.to_vec(), new_dtb, "Mismatch for {name}");
    }
}

#[test]
#[cfg(feature = "write")]
fn round_trip_raw() {
    for (dtb, _dts, name) in ALL_DT_FILES {
        let fdt = unsafe { Fdt::from_raw(dtb.as_ptr()).unwrap() };
        let ir = DeviceTree::from_fdt(&fdt).unwrap();
        let new_dtb = ir.to_dtb();
        assert_eq!(dtb.to_vec(), new_dtb, "Mismatch for {name}");
    }
}
