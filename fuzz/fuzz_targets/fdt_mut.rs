// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Fuzzer for testing whether arbitrary sequences of `FdtMut` operations panic, leave the
//! buffer as an invalid FDT, or fail to have their expected observable effect.

#![no_main]

use arbitrary::Arbitrary;
use dtoolkit::{
    Node, Property, Tree,
    fdt::{Fdt, FdtNode},
    fdt_mut::FdtMut,
};
use libfuzzer_sys::fuzz_target;

static BASE_DTB: &[u8] = include_bytes!("../../tests/dtb/test_props.dtb");
const SPARE_BYTES: usize = 4096;
const COMMON_NAMES: [&str; 5] = ["a", "b", "reg", "compatible", "status"];

#[derive(Arbitrary, Debug)]
enum Name {
    Common(u8),
    Raw(String),
}

impl Name {
    fn as_str(&self) -> &str {
        match self {
            Name::Common(i) => COMMON_NAMES[*i as usize % COMMON_NAMES.len()],
            Name::Raw(s) => s,
        }
    }
}

#[derive(Arbitrary, Debug)]
enum Op {
    AddProperty { node: u8, name: Name, value: Vec<u8> },
    SetValue { node: u8, name: Name, value: Vec<u8> },
    RemoveProperty { node: u8, name: Name },
    RemoveChild { node: u8, name: Name },
    RemoveNode { node: u8 },
    Compact,
}

fn collect_paths(node: &FdtNode, path: &str, out: &mut Vec<String>) {
    out.push(if path.is_empty() { "/".to_owned() } else { path.to_owned() });
    for child in node.children() {
        collect_paths(&child, &format!("{path}/{}", child.name()), out);
    }
}

fn pick_path(fdt: &FdtMut<&mut [u8]>, index: u8) -> String {
    let mut paths = Vec::new();
    let ro = fdt.as_read_only();
    collect_paths(&ro.root(), "", &mut paths);
    paths[index as usize % paths.len()].clone()
}

fn assert_valid(fdt: &FdtMut<&mut [u8]>) {
    Fdt::new(fdt.as_read_only().data()).expect("FDT must stay valid after every operation");
}

fuzz_target!(|ops: Vec<Op>| {
    let mut buf = BASE_DTB.to_vec();
    buf.resize(BASE_DTB.len() + SPARE_BYTES, 0);
    let mut fdt = FdtMut::new(buf.as_mut_slice()).expect("base DTB is valid");

    for op in ops {
        match op {
            Op::AddProperty { node, name, value } => {
                let path = pick_path(&fdt, node);
                let Some(mut n) = fdt.find_node_mut(&path) else { continue };
                let existed = n.property(name.as_str()).is_some();
                let result = n.add_property(name.as_str(), value.as_slice()).is_ok();
                if existed {
                    assert!(!result, "adding a duplicate property must fail");
                }
                if result {
                    let p = n.property(name.as_str()).expect("added property must exist");
                    assert_eq!(p.value(), value.as_slice());
                }
            }
            Op::SetValue { node, name, value } => {
                let path = pick_path(&fdt, node);
                let Some(mut n) = fdt.find_node_mut(&path) else { continue };
                let Some(mut p) = n.property_mut(name.as_str()) else { continue };
                if p.set_value(value.as_slice()).is_ok() {
                    let p = n.property(name.as_str()).expect("property must still exist");
                    assert_eq!(p.value(), value.as_slice());
                }
            }
            Op::RemoveProperty { node, name } => {
                let path = pick_path(&fdt, node);
                let Some(mut n) = fdt.find_node_mut(&path) else { continue };
                let existed = n.property(name.as_str()).is_some();
                assert_eq!(n.remove_property(name.as_str()), existed);
                assert!(n.property(name.as_str()).is_none());
            }
            Op::RemoveChild { node, name } => {
                let path = pick_path(&fdt, node);
                let Some(mut n) = fdt.find_node_mut(&path) else { continue };
                let existed = n.child(name.as_str()).is_some();
                assert_eq!(n.remove_child(name.as_str()), existed);
                assert!(n.child(name.as_str()).is_none());
            }
            Op::RemoveNode { node } => {
                let path = pick_path(&fdt, node);
                if path == "/" {
                    continue; // the root cannot be removed
                }
                let Some(n) = fdt.find_node_mut(&path) else { continue };
                let _ = n.remove();
            }
            Op::Compact => fdt.compact(),
        }
        assert_valid(&fdt);
    }
});
