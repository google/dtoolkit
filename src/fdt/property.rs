// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A read-only API for inspecting a device tree property.

use core::fmt::{self, Display, Formatter};
use core::mem::size_of;

use zerocopy::FromBytes;

use super::{FDT_TAGSIZE, Fdt, FdtPropertyHeader, FdtToken};
use crate::Property;

/// A property of a device tree node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FdtProperty<'a> {
    pub(crate) name: &'a str,
    pub(crate) value: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ParsedProperty<'a> {
    pub name: &'a str,
    pub value: &'a [u8],
    pub nameoff: usize,
    pub prop_offset: usize,
    pub value_offset: usize,
    pub len: usize,
}

impl<'a> Property for FdtProperty<'a> {
    type PropEncodedArray<const N: usize> = crate::values::PropEncodedArrayIterator<'a, N>;
    type CellsItem = crate::Cells<'a>;

    fn name(&self) -> &'a str {
        self.name
    }

    fn value(&self) -> &'a [u8] {
        self.value
    }

    crate::impl_property_methods!(get_value = |self| self.value);
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
            let mut strings = (*self)
                .value_as::<crate::values::FdtStringListIterator>()
                .map_err(|_| fmt::Error)?;
            if let Some(first) = strings.next() {
                write!(f, " = \"{first}\"")?;
                for s in strings {
                    write!(f, ", \"{s}\"")?;
                }
                writeln!(f, ";")?;
                return Ok(());
            }
        }

        if self.value.len().is_multiple_of(size_of::<u32>()) {
            write!(f, " = <")?;
            let (chunks, remainder) = self.value.as_chunks::<{ size_of::<u32>() }>();
            debug_assert!(remainder.is_empty());
            for (i, chunk) in chunks.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                let val = u32::from_be_bytes(*chunk);
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

#[derive(Debug, Clone)]
pub(crate) enum InnerPropIter {
    Start { offset: usize },
    Running { offset: usize },
}

impl InnerPropIter {
    #[must_use]
    pub(crate) fn new(offset: usize) -> Self {
        Self::Start { offset }
    }

    #[must_use]
    pub(crate) fn next<'a>(&mut self, fdt: Fdt<'a>) -> Option<ParsedProperty<'a>> {
        match self {
            Self::Start { offset } => {
                let mut off = *offset;
                off += FDT_TAGSIZE; // Skip FDT_BEGIN_NODE
                off = fdt.find_string_end(off).expect("Fdt should be valid");
                off = Fdt::align_tag_offset(off);

                let result = Self::find_property(fdt, &mut off);
                *self = Self::Running { offset: off };
                result
            }
            Self::Running { offset } => Self::next_property_parsed(fdt, offset),
        }
    }

    #[must_use]
    fn next_property_parsed<'a>(fdt: Fdt<'a>, offset: &mut usize) -> Option<ParsedProperty<'a>> {
        *offset = fdt
            .next_property_offset(*offset, false)
            .expect("Fdt should be valid");
        Self::find_property(fdt, offset)
    }

    #[must_use]
    fn find_property<'a>(fdt: Fdt<'a>, offset: &mut usize) -> Option<ParsedProperty<'a>> {
        loop {
            let token = fdt.read_token(*offset).expect("Fdt should be valid");
            match token {
                FdtToken::Prop => return Some(Self::parse_property(fdt, *offset)),
                FdtToken::Nop => *offset += FDT_TAGSIZE,
                _ => return None,
            }
        }
    }

    #[must_use]
    fn parse_property(fdt: Fdt, offset: usize) -> ParsedProperty {
        let (header, _) =
            FdtPropertyHeader::ref_from_prefix(&fdt.data[offset..]).expect("Fdt should be valid");
        let len = header.len() as usize;
        let nameoff = header.nameoff() as usize;
        let value_offset = offset + size_of::<FdtPropertyHeader>();
        let name = fdt.string(nameoff).expect("Fdt should be valid");
        let value = fdt
            .data
            .get(value_offset..value_offset + len)
            .expect("Fdt should be valid");

        ParsedProperty {
            name,
            value,
            nameoff,
            prop_offset: offset,
            value_offset,
            len,
        }
    }
}

/// An iterator over the properties of a device tree node.
#[derive(Debug, Clone)]
pub struct FdtPropIter<'a> {
    pub(crate) fdt: Fdt<'a>,
    pub(crate) inner: InnerPropIter,
}

impl<'a> Iterator for FdtPropIter<'a> {
    type Item = FdtProperty<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let parsed = self.inner.next(self.fdt)?;
        Some(FdtProperty {
            name: parsed.name,
            value: parsed.value,
        })
    }
}
