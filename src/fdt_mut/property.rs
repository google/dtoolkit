// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::fmt;
use core::fmt::{Display, Formatter};

use zerocopy::{FromBytes, big_endian};

use crate::Property;
use crate::error::FdtMutError;
use crate::fdt::property::InnerPropIter;
use crate::fdt::{FDT_NOP, FDT_TAGSIZE, Fdt, FdtProperty};
use crate::fdt_mut::FdtMut;
use crate::fdt_mut::buffer::FdtBuffer;

/// A mutable property of a device tree node.
#[derive(Debug)]
pub struct FdtPropertyMut<'a, B: FdtBuffer> {
    pub(crate) data: &'a mut FdtMut<B>,
    pub(crate) nameoff: usize,
    pub(crate) prop_offset: usize,
    pub(crate) value_offset: usize,
    pub(crate) len: usize,
}

impl<B: FdtBuffer> FdtPropertyMut<'_, B> {
    /// Sets the value of the property.
    ///
    /// # Errors
    ///
    /// Returns an [`FdtMutError`] if shifting data fails.
    ///
    /// # Panics
    ///
    /// Panics if the new value's length cannot fit in a `u32`.
    ///
    /// Panics if the [`Fdt`] structure was constructed using
    /// [`Fdt::new_unchecked`] or [`Fdt::from_raw_unchecked`] and the FDT is not
    /// valid.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::fdt_mut::FdtMut;
    /// use dtoolkit::{Node, Property};
    ///
    /// # let mut dtb = include_bytes!("../../tests/dtb/test_traversal.dtb").to_vec();
    /// let mut fdt = FdtMut::new(&mut dtb[..]).unwrap();
    /// let mut node = fdt.find_node_mut("/a/b/c").unwrap();
    /// assert_eq!(node.property("prop").unwrap().value(), b"\0\0\x04\xd2");
    /// node.property_mut("prop").unwrap().set_value(b"foo\0");
    /// assert_eq!(node.property("prop").unwrap().value(), b"foo\0");
    /// ```
    pub fn set_value(&mut self, new_value: &[u8]) -> Result<(), FdtMutError> {
        let old_padded = Fdt::align_tag_offset(self.len);
        let new_padded = Fdt::align_tag_offset(new_value.len());

        self.ensure_new_length_fits(old_padded, new_padded)?;

        // Update the length in the FDT property header
        let (len_bytes, _) = <big_endian::U32>::mut_from_prefix(
            &mut self.data.data_mut()[self.prop_offset + FDT_TAGSIZE..],
        )
        .expect("Fdt should be valid");
        len_bytes.set(
            new_value
                .len()
                .try_into()
                .expect("length should fit in u32"),
        );

        // Copy the new value
        self.data.data_mut()[self.value_offset..self.value_offset + new_value.len()]
            .copy_from_slice(new_value);

        // Zero out any padding bytes
        for i in new_value.len()..new_padded {
            self.data.data_mut()[self.value_offset + i] = 0;
        }

        self.pad_with_nops(old_padded, new_padded);

        self.len = new_value.len();

        Ok(())
    }

    fn ensure_new_length_fits(
        &mut self,
        old_padded: usize,
        new_padded: usize,
    ) -> Result<(), FdtMutError> {
        if new_padded > old_padded {
            let needed_bytes = new_padded - old_padded;

            let mut offset = self.value_offset + old_padded;
            for _ in 0..(needed_bytes / FDT_TAGSIZE) {
                if offset + FDT_TAGSIZE > self.data.data_mut().len() {
                    return Err(FdtMutError::ShiftingRequired);
                }

                let tag_bytes = &self.data.data_mut()[offset..offset + FDT_TAGSIZE];
                if tag_bytes != FDT_NOP.to_be_bytes() {
                    return Err(FdtMutError::ShiftingRequired);
                }

                offset += FDT_TAGSIZE;
            }
        }

        Ok(())
    }

    fn pad_with_nops(&mut self, old_padded: usize, new_padded: usize) {
        if new_padded < old_padded {
            let mut offset = self.value_offset + new_padded;
            while offset < self.value_offset + old_padded {
                self.data.data_mut()[offset..offset + FDT_TAGSIZE]
                    .copy_from_slice(&FDT_NOP.to_be_bytes());
                offset += FDT_TAGSIZE;
            }
        }
    }

    /// Returns a read only view of this property.
    ///
    /// # Panics
    ///
    /// Panics if the underlying device tree data is invalid.
    #[must_use]
    pub fn as_read_only(&self) -> FdtProperty<'_> {
        let fdt = self.data.as_read_only();
        let name = fdt.string(self.nameoff).expect("Fdt should be valid");
        let value = fdt
            .data
            .get(self.value_offset..self.value_offset + self.len)
            .expect("Fdt should be valid");
        FdtProperty { name, value }
    }

    /// Removes the property by overwriting its structure with `NOP` tags.
    ///
    /// The memory previously occupied by this property will be replaced with
    /// `NOP` tags, rendering it invisible to Device Tree iterators
    /// without requiring data to be shifted.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::fdt_mut::FdtMut;
    /// use dtoolkit::{Node, Property};
    ///
    /// # let mut dtb = include_bytes!("../../tests/dtb/test_traversal.dtb").to_vec();
    /// let mut fdt = FdtMut::new(&mut dtb[..]).unwrap();
    /// let mut node = fdt.find_node_mut("/a/b/c").unwrap();
    /// let prop = node.property_mut("prop").unwrap();
    /// prop.remove();
    /// assert!(node.property("prop").is_none());
    /// ```
    pub fn remove(self) {
        let start = self.prop_offset;
        let end = Fdt::align_tag_offset(self.value_offset + self.len);
        let nop_bytes = FDT_NOP.to_be_bytes();

        let mut offset = start;
        while offset < end {
            self.data.data_mut()[offset..offset + FDT_TAGSIZE].copy_from_slice(&nop_bytes);
            offset += FDT_TAGSIZE;
        }
    }
}

