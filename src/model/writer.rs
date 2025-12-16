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

use crate::fdt::{
    FDT_BEGIN_NODE, FDT_END, FDT_END_NODE, FDT_MAGIC, FDT_PROP, FDT_TAGSIZE, Fdt, FdtHeader,
};
use crate::memreserve::MemoryReservation;
use crate::model::{DeviceTree, DeviceTreeNode, DeviceTreeProperty};

// https://devicetree-specification.readthedocs.io/en/latest/chapter5-flattened-format.html#header
const LAST_VERSION: u32 = 17;
const LAST_COMP_VERSION: u32 = 16;

impl DeviceTree {
    /// Serializes the [`DeviceTree`] to a flattened device tree blob.
    ///
    /// # Panics
    ///
    /// This may panic if any of the lengths written to the DTB (block sizes,
    /// property value length, etc.) exceed [`u32::MAX`].
    #[must_use]
    pub fn to_dtb(&self) -> Vec<u8> {
        let mut string_map = StringMap::new();
        let header = self.generate_header(&mut string_map);

        let mut dtb = Vec::with_capacity(header.totalsize() as usize);
        dtb.extend_from_slice(header.as_bytes());

        Self::write_memory_reservations(&mut dtb, &self.memory_reservations);
        Self::write_root(&mut dtb, &string_map, &self.root);
        string_map.write_string_block(&mut dtb);

        debug_assert_eq!(
            dtb.len(),
            header.totalsize() as usize,
            "calculated buffer size was not big enough"
        );

        dtb
    }

    /// Calculate all needed sizes (so that we can pre-allocate the buffer) and
    /// return [`FdtHeader`].
    #[must_use]
    fn generate_header(&self, string_map: &mut StringMap) -> FdtHeader {
        // entries + terminator
        let mem_reservations_size =
            (self.memory_reservations.len() + 1) * size_of::<MemoryReservation>();
        // +FDT_TAGSIZE for FDT_END
        let dt_struct_size = Self::calculate_node_size(string_map, &self.root) + FDT_TAGSIZE;
        let dt_strings_size = string_map.next_offset as usize;

        let header_size = size_of::<FdtHeader>();
        let off_mem_rsvmap = header_size;
        let off_dt_struct = off_mem_rsvmap + mem_reservations_size;
        let off_dt_strings = off_dt_struct + dt_struct_size;
        let totalsize = off_dt_strings + dt_strings_size;

        let size_dt_strings = totalsize - off_dt_strings;
        let size_dt_struct = off_dt_strings - off_dt_struct;

        FdtHeader {
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
        }
    }

    fn calculate_node_size(string_map: &mut StringMap, node: &DeviceTreeNode) -> usize {
        let mut size = 0;
        size += FDT_TAGSIZE; // FDT_BEGIN_NODE

        // name + null terminator + padding
        let name_len = node.name().len() + 1;
        size += Fdt::align_tag_offset(name_len);

        for prop in node.properties() {
            size += Self::calculate_prop_size(string_map, prop);
        }

        for child in node.children() {
            size += Self::calculate_node_size(string_map, child);
        }

        size += FDT_TAGSIZE; // FDT_END_NODE
        size
    }

    fn calculate_prop_size(string_map: &mut StringMap, prop: &DeviceTreeProperty) -> usize {
        let mut size = 0;
        size += FDT_TAGSIZE; // FDT_PROP
        size += size_of::<u32>(); // len
        size += size_of::<u32>(); // nameoff

        // ensure the name is in the map
        string_map.insert(prop.name());

        // value + padding
        size += Fdt::align_tag_offset(prop.value().len());
        size
    }

    fn write_memory_reservations(dtb: &mut Vec<u8>, reservations: &[MemoryReservation]) {
        for reservation in reservations {
            dtb.extend_from_slice(reservation.as_bytes());
        }
        dtb.extend_from_slice(MemoryReservation::TERMINATOR.as_bytes());
    }

    fn write_root(dtb: &mut Vec<u8>, string_map: &StringMap, root_node: &DeviceTreeNode) {
        Self::write_node(dtb, string_map, root_node);
        dtb.extend_from_slice(&FDT_END.to_be_bytes());
    }

    fn write_node(dtb: &mut Vec<u8>, string_map: &StringMap, node: &DeviceTreeNode) {
        dtb.extend_from_slice(&FDT_BEGIN_NODE.to_be_bytes());
        dtb.extend_from_slice(node.name().as_bytes());
        dtb.push(0);
        Self::align(dtb);

        for prop in node.properties() {
            Self::write_prop(dtb, string_map, prop);
        }

        for child in node.children() {
            Self::write_node(dtb, string_map, child);
        }

        dtb.extend_from_slice(&FDT_END_NODE.to_be_bytes());
    }

    fn write_prop(dtb: &mut Vec<u8>, string_map: &StringMap, prop: &DeviceTreeProperty) {
        let name_offset = string_map.get_offset(prop.name());

        dtb.extend_from_slice(&FDT_PROP.to_be_bytes());
        dtb.extend_from_slice(
            &u32::try_from(prop.value().len())
                .expect("property value length exceeds u32")
                .to_be_bytes(),
        );
        dtb.extend_from_slice(&name_offset.to_be_bytes());
        dtb.extend_from_slice(prop.value());
        Self::align(dtb);
    }

    fn align(vec: &mut Vec<u8>) {
        let len = vec.len();
        let new_len = Fdt::align_tag_offset(len);
        vec.resize(new_len, 0);
    }
}

struct StringMap {
    string_map: BTreeMap<String, u32>,
    next_offset: u32,
}

impl StringMap {
    #[must_use]
    fn new() -> Self {
        Self {
            string_map: BTreeMap::new(),
            next_offset: 0,
        }
    }

    fn insert(&mut self, key: &str) {
        if !self.string_map.contains_key(key) {
            let offset = self.next_offset;
            self.string_map.insert(key.to_owned(), offset);
            self.next_offset = u32::try_from(self.next_offset as usize + key.len() + 1)
                .expect("string block length exceeds u32");
        }
    }

    #[must_use]
    fn get_offset(&self, key: &str) -> u32 {
        *self
            .string_map
            .get(key)
            .expect("the key should have been inserted at the size calculation step")
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
