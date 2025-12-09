// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use dtoolkit::fdt::Fdt;
use dtoolkit::memreserve::MemoryReservation;

#[test]
fn memreserve() {
    let dtb = include_bytes!("dtb/test_memreserve.dtb");
    let fdt = Fdt::new(dtb).unwrap();

    let reservations: Result<Vec<_>, _> = fdt.memory_reservations().collect();
    let reservations = reservations.unwrap();
    assert_eq!(
        reservations,
        &[
            MemoryReservation::new(0x1000, 0x100),
            MemoryReservation::new(0x2000, 0x200)
        ]
    );

    let dts = fdt.to_string();
    assert!(dts.contains("/memreserve/ 0x1000 0x100;"));
    assert!(dts.contains("/memreserve/ 0x2000 0x200;"));
}
