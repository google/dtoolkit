// Copyright 2026 Arm Limited and/or its affiliates <open-source-office@arm.com>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Fuzzer for testing whether `Fdt::new` or any operations invoked on a seemingly valid tree would
//! panic.

#![no_main]

use dtoolkit::{
    Node, Property,
    fdt::{Fdt, FdtNode},
    standard::NodeStandard,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(fdt) = Fdt::new(data) {
        let _ = fdt.data();
        let _ = fdt.version();
        let _ = fdt.last_comp_version();
        let _ = fdt.boot_cpuid_phys();
        fdt.memory_reservations().for_each(drop);
        let _ = fdt.root();

        if let Some(chosen) = fdt.chosen() {
            let _ = chosen.bootargs();
            let _ = chosen.stdout_path();
            let _ = chosen.stdin_path();
        }

        if let Ok(cpus) = fdt.cpus() {
            for cpu in cpus.cpus() {
                let _ = cpu.enable_method().map(|methods| methods.for_each(drop));
                let _ = cpu.cpu_release_addr();
                let _ = cpu.ids().map(|ids| ids.for_each(drop));
            }
        }

        if let Ok(memory) = fdt.memory() {
            let _ = memory
                .initial_mapped_area()
                .map(|areas| areas.map(|areas| areas.for_each(drop)));
            let _ = memory.hotpluggable();
        }

        if let Some(reserved_memory) = fdt.reserved_memory() {
            for reserved in reserved_memory {
                let _ = reserved.size();
                let _ = reserved.alignment();
                let _ = reserved.no_map();
                let _ = reserved.no_map_fixup();
                let _ = reserved.reusable();
                let _ = reserved
                    .alloc_ranges()
                    .map(|ranges| ranges.map(|ranges| ranges.for_each(drop)));
            }
        }

        walk(&fdt.root());
    }
});

fn walk(node: &FdtNode) {
    let _ = node.name();
    let _ = node
        .compatible()
        .map(|compatible| compatible.for_each(drop));
    let _ = node.model();
    let _ = node.phandle();
    let _ = node.status();
    let _ = node.address_cells();
    let _ = node.size_cells();
    let _ = node.virtual_reg();
    let _ = node.dma_coherent();
    let _ = node.reg().map(|regs| regs.map(|regs| regs.for_each(drop)));
    let _ = node
        .ranges()
        .map(|ranges| ranges.map(|ranges| ranges.for_each(drop)));
    let _ = node
        .dma_ranges()
        .map(|ranges| ranges.map(|ranges| ranges.for_each(drop)));

    for property in node.properties() {
        let _ = property.name();
        let _ = property.value();
    }

    for child in node.children() {
        walk(&child);
    }
}
