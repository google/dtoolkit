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

impl<'a> FromPropertyValue<'a> for FdtStringListIterator<'a> {
    fn from_property_value(value: &'a [u8]) -> Result<Self, PropertyError> {
        Ok(Self { value })
    }
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

/// A trait for types that can be parsed from a device tree property value.
pub trait FromPropertyValue<'a>: Sized {
    /// Parses a value of `Self` from a device tree property byte slice.
    ///
    /// # Errors
    ///
    /// Returns a [`PropertyError`] if the byte slice cannot be converted into
    /// `Self`.
    fn from_property_value(value: &'a [u8]) -> Result<Self, PropertyError>;
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

impl<'a> FromPropertyValue<'a> for &'a [u8] {
    fn from_property_value(value: &'a [u8]) -> Result<Self, PropertyError> {
        Ok(value)
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

impl<'a, const N: usize> FromPropertyValue<'a> for [u8; N] {
    fn from_property_value(value: &'a [u8]) -> Result<Self, PropertyError> {
        value.try_into().map_err(|_| PropertyError::InvalidLength)
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

impl<'a, const N: usize> FromPropertyValue<'a> for &'a [u8; N] {
    fn from_property_value(value: &'a [u8]) -> Result<Self, PropertyError> {
        value.try_into().map_err(|_| PropertyError::InvalidLength)
    }
}

#[cfg(feature = "alloc")]
impl<'a> FromPropertyValue<'a> for Vec<u8> {
    fn from_property_value(value: &'a [u8]) -> Result<Self, PropertyError> {
        Ok(value.to_vec())
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

impl<'a> FromPropertyValue<'a> for u32 {
    fn from_property_value(value: &'a [u8]) -> Result<Self, PropertyError> {
        value
            .try_into()
            .map(u32::from_be_bytes)
            .map_err(|_| PropertyError::InvalidLength)
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

impl<'a> FromPropertyValue<'a> for u64 {
    fn from_property_value(value: &'a [u8]) -> Result<Self, PropertyError> {
        value
            .try_into()
            .map(u64::from_be_bytes)
            .map_err(|_| PropertyError::InvalidLength)
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

impl<'a> FromPropertyValue<'a> for &'a str {
    fn from_property_value(value: &'a [u8]) -> Result<Self, PropertyError> {
        let stripped = value
            .strip_suffix(b"\0")
            .ok_or(PropertyError::InvalidString)?;
        core::str::from_utf8(stripped).map_err(|_| PropertyError::InvalidString)
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
impl<'a> FromPropertyValue<'a> for String {
    fn from_property_value(value: &'a [u8]) -> Result<Self, PropertyError> {
        let stripped = value
            .strip_suffix(b"\0")
            .ok_or(PropertyError::InvalidString)?;
        core::str::from_utf8(stripped)
            .map(String::from)
            .map_err(|_| PropertyError::InvalidString)
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

impl<'a, const N: usize> FromPropertyValue<'a> for [u32; N] {
    fn from_property_value(value: &'a [u8]) -> Result<Self, PropertyError> {
        if value.len() != N * size_of::<u32>() {
            return Err(PropertyError::InvalidLength);
        }
        let mut out = [0u32; N];
        for (i, chunk) in value
            .as_chunks::<{ size_of::<u32>() }>()
            .0
            .iter()
            .enumerate()
        {
            out[i] = u32::from_be_bytes(*chunk);
        }
        Ok(out)
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
impl<'a> FromPropertyValue<'a> for Vec<u32> {
    fn from_property_value(value: &'a [u8]) -> Result<Self, PropertyError> {
        if !value.len().is_multiple_of(size_of::<u32>()) {
            return Err(PropertyError::InvalidLength);
        }
        let mut out = Vec::with_capacity(value.len() / size_of::<u32>());
        for chunk in value.as_chunks::<{ size_of::<u32>() }>().0 {
            out.push(u32::from_be_bytes(*chunk));
        }
        Ok(out)
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
impl<'a> FromPropertyValue<'a> for Vec<&'a str> {
    fn from_property_value(value: &'a [u8]) -> Result<Self, PropertyError> {
        Ok(FdtStringListIterator { value }.collect::<Vec<_>>())
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

impl<'a> FromPropertyValue<'a> for Cells<'a> {
    fn from_property_value(value: &'a [u8]) -> Result<Self, PropertyError> {
        let cells = <[big_endian::U32] as FromBytes>::ref_from_bytes(value)
            .map_err(|_| PropertyError::InvalidLength)?;
        Ok(Self(cells))
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

    #[test]
    fn u32_array_invalid_length() {
        let bytes = [0, 0, 0, 1, 0, 0, 0, 2, 0]; // 9 bytes, not 8
        assert_eq!(
            <[u32; 2]>::from_property_value(&bytes),
            Err(PropertyError::InvalidLength)
        );
        let short_bytes = [0, 0, 0, 1]; // 4 bytes, needed 8
        assert_eq!(
            <[u32; 2]>::from_property_value(&short_bytes),
            Err(PropertyError::InvalidLength)
        );
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn u32_vec_invalid_length() {
        let bytes = [0, 0, 0, 1, 0]; // 5 bytes, not multiple of 4
        assert_eq!(
            <Vec<u32>>::from_property_value(&bytes),
            Err(PropertyError::InvalidLength)
        );
    }

    #[test]
    fn u8_conversions() {
        let raw = [1u8, 2, 3, 4];
        let mut buf = [0u8; 4];
        raw.write_property_value(&mut buf);
        assert_eq!(buf, raw);
        assert_eq!(raw.property_value_len(), 4);

        let slice: &[u8] = &buf;
        assert_eq!(slice.property_value_len(), 4);
        let mut slice_buf = [0u8; 4];
        slice.write_property_value(&mut slice_buf);
        assert_eq!(slice_buf, raw);
        assert_eq!(<&[u8]>::from_property_value(&buf).unwrap(), &raw[..]);

        let arr: [u8; 4] = FromPropertyValue::from_property_value(&buf).unwrap();
        assert_eq!(arr, raw);
        let arr_ref: &[u8; 4] = FromPropertyValue::from_property_value(&buf).unwrap();
        assert_eq!(arr_ref, &raw);
    }

    #[test]
    fn integer_conversions() {
        let val32: u32 = 0x1234_5678;
        assert_eq!(val32.property_value_len(), 4);
        let mut val32_buf = [0u8; 4];
        val32.write_property_value(&mut val32_buf);
        assert_eq!(val32_buf, [0x12, 0x34, 0x56, 0x78]);

        let val64: u64 = 0x1122_3344_5566_7788;
        assert_eq!(val64.property_value_len(), 8);
        let mut val64_buf = [0u8; 8];
        val64.write_property_value(&mut val64_buf);
        assert_eq!(val64_buf, [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88]);
    }

    #[test]
    fn u32_conversions() {
        let u32_slice: &[u32] = &[0x11, 0x22];
        assert_eq!(u32_slice.property_value_len(), 8);
        let mut u32_slice_buf = [0u8; 8];
        u32_slice.write_property_value(&mut u32_slice_buf);
        assert_eq!(u32_slice_buf, [0, 0, 0, 0x11, 0, 0, 0, 0x22]);

        let u32_arr: [u32; 2] = [0x11, 0x22];
        assert_eq!(u32_arr.property_value_len(), 8);
        let mut u32_arr_buf = [0u8; 8];
        u32_arr.write_property_value(&mut u32_arr_buf);
        assert_eq!(u32_arr_buf, [0, 0, 0, 0x11, 0, 0, 0, 0x22]);
        let decoded_arr: [u32; 2] = FromPropertyValue::from_property_value(&u32_arr_buf).unwrap();
        assert_eq!(decoded_arr, u32_arr);
    }

    #[test]
    fn str_slice_conversions() {
        let str_list: &[&str] = &["abc", "def"];
        assert_eq!(str_list.property_value_len(), 8);
        let mut str_list_buf = [0u8; 8];
        str_list.write_property_value(&mut str_list_buf);
        assert_eq!(&str_list_buf, b"abc\0def\0");
    }

    #[test]
    fn cells_conversions() {
        let cells_be = [big_endian::U32::new(0x10), big_endian::U32::new(0x20)];
        let cells = Cells(&cells_be);
        assert_eq!(cells.property_value_len(), 8);
        let mut cells_buf = [0u8; 8];
        cells.write_property_value(&mut cells_buf);
        assert_eq!(cells_buf, [0, 0, 0, 0x10, 0, 0, 0, 0x20]);
        let decoded_cells = Cells::from_property_value(&cells_buf).unwrap();
        assert_eq!(decoded_cells, cells);
        assert_eq!(cells.as_ref(), &cells_be[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn vec_u8_conversions() {
        let vec_u8 = alloc::vec![1u8, 2, 3];
        assert_eq!(vec_u8.property_value_len(), 3);
        let mut vec_u8_buf = [0u8; 3];
        vec_u8.write_property_value(&mut vec_u8_buf);
        assert_eq!(vec_u8_buf, [1, 2, 3]);
        let decoded_vec_u8: Vec<u8> = FromPropertyValue::from_property_value(&vec_u8_buf).unwrap();
        assert_eq!(decoded_vec_u8, vec_u8);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn string_conversions() {
        let string = String::from("hello");
        assert_eq!(string.property_value_len(), 6);
        let mut str_buf = [0u8; 6];
        string.write_property_value(&mut str_buf);
        assert_eq!(&str_buf, b"hello\0");
        let decoded_string: String = FromPropertyValue::from_property_value(&str_buf).unwrap();
        assert_eq!(decoded_string, string);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn vec_u32_conversions() {
        let vec_u32 = alloc::vec![0x10, 0x20];
        assert_eq!(vec_u32.property_value_len(), 8);
        let mut vec_u32_buf = [0u8; 8];
        vec_u32.write_property_value(&mut vec_u32_buf);
        assert_eq!(vec_u32_buf, [0, 0, 0, 0x10, 0, 0, 0, 0x20]);
        let decoded_vec_u32: Vec<u32> =
            FromPropertyValue::from_property_value(&vec_u32_buf).unwrap();
        assert_eq!(decoded_vec_u32, vec_u32);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn vec_str_conversions() {
        let vec_str = alloc::vec!["foo", "bar"];
        assert_eq!(vec_str.property_value_len(), 8);
        let mut vec_str_buf = [0u8; 8];
        vec_str.write_property_value(&mut vec_str_buf);
        assert_eq!(&vec_str_buf, b"foo\0bar\0");
        let decoded_vec_str: Vec<&str> =
            FromPropertyValue::from_property_value(&vec_str_buf).unwrap();
        assert_eq!(decoded_vec_str, vec_str);
    }
}
