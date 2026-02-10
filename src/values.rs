use core::ffi::CStr;

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
        if !value.len().is_multiple_of(chunk_bytes) {
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
