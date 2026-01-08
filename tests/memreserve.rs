// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use dtoolkit::fdt::Fdt;

#[test]
fn memreserve() {
    let dtb = include_bytes!("dtb/test_memreserve.dtb");
    let fdt = Fdt::new(dtb).unwrap();

    let reservations: Vec<_> = fdt.memory_reservations().collect();
    assert_eq!(reservations.len(), 2);
    assert_eq!(reservations[0].address(), 0x1000);
    assert_eq!(reservations[0].size(), 0x100);
    assert_eq!(reservations[1].address(), 0x2000);
    assert_eq!(reservations[1].size(), 0x200);

    let dts = fdt.to_string();
    assert!(dts.contains("/memreserve/ 0x1000 0x100;"));
    assert!(dts.contains("/memreserve/ 0x2000 0x200;"));
}
