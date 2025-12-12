// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use alloc::borrow::ToOwned;
use alloc::collections::btree_map::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use zerocopy::{FromZeros, IntoBytes};

use crate::fdt::{FDT_BEGIN_NODE, FDT_END, FDT_END_NODE, FDT_MAGIC, FDT_PROP, Fdt, FdtHeader};
use crate::memreserve::MemoryReservation;
use crate::model::{DeviceTree, DeviceTreeNode, DeviceTreeProperty};

// https://devicetree-specification.readthedocs.io/en/latest/chapter5-flattened-format.html#header
const LAST_VERSION: u32 = 17;
const LAST_COMP_VERSION: u32 = 16;

pub(crate) fn to_bytes(tree: &DeviceTree) -> Vec<u8> {
    let mut dtb = Vec::new();

    // reserve space for header
    dtb.extend_from_slice(FdtHeader::new_zeroed().as_bytes());

    let off_mem_rsvmap = dtb.len();
    write_memory_reservations(&mut dtb, &tree.memory_reservations);

    let off_dt_struct = dtb.len();
    let mut string_map = StringMap::new();
    write_root(&mut dtb, &mut string_map, &tree.root);

    let off_dt_strings = dtb.len();
    string_map.write_string_block(&mut dtb);

    let totalsize = dtb.len();
    let size_dt_strings = totalsize - off_dt_strings;
    let size_dt_struct = off_dt_strings - off_dt_struct;

    // write header
    let header = FdtHeader {
        magic: FDT_MAGIC.into(),
        totalsize: u32::try_from(totalsize)
            .expect("totalsize exceeds u32")
            .into(),
        off_dt_struct: u32::try_from(off_dt_struct)
            .expect("off_dt_struct exceeds u32")
            .into(),
        off_dt_strings: u32::try_from(off_dt_strings)
            .expect("off_dt_strings exceeds u32")
            .into(),
        off_mem_rsvmap: u32::try_from(off_mem_rsvmap)
            .expect("off_mem_rsvmap exceeds u32")
            .into(),
        version: LAST_VERSION.into(),
        last_comp_version: LAST_COMP_VERSION.into(),
        boot_cpuid_phys: 0u32.into(),
        size_dt_strings: u32::try_from(size_dt_strings)
            .expect("size_dt_strings exceeds u32")
            .into(),
        size_dt_struct: u32::try_from(size_dt_struct)
            .expect("size_dt_struct exceeds u32")
            .into(),
    };

    header
        .write_to_prefix(&mut dtb)
        .expect("writing the header should succeed because we've allocated the space for it");

    dtb
}

fn write_memory_reservations(dtb: &mut Vec<u8>, reservations: &[MemoryReservation]) {
    for reservation in reservations {
        dtb.extend_from_slice(&reservation.address().to_be_bytes());
        dtb.extend_from_slice(&reservation.size().to_be_bytes());
    }
    dtb.extend_from_slice(&0u64.to_be_bytes());
    dtb.extend_from_slice(&0u64.to_be_bytes());
}

fn write_root(dtb: &mut Vec<u8>, string_map: &mut StringMap, root_node: &DeviceTreeNode) {
    write_node(dtb, string_map, root_node);
    dtb.extend_from_slice(&FDT_END.to_be_bytes());
}

fn write_node(dtb: &mut Vec<u8>, string_map: &mut StringMap, node: &DeviceTreeNode) {
    dtb.extend_from_slice(&FDT_BEGIN_NODE.to_be_bytes());
    dtb.extend_from_slice(node.name().as_bytes());
    dtb.push(0);
    align(dtb);

    for prop in node.properties() {
        write_prop(dtb, string_map, prop);
    }

    for child in node.children() {
        write_node(dtb, string_map, child);
    }

    dtb.extend_from_slice(&FDT_END_NODE.to_be_bytes());
}

fn write_prop(dtb: &mut Vec<u8>, string_map: &mut StringMap, prop: &DeviceTreeProperty) {
    let name_offset = string_map.get_offset(prop.name());

    dtb.extend_from_slice(&FDT_PROP.to_be_bytes());
    dtb.extend_from_slice(
        &u32::try_from(prop.value().len())
            .expect("property value length exceeds u32")
            .to_be_bytes(),
    );
    dtb.extend_from_slice(&name_offset.to_be_bytes());
    dtb.extend_from_slice(prop.value());
    align(dtb);
}

fn align(vec: &mut Vec<u8>) {
    let len = vec.len();
    let new_len = Fdt::align_tag_offset(len);
    vec.resize(new_len, 0);
}

struct StringMap {
    string_map: BTreeMap<String, u32>,
    next_offset: u32,
}

impl StringMap {
    fn new() -> Self {
        Self {
            string_map: BTreeMap::new(),
            next_offset: 0,
        }
    }

    fn get_offset(&mut self, key: &str) -> u32 {
        if let Some(&offset) = self.string_map.get(key) {
            offset
        } else {
            let offset = self.next_offset;
            self.string_map.insert(key.to_owned(), offset);
            self.next_offset = u32::try_from(self.next_offset as usize + key.len() + 1)
                .expect("string block length exceeds u32");
            offset
        }
    }

    fn write_string_block(self, dtb: &mut Vec<u8>) {
        // write the strings in the order when they appear, mimicking the behavior
        // of `dtc` (Device Tree Compiler)
        let mut items: Vec<_> = self.string_map.into_iter().collect();
        items.sort_unstable_by_key(|(_s, offset)| *offset);

        for (s, _offset) in items {
            dtb.extend_from_slice(s.as_bytes());
            dtb.push(0);
        }
    }
}
