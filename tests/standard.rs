// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use dtoolkit::fdt::Fdt;
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
