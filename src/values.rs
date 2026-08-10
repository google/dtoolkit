// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Helper types and traits for property value conversions.

#[cfg(feature = "alloc")]
use alloc::string::String;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;
use core::ffi::CStr;
use core::mem::{size_of, size_of_val};

use zerocopy::{FromBytes, big_endian};

use crate::Cells;
use crate::error::PropertyError;

/// An iterator over the strings in a device tree property.
#[derive(Debug, Clone)]
pub struct FdtStringListIterator<'a> {
    pub(crate) value: &'a [u8],
}

impl<'a> Iterator for FdtStringListIterator<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.value.is_empty() {
            return None;
        }
        let cstr = CStr::from_bytes_until_nul(self.value).ok()?;
        let s = cstr.to_str().ok()?;
        self.value = &self.value[s.len() + 1..];
        Some(s)
    }
}

/// An iterator over the prop-encoded-array elements of a device tree property.
#[derive(Debug, Clone)]
pub struct PropEncodedArrayIterator<'a, const N: usize> {
    chunks: core::slice::ChunksExact<'a, u8>,
    fields_cells: [usize; N],
}

impl<'a, const N: usize> PropEncodedArrayIterator<'a, N> {
    pub(crate) fn new(value: &'a [u8], fields_cells: [usize; N]) -> Result<Self, PropertyError> {
        let chunk_cells: usize = fields_cells.iter().sum();
        let chunk_bytes = chunk_cells * size_of::<u32>();
        if chunk_cells == 0 || !value.len().is_multiple_of(chunk_bytes) {
            return Err(PropertyError::PropEncodedArraySizeMismatch {
                size: value.len(),
                chunk: chunk_cells,
            });
        }
        Ok(Self {
            chunks: value.chunks_exact(chunk_bytes),
            fields_cells,
        })
    }
}

impl<'a, const N: usize> Iterator for PropEncodedArrayIterator<'a, N> {
    type Item = [Cells<'a>; N];

    fn next(&mut self) -> Option<Self::Item> {
        let chunk = self.chunks.next()?;
        let mut cells_slice = <[big_endian::U32]>::ref_from_bytes(chunk)
            .expect("chunk should be a multiple of 4 bytes because of chunks_exact");

        Some(self.fields_cells.map(|field_cells| {
            let field;
            (field, cells_slice) = cells_slice.split_at(field_cells);
            Cells(field)
        }))
    }
}

/// A trait for types that can be serialized into a device tree property value.
pub trait ToPropertyValue {
    /// Returns the length in bytes of the serialized property value.
    #[must_use]
    fn property_value_len(&self) -> usize;

    /// Writes the serialized property value into `buffer`.
    ///
    /// # Panics
    ///
    /// The caller must ensure that `buffer.len() == self.property_value_len()`.
    /// May panic if `buffer.len() != self.property_value_len()`.
    fn write_property_value(&self, buffer: &mut [u8]);
}

impl<T: ToPropertyValue> ToPropertyValue for &T {
    fn property_value_len(&self) -> usize {
        (*self).property_value_len()
    }

    fn write_property_value(&self, buffer: &mut [u8]) {
        (*self).write_property_value(buffer);
    }
}

impl ToPropertyValue for &[u8] {
    fn property_value_len(&self) -> usize {
        self.len()
    }

    fn write_property_value(&self, buffer: &mut [u8]) {
        buffer.copy_from_slice(self);
    }
}

impl<const N: usize> ToPropertyValue for [u8; N] {
    fn property_value_len(&self) -> usize {
        self.len()
    }

    fn write_property_value(&self, buffer: &mut [u8]) {
        buffer.copy_from_slice(self);
    }
}

#[cfg(feature = "alloc")]
impl ToPropertyValue for Vec<u8> {
    fn property_value_len(&self) -> usize {
        self.len()
    }

    fn write_property_value(&self, buffer: &mut [u8]) {
        buffer.copy_from_slice(self);
    }
}

impl ToPropertyValue for u32 {
    fn property_value_len(&self) -> usize {
        size_of::<u32>()
    }

