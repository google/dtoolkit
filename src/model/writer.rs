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

use zerocopy::IntoBytes;

use crate::fdt::{FDT_BEGIN_NODE, FDT_END, FDT_END_NODE, FDT_MAGIC, FDT_PROP, Fdt, FdtHeader};
use crate::memreserve::MemoryReservation;
use crate::model::{DeviceTree, DeviceTreeNode, DeviceTreeProperty};

// https://devicetree-specification.readthedocs.io/en/latest/chapter5-flattened-format.html#header
const LAST_VERSION: u32 = 17;
const LAST_COMP_VERSION: u32 = 16;

pub(crate) fn to_bytes(tree: &DeviceTree) -> Vec<u8> {
    let memory_reservations = write_memory_reservations(&tree.memory_reservations);
    let (struct_block, strings_block) = write_root(tree.root());

    let off_mem_rsvmap = size_of::<FdtHeader>();
    let off_dt_struct = off_mem_rsvmap + memory_reservations.len();
    let off_dt_strings = off_dt_struct + struct_block.len();
    let totalsize = off_dt_strings + strings_block.len();

    let mut dtb = Vec::new();

    // Header
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
        size_dt_strings: u32::try_from(strings_block.len())
            .expect("size_dt_strings exceeds u32")
            .into(),
        size_dt_struct: u32::try_from(struct_block.len())
            .expect("size_dt_struct exceeds u32")
            .into(),
    };
    dtb.extend_from_slice(header.as_bytes());
    assert_eq!(
        dtb.len(),
        size_of::<FdtHeader>(),
        "invalid header size after writing"
    );

    // Memory reservations block
    dtb.extend_from_slice(&memory_reservations);

    // Struct block
    dtb.extend_from_slice(&struct_block);

    // Strings block
    dtb.extend_from_slice(&strings_block);

    dtb
}

fn write_memory_reservations(reservations: &[MemoryReservation]) -> Vec<u8> {
    let mut memory_reservations = Vec::new();
    for reservation in reservations {
        memory_reservations.extend_from_slice(&reservation.address().to_be_bytes());
        memory_reservations.extend_from_slice(&reservation.size().to_be_bytes());
    }
    memory_reservations.extend_from_slice(&0u64.to_be_bytes());
    memory_reservations.extend_from_slice(&0u64.to_be_bytes());
    memory_reservations
}

fn write_root(root_node: &DeviceTreeNode) -> (Vec<u8>, Vec<u8>) {
    let mut struct_block = Vec::new();
    let mut strings_block = Vec::new();
    let mut string_map = BTreeMap::new();

    write_node(
        &mut struct_block,
        &mut strings_block,
        &mut string_map,
        root_node,
    );
    struct_block.extend_from_slice(&FDT_END.to_be_bytes());

    (struct_block, strings_block)
}

fn write_node(
    struct_block: &mut Vec<u8>,
    strings_block: &mut Vec<u8>,
    string_map: &mut BTreeMap<String, u32>,
    node: &DeviceTreeNode,
) {
    struct_block.extend_from_slice(&FDT_BEGIN_NODE.to_be_bytes());
    struct_block.extend_from_slice(node.name().as_bytes());
    struct_block.push(0);
    align(struct_block);

    for prop in node.properties() {
        write_prop(struct_block, strings_block, string_map, prop);
    }

    for child in node.children() {
        write_node(struct_block, strings_block, string_map, child);
    }

    struct_block.extend_from_slice(&FDT_END_NODE.to_be_bytes());
}

fn write_prop(
    struct_block: &mut Vec<u8>,
    strings_block: &mut Vec<u8>,
    string_map: &mut BTreeMap<String, u32>,
    prop: &DeviceTreeProperty,
) {
    let name_offset = if let Some(offset) = string_map.get(prop.name()) {
        *offset
    } else {
        let offset = u32::try_from(strings_block.len()).expect("string block length exceeds u32");
        strings_block.extend_from_slice(prop.name().as_bytes());
        strings_block.push(0);
        string_map.insert(prop.name().to_owned(), offset);
        offset
    };

    struct_block.extend_from_slice(&FDT_PROP.to_be_bytes());
    struct_block.extend_from_slice(
        &u32::try_from(prop.value().len())
            .expect("property value length exceeds u32")
            .to_be_bytes(),
    );
    struct_block.extend_from_slice(&name_offset.to_be_bytes());
    struct_block.extend_from_slice(prop.value());
    align(struct_block);
}

fn align(vec: &mut Vec<u8>) {
    let len = vec.len();
    let new_len = Fdt::align_tag_offset(len);
    vec.resize(new_len, 0);
}
