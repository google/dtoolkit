// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use dtoolkit::fdt::Fdt;
#[cfg(feature = "write")]
use dtoolkit::model::DeviceTree;
use dtoolkit::standard::{NodeStandard, TreeStandard};
use dtoolkit::{Node, Tree};

#[test]
fn cpus_fdt() {
    let dtb = include_bytes!("dtb/test_standard.dtb");
    let fdt = Fdt::new(dtb).unwrap();
    cpus(fdt);
}

#[cfg(feature = "write")]
#[test]
fn cpus_model() {
    let dtb = include_bytes!("dtb/test_standard.dtb");
    let fdt = Fdt::new(dtb).unwrap();
    let tree = DeviceTree::from_fdt(&fdt);
    cpus(&tree);
}

fn cpus<'a, T: Tree<'a>>(tree: T) {
    let all_cpus = tree.cpus().expect("/cpus should exist");
    assert_eq!(all_cpus.name(), "cpus");

    let cpu_list: Vec<_> = all_cpus.cpus().collect();
    assert_eq!(cpu_list.len(), 2);

    let first_cpu = &cpu_list[0];
    assert_eq!(first_cpu.name(), "cpu@0");
    assert_eq!(first_cpu.name_without_address(), "cpu");
    let enable_method: Vec<_> = first_cpu
        .enable_method()
        .expect("enable-method should exist")
        .collect();
    assert_eq!(enable_method, vec!["spin-table"]);
    assert_eq!(
        first_cpu
            .cpu_release_addr()
            .expect("cpu-release-addr should be valid"),
        Some(0x1234_5678_9abc_def0)
    );

    let ids: Vec<_> = first_cpu.ids().expect("ids should be valid").collect();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0].to_int::<u32>().expect("id should fit in u32"), 0);

    let second_cpu = &cpu_list[1];
    assert_eq!(second_cpu.name(), "cpu@1");
    assert!(second_cpu.enable_method().is_none());
    assert_eq!(
        second_cpu
            .cpu_release_addr()
            .expect("cpu-release-addr should be valid"),
        None
    );
    assert!(matches!(
        second_cpu.ids(),
        Err(dtoolkit::error::StandardError::CpuMissingReg)
    ));
}

#[test]
fn cpus_missing_fdt() {
    let dtb_no_cpus = include_bytes!("dtb/test.dtb");
    let fdt_no_cpus = Fdt::new(dtb_no_cpus).unwrap();
    cpus_missing(fdt_no_cpus);
}

#[cfg(feature = "write")]
#[test]
fn cpus_missing_model() {
    let dtb_no_cpus = include_bytes!("dtb/test.dtb");
    let fdt_no_cpus = Fdt::new(dtb_no_cpus).unwrap();
    let tree_no_cpus = DeviceTree::from_fdt(&fdt_no_cpus);
    cpus_missing(&tree_no_cpus);
}

fn cpus_missing<'a, T: Tree<'a>>(tree: T) {
    assert!(matches!(
        tree.cpus(),
        Err(dtoolkit::error::StandardError::CpusMissing)
    ));
}

#[test]
fn chosen_fdt() {
    let dtb = include_bytes!("dtb/test_standard.dtb");
    let fdt = Fdt::new(dtb).unwrap();
    chosen(fdt);
}

#[cfg(feature = "write")]
#[test]
fn chosen_model() {
    let dtb = include_bytes!("dtb/test_standard.dtb");
    let fdt = Fdt::new(dtb).unwrap();
    let tree = DeviceTree::from_fdt(&fdt);
    chosen(&tree);
}

fn chosen<'a, T: Tree<'a>>(tree: T) {
    let chosen = tree.chosen().expect("/chosen should exist");
    assert_eq!(chosen.name(), "chosen");
    assert_eq!(
        chosen.bootargs().expect("bootargs should be valid"),
        Some("console=ttyS0 root=/dev/ram0")
    );
    assert_eq!(
        chosen.stdout_path().expect("stdout-path should be valid"),
        Some("/soc/serial@1000")
    );
    assert_eq!(
        chosen.stdin_path().expect("stdin-path should be valid"),
        Some("/soc/serial@1000")
    );
}

#[test]
fn chosen_missing_fdt() {
    let dtb_no_chosen = include_bytes!("dtb/test.dtb");
    let fdt_no_chosen = Fdt::new(dtb_no_chosen).unwrap();
    chosen_missing(fdt_no_chosen);
}