    fn write_property_value(&self, buffer: &mut [u8]) {
        buffer.copy_from_slice(&self.to_be_bytes());
    }
}

impl ToPropertyValue for u64 {
    fn property_value_len(&self) -> usize {
        size_of::<u64>()
    }

    fn write_property_value(&self, buffer: &mut [u8]) {
        buffer.copy_from_slice(&self.to_be_bytes());
    }
}

impl ToPropertyValue for &str {
    fn property_value_len(&self) -> usize {
        self.len() + 1
    }

    fn write_property_value(&self, buffer: &mut [u8]) {
        let len = self.len();
        buffer[..len].copy_from_slice(self.as_bytes());
        buffer[len] = 0;
    }
}

#[cfg(feature = "alloc")]
impl ToPropertyValue for String {
    fn property_value_len(&self) -> usize {
        self.len() + 1
    }

    fn write_property_value(&self, buffer: &mut [u8]) {
        let len = self.len();
        buffer[..len].copy_from_slice(self.as_bytes());
        buffer[len] = 0;
    }
}

impl ToPropertyValue for &[u32] {
    fn property_value_len(&self) -> usize {
        size_of_val(*self)
    }

    fn write_property_value(&self, mut buffer: &mut [u8]) {
        for val in *self {
            buffer[..size_of::<u32>()].copy_from_slice(&val.to_be_bytes());
            buffer = &mut buffer[size_of::<u32>()..];
        }
    }
}

impl<const N: usize> ToPropertyValue for [u32; N] {
    fn property_value_len(&self) -> usize {
        self.len() * size_of::<u32>()
    }

    fn write_property_value(&self, buffer: &mut [u8]) {
        let (chunks, _) = buffer.as_chunks_mut::<{ size_of::<u32>() }>();
        for (chunk, val) in chunks.iter_mut().zip(self) {
            *chunk = val.to_be_bytes();
        }
    }
}

#[cfg(feature = "alloc")]
impl ToPropertyValue for Vec<u32> {
    fn property_value_len(&self) -> usize {
        self.len() * size_of::<u32>()
    }

    fn write_property_value(&self, mut buffer: &mut [u8]) {
        for val in self {
            buffer[..size_of::<u32>()].copy_from_slice(&val.to_be_bytes());
            buffer = &mut buffer[size_of::<u32>()..];
        }
    }
}

impl ToPropertyValue for &[&str] {
    fn property_value_len(&self) -> usize {
        self.iter().map(|s| s.len() + 1).sum()
    }

    fn write_property_value(&self, mut buffer: &mut [u8]) {
        for s in *self {
            let len = s.len();
            buffer[..len].copy_from_slice(s.as_bytes());
            buffer[len] = 0;
            buffer = &mut buffer[len + 1..];
        }
    }
}

#[cfg(feature = "alloc")]
impl ToPropertyValue for Vec<&str> {
    fn property_value_len(&self) -> usize {
        self.iter().map(|s| s.len() + 1).sum()
    }

    fn write_property_value(&self, mut buffer: &mut [u8]) {
        for s in self {
            let len = s.len();
            buffer[..len].copy_from_slice(s.as_bytes());
            buffer[len] = 0;
            buffer = &mut buffer[len + 1..];
        }
    }
}

impl ToPropertyValue for Cells<'_> {
    fn property_value_len(&self) -> usize {
        self.0.len() * size_of::<u32>()
    }

    fn write_property_value(&self, mut buffer: &mut [u8]) {
        for val in self.0 {
            buffer[..size_of::<u32>()].copy_from_slice(&val.get().to_be_bytes());
            buffer = &mut buffer[size_of::<u32>()..];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prop_encoded_array_zero_cells() {
        assert_eq!(
            PropEncodedArrayIterator::new(&[], [0, 0]).unwrap_err(),
            PropertyError::PropEncodedArraySizeMismatch { size: 0, chunk: 0 }
        );
        assert_eq!(
            PropEncodedArrayIterator::new(&[1, 2, 3, 4], [0, 0, 0]).unwrap_err(),
            PropertyError::PropEncodedArraySizeMismatch { size: 4, chunk: 0 }
        );
    }
}
