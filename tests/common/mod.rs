// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Low-level helpers for building DTBs in tests.
//!
//! These helpers write the binary format directly and do not use any code of
//! the library under test.

pub(crate) const FDT_BEGIN_NODE: u32 = 0x1;
pub(crate) const FDT_END_NODE: u32 = 0x2;
pub(crate) const FDT_PROP: u32 = 0x3;
pub(crate) const FDT_NOP: u32 = 0x4;
pub(crate) const FDT_END: u32 = 0x9;

/// Builds a version 17 FDT blob from a structure block and a strings block.
pub(crate) fn build_dtb(structure: &[u8], strings: &[u8]) -> Vec<u8> {
    // The header is followed by an empty memory reservation block (just the
    // terminator), then by the structure block and the strings block.
    let header_size = 40;
    let off_mem_rsvmap = header_size;
    let off_dt_struct = off_mem_rsvmap + 16;
    let off_dt_strings = off_dt_struct + structure.len();
    let totalsize = off_dt_strings + strings.len();

    let mut dtb = Vec::with_capacity(totalsize);
    for value in [
        0xd00d_feed,
        totalsize,
        off_dt_struct,
        off_dt_strings,
        off_mem_rsvmap,
        17,
        16,
        0,
        strings.len(),
        structure.len(),
    ] {
        dtb.extend_from_slice(
            &u32::try_from(value)
                .expect("DTB header fields should fit in u32")
                .to_be_bytes(),
        );
    }
    dtb.extend_from_slice(&[0; 16]);
    dtb.extend_from_slice(structure);
    dtb.extend_from_slice(strings);
    dtb
}

/// Appends a big-endian token (or any other 32-bit value) to `structure`.
pub(crate) fn push_token(structure: &mut Vec<u8>, token: u32) {
    structure.extend_from_slice(&token.to_be_bytes());
}

/// Appends an `FDT_BEGIN_NODE` token and the padded node name to `structure`.
pub(crate) fn push_begin_node(structure: &mut Vec<u8>, name: &str) {
    push_token(structure, FDT_BEGIN_NODE);
    structure.extend_from_slice(name.as_bytes());
    structure.push(0);
    structure.resize(structure.len().next_multiple_of(4), 0);
}

/// Builds a DTB with `depth` nested nodes named `a` below the root.
pub(crate) fn build_chain_dtb(depth: usize) -> Vec<u8> {
    let mut structure = Vec::new();
    push_begin_node(&mut structure, "");
    for _ in 0..depth {
        push_begin_node(&mut structure, "a");
    }
    for _ in 0..=depth {
        push_token(&mut structure, FDT_END_NODE);
    }
    push_token(&mut structure, FDT_END);
    build_dtb(&structure, &[])
}

/// Builds a DTB with `count` childless nodes named `a` directly below the root.
pub(crate) fn build_flat_dtb(count: usize) -> Vec<u8> {
    let mut structure = Vec::new();
    push_begin_node(&mut structure, "");
    for _ in 0..count {
        push_begin_node(&mut structure, "a");
        push_token(&mut structure, FDT_END_NODE);
    }
    push_token(&mut structure, FDT_END_NODE);
    push_token(&mut structure, FDT_END);
    build_dtb(&structure, &[])
}