#[cfg(feature = "write")]
#[test]
fn chosen_missing_model() {
    let dtb_no_chosen = include_bytes!("dtb/test.dtb");
    let fdt_no_chosen = Fdt::new(dtb_no_chosen).unwrap();
    let tree_no_chosen = DeviceTree::from_fdt(&fdt_no_chosen);
    chosen_missing(&tree_no_chosen);
}

fn chosen_missing<'a, T: Tree<'a>>(tree: T) {
    assert!(tree.chosen().is_none());
}

#[test]
fn memory_fdt() {
    let dtb = include_bytes!("dtb/test_pretty_print.dtb");
    let fdt = Fdt::new(dtb).unwrap();
    memory(fdt);

    let memory = fdt.memory().unwrap();
    assert!(format!("{memory}").starts_with("memory@80000000 {"));
    let shared = fdt.reserved_memory().unwrap().next().unwrap();
    assert!(format!("{shared}").starts_with("shared {"));
}

#[cfg(feature = "write")]
#[test]
fn memory_model() {
    let dtb = include_bytes!("dtb/test_pretty_print.dtb");
    let fdt = Fdt::new(dtb).unwrap();
    let tree = DeviceTree::from_fdt(&fdt);
    memory(&tree);
}

fn memory<'a, T: Tree<'a>>(tree: T) {
    let memory = tree.memory().expect("/memory should exist");
    assert_eq!(memory.name(), "memory@80000000");
    assert!(memory.hotpluggable());

    let ima = memory
        .initial_mapped_area()
        .expect("initial-mapped-area should be valid")
        .expect("initial-mapped-area should exist")
        .next()
        .expect("initial-mapped-area should have an entry");
    assert_eq!(ima.effective_address, 0x1234);
    assert_eq!(ima.physical_address, 0x4321);
    assert_eq!(ima.size, 0x1000);

    let mut reserved = tree
        .reserved_memory()
        .expect("/reserved-memory should exist");
    let shared = reserved
        .next()
        .expect("/reserved-memory should have a child");
    assert!(shared.reusable());
    assert!(!shared.no_map());
    assert!(!shared.no_map_fixup());
    assert!(
        shared
            .alloc_ranges()
            .expect("alloc-ranges should be valid")
            .is_none()
    );
}

#[test]
fn memory_missing_fdt() {
    let dtb_no_mem = include_bytes!("dtb/test_standard.dtb");
    let fdt_no_mem = Fdt::new(dtb_no_mem).unwrap();
    memory_missing(fdt_no_mem);
}

#[cfg(feature = "write")]
#[test]
fn memory_missing_model() {
    let dtb_no_mem = include_bytes!("dtb/test_standard.dtb");
    let fdt_no_mem = Fdt::new(dtb_no_mem).unwrap();
    let tree_no_mem = DeviceTree::from_fdt(&fdt_no_mem);
    memory_missing(&tree_no_mem);
}

fn memory_missing<'a, T: Tree<'a>>(tree: T) {
    assert!(matches!(
        tree.memory(),
        Err(dtoolkit::error::StandardError::MemoryMissing)
    ));
    assert!(tree.reserved_memory().is_none());
}

#[test]
fn node_standard_fdt() {
    let dtb = include_bytes!("dtb/test_standard.dtb");
    let fdt = Fdt::new(dtb).unwrap();
    node_standard(fdt);
}

#[cfg(feature = "write")]
#[test]
fn node_standard_model() {
    let dtb = include_bytes!("dtb/test_standard.dtb");
    let fdt = Fdt::new(dtb).unwrap();
    let tree = DeviceTree::from_fdt(&fdt);
    node_standard(&tree);
}

fn node_standard<'a, T: Tree<'a>>(tree: T) {
    let root = tree.root();
    let cpus_container = root.child("cpus").expect("cpus should exist");

    assert_eq!(cpus_container.find_compatible("arm,cortex-a9").count(), 2);
    assert_eq!(cpus_container.find_compatible("non-existent").count(), 0);

    let cpu0 = cpus_container.child("cpu@0").expect("cpu@0 should exist");
    assert!(cpu0.is_compatible("arm,cortex-a9"));
    assert!(!cpu0.is_compatible("non-existent"));

    let l2 = cpus_container
        .child("l2-cache")
        .expect("l2-cache should exist");
    assert!(!l2.is_compatible("arm,cortex-a9"));
    assert!(l2.compatible().is_none());
    assert_eq!(l2.model().expect("model should be valid"), None);
}
