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
use crate::fdt::property::{FdtPropIter, FdtProperty};
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

    /// Returns the name of this node.
    ///
    /// # Panics
    ///
    /// Panics if the [`Fdt`] structure was constructed using
    /// [`Fdt::new_unchecked`] or [`Fdt::from_raw_unchecked`] and the FDT is not
    /// valid.
    fn name(&self) -> &'a str {
        let name_offset = self.offset + FDT_TAGSIZE;
        self.fdt
            .string_at_offset(name_offset, None)
            .expect("Fdt should be valid")
    }

    fn properties(&self) -> impl Iterator<Item = FdtProperty<'a>> + use<'a> {
        FdtPropIter::Start {
            fdt: self.fdt,
            offset: self.offset,
        }
    }

    fn children(&self) -> impl Iterator<Item = FdtNode<'a>> + use<'a> {
        FdtChildIter::Start { node: *self }
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

/// An iterator over the children of a device tree node.
enum FdtChildIter<'a> {
    Start {
        node: FdtNode<'a>,
    },
    Running {
        fdt: Fdt<'a>,
        offset: usize,
        address_space: AddressSpaceProperties,
    },
}

impl<'a> Iterator for FdtChildIter<'a> {
    type Item = FdtNode<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Start { node } => {
                let address_space = node.address_space();
                let mut offset = node.offset;
                offset += FDT_TAGSIZE; // Skip FDT_BEGIN_NODE
                offset = node
                    .fdt
                    .find_string_end(offset)
                    .expect("Fdt should be valid");
                offset = Fdt::align_tag_offset(offset);
                *self = Self::Running {
                    fdt: node.fdt,
                    offset,
                    address_space,
                };
                self.next()
            }
            Self::Running {
                fdt,
                offset,
                address_space,
            } => Self::try_next(*fdt, offset, *address_space),
        }
    }
}

impl<'a> FdtChildIter<'a> {
    fn try_next(
        fdt: Fdt<'a>,
        offset: &mut usize,
        parent_address_space: AddressSpaceProperties,
    ) -> Option<FdtNode<'a>> {
        loop {
            let token = fdt.read_token(*offset).expect("Fdt should be valid");
            match token {
                FdtToken::BeginNode => {
                    let node_offset = *offset;
                    *offset = fdt
                        .next_sibling_offset(*offset)
                        .expect("Fdt should be valid");
                    return Some(FdtNode {
                        fdt,
                        offset: node_offset,
                        parent_address_space,
                    });
                }
                FdtToken::Prop => {
                    *offset = fdt
                        .next_property_offset(*offset + FDT_TAGSIZE, false)
                        .expect("Fdt should be valid");
                }
                FdtToken::EndNode | FdtToken::End => return None,
                FdtToken::Nop => *offset += FDT_TAGSIZE,
            }
        }
    }
}
