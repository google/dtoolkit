// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[cfg(feature = "arrayvec07")]
use arrayvec::ArrayVec;
use zerocopy::FromBytes;

use crate::error::{BufferError, FdtErrorKind, FdtParseError};
use crate::fdt::FdtHeader;

/// A trait for buffers that can hold and mutate a Flattened Device Tree.
pub trait FdtBuffer: AsRef<[u8]> + AsMut<[u8]> {
    /// Attempts to resize the buffer to `new_len`.
    ///
    /// If `new_len` is greater than the current length, the buffer is grown and
    /// the new elements must be initialized to 0. If `new_len` is less than
    /// the current length, the buffer is truncated to `new_len`.
    ///
    /// The implementation may or may not update the length in the FDT header to
    /// match `new_len`.
    ///
    /// # Errors
    ///
    /// Returns a [`BufferError`] if the buffer cannot be resized to `new_len`.
    /// This method is guaranteed to never fail if `new_len` is less than or
    /// equal to the current length.
    fn try_resize(&mut self, new_len: usize) -> Result<(), BufferError>;
}

impl<B: FdtBuffer + ?Sized> FdtBuffer for &mut B {
    fn try_resize(&mut self, new_len: usize) -> Result<(), BufferError> {
        (**self).try_resize(new_len)
    }
}

/// A buffer wrapping a mutable byte slice `&'a mut [u8]`.
///
/// Instead of using the entire slice directly as the active FDT blob,
/// `SliceBuffer` reads the active length (`totalsize`) from the FDT header upon
/// creation. The remaining slice capacity can be used when expanding the
/// buffer.
#[derive(Debug, PartialEq, Eq)]
pub struct SliceBuffer<'a> {
    pub(crate) slice: &'a mut [u8],
}

impl<'a> SliceBuffer<'a> {
    /// Creates a new `SliceBuffer` from a mutable slice containing an FDT blob.
    ///
    /// Reads `totalsize` from the FDT header at the start of `slice` to verify
    /// the buffer length. The rest of `slice` is used as capacity for future
    /// buffer expansion.
    ///
    /// # Errors
    ///
    /// Returns an [`FdtParseError`] if `slice` is smaller than an FDT header,
    /// or if the header's `totalsize` is larger than the length of `slice`.
    pub fn new(slice: &'a mut [u8]) -> Result<Self, FdtParseError> {
        let (header, _) = FdtHeader::ref_from_prefix(slice)
            .map_err(|_| FdtParseError::new(FdtErrorKind::InvalidLength, 0))?;
        let len = header.totalsize() as usize;
        if len < size_of::<FdtHeader>() || len > slice.len() {
            return Err(FdtParseError::new(
                FdtErrorKind::InvalidLength,
                core::mem::offset_of!(FdtHeader, totalsize),
            ));
        }
        Ok(Self::new_unchecked(slice))
    }

    /// Creates a new `SliceBuffer` without validating the FDT header.
    ///
    /// The caller must ensure that `slice` contains a valid Flattened Device
    /// Tree (FDT) blob. If the blob is invalid, methods on `Fdt` and
    /// related types may panic.
    #[must_use]
    pub fn new_unchecked(slice: &'a mut [u8]) -> Self {
        Self { slice }
    }

    fn header(&self) -> &FdtHeader {
        let (header, _) = FdtHeader::ref_from_prefix(self.slice)
            .expect("FDT header should be valid after SliceBuffer construction");
        header
    }

    fn header_mut(&mut self) -> &mut FdtHeader {
        let (header, _) = FdtHeader::mut_from_prefix(self.slice)
            .expect("FDT header should be valid after SliceBuffer construction");
        header
    }
}

impl AsRef<[u8]> for SliceBuffer<'_> {
    fn as_ref(&self) -> &[u8] {
        let len = self.header().totalsize() as usize;
        &self.slice[..len]
    }
}

impl AsMut<[u8]> for SliceBuffer<'_> {
    fn as_mut(&mut self) -> &mut [u8] {
        let len = self.header().totalsize() as usize;
        &mut self.slice[..len]
    }
}

