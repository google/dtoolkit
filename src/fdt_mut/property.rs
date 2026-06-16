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
use crate::fdt::{FDT_TAGSIZE, Fdt, FdtProperty};
use crate::fdt_mut::FdtMut;

/// A mutable property of a device tree node.
#[derive(Debug)]
pub struct FdtPropertyMut<'a> {
    pub(crate) data: FdtMut<'a>,
    pub(crate) nameoff: usize,
    pub(crate) prop_offset: usize,
    pub(crate) value_offset: usize,
    pub(crate) len: usize,
}

impl FdtPropertyMut<'_> {
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
    /// let mut fdt = FdtMut::new(&mut dtb).unwrap();
    /// let mut node = fdt.find_node_mut("/a/b/c").unwrap();
    /// assert_eq!(node.property("prop").unwrap().value(), b"\0\0\x04\xd2");
    /// node.property_mut("prop").unwrap().set_value(b"foo\0");
    /// assert_eq!(node.property("prop").unwrap().value(), b"foo\0");
    /// ```
    pub fn set_value(&mut self, new_value: &[u8]) -> Result<(), FdtMutError> {
        let old_padded = Fdt::align_tag_offset(self.len);
        let new_padded = Fdt::align_tag_offset(new_value.len());

        if new_padded != old_padded {
            todo!("the new value requires shifting data, which is not yet supported");
        }

        // Update the length in the FDT property header
        let (len_bytes, _) = <big_endian::U32>::mut_from_prefix(
            &mut self.data.data[self.prop_offset + FDT_TAGSIZE..],
        )
        .expect("Fdt should be valid");
        len_bytes.set(
            new_value
                .len()
                .try_into()
                .expect("length should fit in u32"),
        );

        // Copy the new value
        self.data.data[self.value_offset..self.value_offset + new_value.len()]
            .copy_from_slice(new_value);

        // Zero out any padding bytes
        for i in new_value.len()..new_padded {
            self.data.data[self.value_offset + i] = 0;
        }

        self.len = new_value.len();

        Ok(())
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
}

impl Display for FdtPropertyMut<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{}", self.as_read_only())
    }
}

impl<'a> Property for &'a FdtPropertyMut<'_> {
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
pub struct FdtPropMutIter<'a> {
    pub(crate) data: FdtMut<'a>,
    pub(crate) inner: InnerPropIter,
}

impl FdtPropMutIter<'_> {
    /// Returns the next mutable property.
    ///
    /// # Panics
    ///
    /// Panics if the underlying device tree data is invalid.
    pub fn next(&mut self) -> Option<FdtPropertyMut<'_>> {
        let fdt = self.data.as_read_only();
        let parsed = self.inner.next(fdt)?;
        Some(FdtPropertyMut {
            prop_offset: parsed.prop_offset,
            value_offset: parsed.value_offset,
            len: parsed.len,
            nameoff: parsed.nameoff,
            data: self.data.reborrow(),
        })
    }
}
