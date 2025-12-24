// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A read-only API for inspecting a device tree property.

use core::fmt::{self, Display, Formatter};

use zerocopy::{FromBytes, big_endian};

use super::{FDT_TAGSIZE, Fdt, FdtToken};
use crate::Property;

/// A property of a device tree node.
#[derive(Debug, PartialEq)]
pub struct FdtProperty<'a> {
    name: &'a str,
    value: &'a [u8],
    value_offset: usize,
}

impl<'a> Property<'a> for FdtProperty<'a> {
    fn name(&self) -> &'a str {
        self.name
    }

    fn value(&self) -> &'a [u8] {
        self.value
    }
}

impl FdtProperty<'_> {
    pub(crate) fn fmt(&self, f: &mut Formatter, indent: usize) -> fmt::Result {
        write!(f, "{:indent$}{}", "", self.name, indent = indent)?;

        if self.value.is_empty() {
            writeln!(f, ";")?;
            return Ok(());
        }

        let is_printable = self
            .value
            .iter()
            .all(|&ch| ch.is_ascii_graphic() || ch == b' ' || ch == 0);
        let has_empty = self.value.windows(2).any(|window| window == [0, 0]);
        if is_printable && self.value.ends_with(&[0]) && !has_empty {
            let mut strings = self.as_str_list();
            if let Some(first) = strings.next() {
                write!(f, " = \"{first}\"")?;
                for s in strings {
                    write!(f, ", \"{s}\"")?;
                }
                writeln!(f, ";")?;
                return Ok(());
            }
        }

        if self.value.len().is_multiple_of(4) {
            write!(f, " = <")?;
            for (i, chunk) in self.value.chunks_exact(4).enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                let val = u32::from_be_bytes(
                    chunk
                        .try_into()
                        .expect("u32::from_be_bytes() should always succeed with 4 bytes"),
                );
                write!(f, "0x{val:02x}")?;
            }
            writeln!(f, ">;")?;
        } else {
            write!(f, " = [")?;
            for (i, byte) in self.value.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{byte:02x}")?;
            }
            writeln!(f, "];")?;
        }

        Ok(())
    }
}

impl Display for FdtProperty<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.fmt(f, 0)
    }
}

/// An iterator over the properties of a device tree node.
pub(crate) enum FdtPropIter<'a> {
    Start { fdt: Fdt<'a>, offset: usize },
    Running { fdt: Fdt<'a>, offset: usize },
}

impl<'a> Iterator for FdtPropIter<'a> {
    type Item = FdtProperty<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Start { fdt, offset } => {
                let mut offset = *offset;
                offset += FDT_TAGSIZE; // Skip FDT_BEGIN_NODE
                offset = fdt.find_string_end(offset).expect("Fdt should be valid");
                offset = Fdt::align_tag_offset(offset);
                *self = Self::Running { fdt: *fdt, offset };
                self.next()
            }
            Self::Running { fdt, offset } => Self::next_prop(*fdt, offset),
        }
    }
}

impl<'a> FdtPropIter<'a> {
    fn next_prop(fdt: Fdt<'a>, offset: &mut usize) -> Option<FdtProperty<'a>> {
        loop {
            let token = fdt.read_token(*offset).expect("Fdt should be valid");
            match token {
                FdtToken::Prop => {
                    let len = big_endian::U32::ref_from_prefix(&fdt.data[*offset + FDT_TAGSIZE..])
                        .expect("Fdt should be valid")
                        .0
                        .get() as usize;
                    let nameoff =
                        big_endian::U32::ref_from_prefix(&fdt.data[*offset + 2 * FDT_TAGSIZE..])
                            .expect("Fdt should be valid")
                            .0
                            .get() as usize;
                    let prop_offset = *offset + 3 * FDT_TAGSIZE;
                    *offset = Fdt::align_tag_offset(prop_offset + len);
                    let name = fdt.string(nameoff).expect("Fdt should be valid");
                    let value = fdt
                        .data
                        .get(prop_offset..prop_offset + len)
                        .expect("Fdt should be valid");
                    return Some(FdtProperty {
                        name,
                        value,
                        value_offset: prop_offset,
                    });
                }
                FdtToken::Nop => *offset += FDT_TAGSIZE,
                _ => return None,
            }
        }
    }
}
