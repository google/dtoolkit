// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use dtoolkit::fdt::{Fdt, FdtNode};
use dtoolkit::standard::NodeStandard;
use dtoolkit::{Node, ToCellInt};

#[test]
fn cpus() {
    let dtb = include_bytes!("dtb/test_standard.dtb");
    let fdt = Fdt::new(dtb).unwrap();

    let all_cpus = fdt.cpus().unwrap();
    assert_eq!(all_cpus.name(), "cpus");

    let cpu_list: Vec<_> = all_cpus.cpus().collect();
    assert_eq!(cpu_list.len(), 2);

    let first_cpu = &cpu_list[0];
    assert_eq!(first_cpu.name(), "cpu@0");
    assert_eq!(first_cpu.name_without_address(), "cpu");
    let enable_method: Vec<_> = first_cpu.enable_method().unwrap().collect();
    assert_eq!(enable_method, vec!["spin-table"]);
    assert_eq!(
        first_cpu.cpu_release_addr().unwrap(),
        Some(0x1234_5678_9abc_def0)
    );

    let ids: Vec<_> = first_cpu.ids().unwrap().collect();
    assert_eq!(ids.len(), 1);
    assert_eq!(ids[0].to_int::<u32>().unwrap(), 0);

    let second_cpu = &cpu_list[1];
    assert_eq!(second_cpu.name(), "cpu@1");
    assert!(second_cpu.enable_method().is_none());
    assert_eq!(second_cpu.cpu_release_addr().unwrap(), None);
    assert!(matches!(
        second_cpu.ids(),
        Err(dtoolkit::error::StandardError::CpuMissingReg)
    ));
}

#[test]
fn cpus_missing() {
    let dtb_no_cpus = include_bytes!("dtb/test.dtb");
    let fdt_no_cpus = Fdt::new(dtb_no_cpus).unwrap();

    assert!(matches!(
        fdt_no_cpus.cpus(),
        Err(dtoolkit::error::StandardError::CpusMissing)
    ));
}

#[test]
fn chosen() {
    let dtb = include_bytes!("dtb/test_standard.dtb");
    let fdt = Fdt::new(dtb).unwrap();

    let chosen = fdt.chosen().unwrap();
    assert_eq!(chosen.name(), "chosen");
    assert_eq!(
        chosen.bootargs().unwrap(),
        Some("console=ttyS0 root=/dev/ram0")
    );
    assert_eq!(chosen.stdout_path().unwrap(), Some("/soc/serial@1000"));
    assert_eq!(chosen.stdin_path().unwrap(), Some("/soc/serial@1000"));
}

#[test]
fn chosen_missing() {
    let dtb_no_chosen = include_bytes!("dtb/test.dtb");
    let fdt_no_chosen = Fdt::new(dtb_no_chosen).unwrap();

    assert!(fdt_no_chosen.chosen().is_none());
}

#[test]
fn memory() {
    let dtb = include_bytes!("dtb/test_pretty_print.dtb");
    let fdt = Fdt::new(dtb).unwrap();

    let memory = fdt.memory().unwrap();
    assert_eq!(memory.name(), "memory@80000000");
    assert!(memory.hotpluggable());
    assert!(format!("{memory}").starts_with("memory@80000000 {"));

    let ima = memory
        .initial_mapped_area()
        .unwrap()
        .unwrap()
        .next()
        .unwrap();
    assert_eq!(ima.effective_address, 0x1234);
    assert_eq!(ima.physical_address, 0x4321);
    assert_eq!(ima.size, 0x1000);

    let mut reserved = fdt.reserved_memory().unwrap();
    let shared = reserved.next().unwrap();
    assert!(format!("{shared}").starts_with("shared {"));
    assert!(shared.reusable());
    assert!(!shared.no_map());
    assert!(!shared.no_map_fixup());
    assert!(shared.alloc_ranges().unwrap().is_none());
}

#[test]
fn memory_missing() {
    let dtb_no_mem = include_bytes!("dtb/test_standard.dtb");
    let fdt_no_mem = Fdt::new(dtb_no_mem).unwrap();

    assert!(matches!(
        fdt_no_mem.memory(),
        Err(dtoolkit::error::StandardError::MemoryMissing)
    ));
    assert!(fdt_no_mem.reserved_memory().is_none());
}

#[test]
fn node_standard() {
    let dtb = include_bytes!("dtb/test_standard.dtb");
    let fdt = Fdt::new(dtb).unwrap();
    let root = fdt.root();
    let cpus_container = root.child("cpus").unwrap();

    assert_eq!(cpus_container.find_compatible("arm,cortex-a9").count(), 2);
    assert_eq!(cpus_container.find_compatible("non-existent").count(), 0);

    let cpu0 = cpus_container.child("cpu@0").unwrap();
    assert!(cpu0.is_compatible("arm,cortex-a9"));
    assert!(!cpu0.is_compatible("non-existent"));

    let l2 = cpus_container.child("l2-cache").unwrap();
    assert!(!l2.is_compatible("arm,cortex-a9"));
    assert!(<FdtNode as NodeStandard>::compatible(&l2).is_none());
    assert_eq!(<FdtNode as NodeStandard>::model(&l2).unwrap(), None);
}