impl<'a> TryFrom<&'a mut [u8]> for SliceBuffer<'a> {
    type Error = FdtParseError;

    fn try_from(slice: &'a mut [u8]) -> Result<Self, Self::Error> {
        Self::new(slice)
    }
}

impl FdtBuffer for SliceBuffer<'_> {
    fn try_resize(&mut self, new_len: usize) -> Result<(), BufferError> {
        if new_len > self.slice.len() {
            return Err(BufferError::OutOfSpace {
                requested: new_len,
                capacity: self.slice.len(),
            });
        }
        let current_len = self.as_ref().len();
        if new_len > current_len {
            self.slice[current_len..new_len].fill(0);
        }

        let header = self.header_mut();
        let len_u32 = u32::try_from(new_len)
            .map_err(|_e| BufferError::FdtLimitExceeded { requested: new_len })?;
        header.totalsize.set(len_u32);

        Ok(())
    }
}

#[cfg(feature = "alloc")]
impl FdtBuffer for Vec<u8> {
    fn try_resize(&mut self, new_len: usize) -> Result<(), BufferError> {
        if new_len > self.len() {
            let additional = new_len - self.len();
            self.try_reserve(additional)?;
        }
        self.resize(new_len, 0);
        Ok(())
    }
}

#[cfg(feature = "arrayvec07")]
impl<const N: usize> FdtBuffer for ArrayVec<u8, N> {
    fn try_resize(&mut self, new_len: usize) -> Result<(), BufferError> {
        if new_len > N {
            return Err(BufferError::OutOfSpace {
                requested: new_len,
                capacity: N,
            });
        }
        if new_len < self.len() {
            self.truncate(new_len);
        } else if new_len > self.len() {
            let needed = new_len - self.len();
            self.extend(core::iter::repeat_n(0, needed));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use zerocopy::IntoBytes;

    use super::*;

    #[test]
    fn slice_buffer() {
        let mut raw = [0u8; 100];
        let header = FdtHeader {
            magic: crate::fdt::FDT_MAGIC.into(),
            totalsize: 40u32.into(),
            off_dt_struct: 0u32.into(),
            off_dt_strings: 0u32.into(),
            off_mem_rsvmap: 0u32.into(),
            version: 17u32.into(),
            last_comp_version: 16u32.into(),
            boot_cpuid_phys: 0u32.into(),
            size_dt_strings: 0u32.into(),
            size_dt_struct: 0u32.into(),
        };
        raw[..size_of::<FdtHeader>()].copy_from_slice(header.as_bytes());

        let mut buf = SliceBuffer::new(&mut raw).unwrap();
        assert_eq!(buf.as_ref().len(), 40);

        assert_eq!(buf.try_resize(50), Ok(()));
        assert_eq!(buf.as_ref().len(), 50);
        assert_eq!(buf.as_ref()[40..50], [0; 10]);

        assert_eq!(
            buf.try_resize(150),
            Err(BufferError::OutOfSpace {
                requested: 150,
                capacity: 100,
            })
        );
        assert_eq!(buf.as_ref().len(), 50);

        buf.try_resize(45).unwrap();
        assert_eq!(buf.as_ref().len(), 45);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn vec_buffer() {
        let mut vec = alloc::vec![0u8; 10];

        assert_eq!(vec.try_resize(15), Ok(()));
        assert_eq!(vec.len(), 15);
        assert_eq!(vec[10..15], [0, 0, 0, 0, 0]);

        vec.try_resize(5).unwrap();
        assert_eq!(vec.len(), 5);
    }

    #[cfg(feature = "arrayvec07")]
    #[test]
    fn arrayvec_buffer() {
        let mut array = ArrayVec::<u8, 15>::new();
        array.try_extend_from_slice(&[0u8; 10]).unwrap();

        assert_eq!(array.try_resize(12), Ok(()));
        assert_eq!(array.len(), 12);

        assert_eq!(
            array.try_resize(20),
            Err(BufferError::OutOfSpace {
                requested: 20,
                capacity: 15
            })
        );
        assert_eq!(array.len(), 12); // Length should not change on failure

        array.try_resize(5).unwrap();
        assert_eq!(array.len(), 5);
    }
}