impl<B: FdtBuffer> Display for FdtPropertyMut<'_, B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.as_read_only())
    }
}

impl<'a, B: FdtBuffer> Property for &'a FdtPropertyMut<'_, B> {
    type Str = &'a str;
    type StrList = crate::values::FdtStringListIterator<'a>;
    type PropEncodedArray<const N: usize> = crate::values::PropEncodedArrayIterator<'a, N>;
    type CellsItem = crate::Cells<'a>;

    fn name(&self) -> &str {
        let fdt = self.data.as_read_only();
        fdt.string(self.nameoff).expect("Fdt should be valid")
    }

    fn value(&self) -> &[u8] {
        let fdt = self.data.as_read_only();
        fdt.data
            .get(self.value_offset..self.value_offset + self.len)
            .expect("Fdt should be valid")
    }

    fn as_cells(&self) -> Result<crate::Cells<'a>, crate::error::PropertyError> {
        self.as_read_only().as_cells()
    }

    fn as_str(&self) -> Result<&'a str, crate::error::PropertyError> {
        self.as_read_only().as_str()
    }

    fn as_str_list(&self) -> Self::StrList {
        self.as_read_only().as_str_list()
    }

    fn as_prop_encoded_array<const N: usize>(
        &self,
        fields_cells: [usize; N],
    ) -> Result<Self::PropEncodedArray<N>, crate::error::PropertyError> {
        self.as_read_only().as_prop_encoded_array(fields_cells)
    }
}

/// A mutable iterator over the properties of a device tree node.
#[derive(Debug)]
pub struct FdtPropMutIter<'a, B: FdtBuffer> {
    pub(crate) data: &'a mut FdtMut<B>,
    pub(crate) inner: InnerPropIter,
}

impl<B: FdtBuffer> FdtPropMutIter<'_, B> {
    /// Returns the next mutable property.
    ///
    /// # Panics
    ///
    /// Panics if the underlying device tree data is invalid.
    pub fn next(&mut self) -> Option<FdtPropertyMut<'_, B>> {
        let fdt = self.data.as_read_only();
        let parsed = self.inner.next(fdt)?;
        Some(FdtPropertyMut {
            prop_offset: parsed.prop_offset,
            value_offset: parsed.value_offset,
            len: parsed.len,
            nameoff: parsed.nameoff,
            data: self.data,
        })
    }
}
