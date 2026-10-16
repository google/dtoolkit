// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A read-only API for inspecting a device tree node.

use core::fmt::{self, Display, Formatter};

use super::{FDT_TAGSIZE, Fdt, FdtToken};
use crate::Node;
use crate::fdt::property::{FdtPropIter, FdtProperty, InnerPropIter};
use crate::standard::{AddressSpaceProperties, NodeStandard};

/// A node in a flattened device tree.
#[derive(Debug, Clone, Copy)]
pub struct FdtNode<'a> {
    pub(crate) fdt: Fdt<'a>,
    pub(crate) offset: usize,
    /// The `#address-cells` and `#size-cells` properties of this node's parent
    /// node.
    pub(crate) parent_address_space: AddressSpaceProperties,
}

impl<'a> Node<'a> for FdtNode<'a> {
    type Property = FdtProperty<'a>;
    type Properties = FdtPropIter<'a>;
    type Children = private::FdtChildIter<'a>;

    /// Returns the name of this node.
    ///
    /// # Panics
    ///
    /// Panics if the [`Fdt`] structure was constructed using
    /// [`Fdt::new_unchecked`] or [`Fdt::from_raw_unchecked`] and the FDT is not
    /// valid.
    fn name(self) -> &'a str {
        let name_offset = self.offset + FDT_TAGSIZE;
        self.fdt
            .string_at_offset(name_offset, None)
            .expect("Fdt should be valid")
    }

    fn properties(self) -> FdtPropIter<'a> {
        FdtPropIter {
            fdt: self.fdt,
            inner: InnerPropIter::new(self.offset),
        }
    }

    fn children(self) -> private::FdtChildIter<'a> {
        private::FdtChildIter {
            fdt: self.fdt,
            inner: InnerChildIter::new(self.offset),
            parent_address_space: self.address_space(),
        }
    }
}

impl<'a> FdtNode<'a> {
    pub(crate) fn new(fdt: Fdt<'a>, offset: usize) -> Self {
        Self {
            fdt,
            offset,
            parent_address_space: AddressSpaceProperties::default(),
        }
    }

    /// Returns the byte offset of this node's `FDT_BEGIN_NODE` tag relative to
    /// the start of the `dt_struct` block.
    ///
    /// This matches the node offset representation used by `libfdt` (where the
    /// root node `/` is at offset `0`).
    ///
    /// # Panics
    ///
    /// Panics if the [`Fdt`] structure was constructed using
    /// [`Fdt::new_unchecked`] or [`Fdt::from_raw_unchecked`] and the FDT is not
    /// valid.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::Node;
    /// use dtoolkit::fdt::Fdt;
    ///
    /// # let dtb = include_bytes!("../../tests/dtb/test_traversal.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// assert_eq!(fdt.root().offset(), 0);
    ///
    /// let node = fdt.find_node("/a/b/c").unwrap();
    /// let by_offset = fdt.node_at_offset(node.offset()).unwrap();
    /// assert_eq!(by_offset.name(), "c");
    /// ```
    #[must_use]
    pub fn offset(self) -> usize {
        self.offset - self.fdt.header().off_dt_struct() as usize
    }

    pub(crate) fn fmt_recursive(&self, f: &mut Formatter, indent: usize) -> fmt::Result {
        let name = self.name();
        if name.is_empty() {
            writeln!(f, "{:indent$}/ {{", "", indent = indent)?;
        } else {
            writeln!(f, "{:indent$}{} {{", "", name, indent = indent)?;
        }

        let mut has_properties = false;
        for prop in self.properties() {
            has_properties = true;
            prop.fmt(f, indent + 4)?;
        }

        let mut first_child = true;
        for child in self.children() {
            if !first_child || has_properties {
                writeln!(f)?;
            }

            first_child = false;
            child.fmt_recursive(f, indent + 4)?;
        }

        writeln!(f, "{:indent$}}};", "", indent = indent)
    }
}

impl Display for FdtNode<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.fmt_recursive(f, 0)
    }
}

#[derive(Debug, Clone)]
pub(crate) enum InnerChildIter {
    Start { offset: usize },
    Running { offset: usize },
}

impl InnerChildIter {
    #[must_use]
    pub(crate) fn new(offset: usize) -> Self {
        Self::Start { offset }
    }

    #[must_use]
    pub(crate) fn next(&mut self, fdt: Fdt<'_>) -> Option<usize> {
        match self {
            Self::Start { offset } => {
                let mut off = *offset;
                off += FDT_TAGSIZE; // Skip FDT_BEGIN_NODE
                off = fdt.find_string_end(off).expect("Fdt should be valid");
                off = Fdt::align_tag_offset(off);

                let result = Self::skip_props(fdt, &mut off);
                *self = Self::Running { offset: off };
                result
            }
            Self::Running { offset } => Self::next_child_parsed(fdt, offset),
        }
    }

    #[must_use]
    fn skip_props(fdt: Fdt<'_>, offset: &mut usize) -> Option<usize> {
        loop {
            let token = fdt.read_token(*offset).expect("Fdt should be valid");
            match token {
                FdtToken::BeginNode => return Some(*offset),
                FdtToken::Prop => {
                    *offset = fdt
                        .next_property_offset(*offset, false)
                        .expect("Fdt should be valid");
                }
                FdtToken::EndNode | FdtToken::End => return None,
                FdtToken::Nop => *offset += FDT_TAGSIZE,
            }
        }
    }

    #[must_use]
    fn next_child_parsed(fdt: Fdt<'_>, offset: &mut usize) -> Option<usize> {
        *offset = fdt
            .next_sibling_offset(*offset)
            .expect("Fdt should be valid");
        loop {
            let token = fdt.read_token(*offset).expect("Fdt should be valid");
            match token {
                FdtToken::BeginNode => {
                    return Some(*offset);
                }
                FdtToken::Prop => {
                    panic!("Fdt should be valid");
                }
                FdtToken::EndNode | FdtToken::End => return None,
                FdtToken::Nop => *offset += FDT_TAGSIZE,
            }
        }
    }
}

pub(crate) mod private {
    use crate::fdt::node::InnerChildIter;
    use crate::fdt::{Fdt, FdtNode};
    use crate::standard::AddressSpaceProperties;

    /// An iterator over the children of a device tree node.
    #[derive(Debug, Clone)]
    pub struct FdtChildIter<'a> {
        pub(crate) fdt: Fdt<'a>,
        pub(crate) inner: InnerChildIter,
        pub(crate) parent_address_space: AddressSpaceProperties,
    }

    impl<'a> Iterator for FdtChildIter<'a> {
        type Item = FdtNode<'a>;

        fn next(&mut self) -> Option<Self::Item> {
            let node_offset = self.inner.next(self.fdt)?;
            Some(FdtNode {
                fdt: self.fdt,
                offset: node_offset,
                parent_address_space: self.parent_address_space,
            })
        }
    }
}
