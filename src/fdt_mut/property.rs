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

use crate::error::FdtMutError;
use crate::fdt::property::InnerPropIter;
use crate::fdt::{FDT_TAGSIZE, Fdt, FdtProperty};
use crate::fdt_mut::FdtMut;
use crate::fdt_mut::buffer::FdtBuffer;
use crate::{Property, ToPropertyValue};

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
    /// # Performance
    ///
    /// If the new value is too big to fit into available space, this results in
    /// growing the buffer and shifting the data placed after the property. This
    /// requires time linear in the buffer size.
    ///
    /// # Errors
    ///
    /// Returns an [`FdtMutError::Resize`] if growing the buffer is required and
    /// it fails.
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
    /// let mut fdt = FdtMut::from_slice(&mut dtb).unwrap();
    /// let mut node = fdt.find_node_mut("/a/b/c").unwrap();
    /// assert_eq!(node.property("prop").unwrap().value(), b"\0\0\x04\xd2");
    /// node.property_mut("prop").unwrap().set_value(b"foo\0");
    /// assert_eq!(node.property("prop").unwrap().value(), b"foo\0");
    /// ```
    pub fn set_value<T: ToPropertyValue>(&mut self, new_value: T) -> Result<(), FdtMutError> {
        let new_len = new_value.property_value_len();
        let old_padded = Fdt::align_tag_offset(self.len);
        let new_padded = Fdt::align_tag_offset(new_len);

        if new_padded > old_padded {
            let needed_bytes = new_padded - old_padded;
            let available_nops =
                self.count_available_nops(self.value_offset + old_padded, needed_bytes);

            if available_nops < needed_bytes {
                let shift_amount = needed_bytes - available_nops;
                let shift_start = self.value_offset + old_padded + available_nops;

                self.data
                    .shift_dt_struct(shift_start, shift_amount)
                    .map_err(FdtMutError::Resize)?;
            }
        }

        // Update the length in the FDT property header
        let (len_bytes, _) = <big_endian::U32>::mut_from_prefix(
            &mut self.data.data_mut()[self.prop_offset + FDT_TAGSIZE..],
        )
        .expect("Fdt should be valid");
        len_bytes.set(new_len.try_into().expect("length should fit in u32"));

        self.data
            .write_property_value_with_padding(&new_value, new_padded, self.value_offset);

        if new_padded < old_padded {
            self.data.replace_with_nops(
                self.value_offset + new_padded,
                self.value_offset + old_padded,
            );
        }

        self.len = new_len;

        Ok(())
    }

    fn count_available_nops(&self, start_offset: usize, max_needed: usize) -> usize {
        let mut offset = start_offset;
        let mut count = 0;
        let fdt = self.data.as_read_only();
        while count < max_needed {
            if let Ok(crate::fdt::FdtToken::Nop) = fdt.read_token(offset) {
                count += FDT_TAGSIZE;
                offset += FDT_TAGSIZE;
            } else {
                break;
            }
        }
        count
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
    /// let mut fdt = FdtMut::from_slice(&mut dtb).unwrap();
    /// let mut node = fdt.find_node_mut("/a/b/c").unwrap();
    /// let prop = node.property_mut("prop").unwrap();
    /// prop.remove();
    /// assert!(node.property("prop").is_none());
    /// ```
    pub fn remove(self) {
        let start = self.prop_offset;
        let end = Fdt::align_tag_offset(self.value_offset + self.len);

        self.data.replace_with_nops(start, end);
    }
}

impl<B: FdtBuffer> Display for FdtPropertyMut<'_, B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.as_read_only())
    }
}

impl<'a, B: FdtBuffer> Property for &'a FdtPropertyMut<'_, B> {
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

    fn value_as<'v, T: crate::FromPropertyValue<'v>>(
        &self,
    ) -> Result<T, crate::error::PropertyError>
    where
        Self: 'v,
    {
        let fdt = self.data.as_read_only();
        let value = fdt
            .data
            .get(self.value_offset..self.value_offset + self.len)
            .expect("Fdt should be valid");
        T::from_property_value(value)
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
