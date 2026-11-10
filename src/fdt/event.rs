// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A flat, single-pass view of the structure block of a device tree.

use super::property::InnerPropIter;
use super::{FDT_TAGSIZE, Fdt, FdtNode, FdtProperty, FdtToken};

/// An item of the structure block of a device tree, as returned by
/// [`FdtNode::events`] and [`Fdt::events`].
///
/// The events follow the order of the tokens in the structure block: each
/// node starts with [`FdtEvent::BeginNode`], followed by its properties, then
/// by its children (each with its own events), and ends with
/// [`FdtEvent::EndNode`]. `FDT_NOP` tokens are skipped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FdtEvent<'a> {
    /// The start of a node (an `FDT_BEGIN_NODE` token).
    BeginNode {
        /// The name of the node, including the unit address, if any. The name
        /// of the root node is empty.
        name: &'a str,
        /// The byte offset of the node's `FDT_BEGIN_NODE` tag relative to the
        /// start of the `dt_struct` block, as returned by
        /// [`FdtNode::offset`].
        ///
        /// It can be passed to [`Fdt::node_at_offset`] to get the
        /// corresponding [`FdtNode`].
        offset: usize,
        /// The depth of the node below the node the iterator started at: 0 for
        /// that node, 1 for its children, and so on. For [`Fdt::events`], it is
        /// the depth below the root node.
        depth: usize,
    },
    /// A property of the node that was most recently started (an `FDT_PROP`
    /// token).
    Property(FdtProperty<'a>),
    /// The end of the node that was most recently started and not yet ended
    /// (an `FDT_END_NODE` token).
    EndNode {
        /// The depth of the node that ends, the same as in its
        /// [`FdtEvent::BeginNode`].
        depth: usize,
    },
}

/// An iterator over the events of a device tree node and its descendants.
///
/// See [`FdtNode::events`] for details.
#[derive(Debug, Clone)]
pub struct FdtEventIter<'a> {
    inner: InnerEventIter<'a>,
}

#[derive(Debug, Clone)]
enum InnerEventIter<'a> {
    /// The node the iterator started at has not ended yet.
    Running {
        fdt: Fdt<'a>,
        /// The offset of the next token to read, relative to the start of the
        /// FDT.
        offset: usize,
        /// The number of nodes that have started and not yet ended. It is zero
        /// only before the first node starts.
        depth: usize,
    },
    /// The node the iterator started at has ended.
    Stopped,
}

impl<'a> FdtEventIter<'a> {
    pub(crate) fn new(node: FdtNode<'a>) -> Self {
        Self {
            inner: InnerEventIter::Running {
                fdt: node.fdt,
                offset: node.offset,
                depth: 0,
            },
        }
    }
}

impl<'a> Iterator for FdtEventIter<'a> {
    type Item = FdtEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let InnerEventIter::Running { fdt, offset, depth } = &mut self.inner else {
            return None;
        };
        let fdt = *fdt;
        loop {
            let token = fdt.read_token(*offset).expect("Fdt should be valid");
            match token {
                FdtToken::BeginNode => {
                    let tag_offset = *offset;
                    let name_offset = tag_offset + FDT_TAGSIZE;
                    let name = fdt
                        .string_at_offset(name_offset, None)
                        .expect("Fdt should be valid");
                    let node_depth = *depth;
                    *depth += 1;
                    // Skip the name and its NUL terminator.
                    *offset = Fdt::align_tag_offset(name_offset + name.len() + 1);
                    return Some(FdtEvent::BeginNode {
                        name,
                        offset: tag_offset - fdt.header().off_dt_struct() as usize,
                        depth: node_depth,
                    });
                }
                FdtToken::Prop => {
                    // A property outside a node only happens on an unchecked
                    // FDT.
                    assert!(*depth > 0, "Fdt should be valid");
                    let parsed = InnerPropIter::parse_property(fdt, *offset);
                    *offset = Fdt::align_tag_offset(parsed.value_offset + parsed.len);
                    return Some(FdtEvent::Property(FdtProperty {
                        name: parsed.name,
                        value: parsed.value,
                    }));
                }
                FdtToken::EndNode => {
                    // An end before a start only happens on an unchecked FDT.
                    *depth = depth.checked_sub(1).expect("Fdt should be valid");
                    let node_depth = *depth;
                    if node_depth == 0 {
                        self.inner = InnerEventIter::Stopped;
                    } else {
                        *offset += FDT_TAGSIZE;
                    }
                    return Some(FdtEvent::EndNode { depth: node_depth });
                }
                FdtToken::Nop => *offset += FDT_TAGSIZE,
                // `FDT_END` may only follow the end of the root node, and the
                // iterator stops before reading it.
                FdtToken::End => panic!("Fdt should be valid"),
            }
        }
    }
}

impl core::iter::FusedIterator for FdtEventIter<'_> {}
