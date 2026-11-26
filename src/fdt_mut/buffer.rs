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
#[cfg(feature = "heapless09")]
use heapless::Vec as HeaplessVec;
use zerocopy::FromBytes;

use crate::error::BufferError;
use crate::fdt::FdtHeader;

/// A trait for buffers that can hold and mutate a Flattened Device Tree.
pub trait FdtBuffer {
    /// Returns the active bytes of the FDT blob.
    fn data(&self) -> &[u8];

    /// Returns the active bytes of the FDT blob as a mutable slice.
    ///
    /// Must return a mutable slice over the same underlying data (with the same
    /// length) as [`data`](Self::data), so that mutations through this slice
    /// are reflected in subsequent calls to [`data`](Self::data).
    fn data_mut(&mut self) -> &mut [u8];

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
    fn data(&self) -> &[u8] {
        (**self).data()
    }

    fn data_mut(&mut self) -> &mut [u8] {
        (**self).data_mut()
    }

    fn try_resize(&mut self, new_len: usize) -> Result<(), BufferError> {
        (**self).try_resize(new_len)
    }
}

fn fdt_slice_active_len(slice: &[u8]) -> usize {
    // slices shorter than FdtHeader cannot hold totalsize
    let Ok((header, _)) = FdtHeader::ref_from_prefix(slice) else {
        return 0;
    };
    // clamp to slice length to avoid panics for malformed DTBs
    (header.totalsize() as usize).min(slice.len())
}

/// Uses `totalsize` from the FDT header as the active length and any remaining
/// bytes as spare capacity.
///
/// The slice needs to be at least 40 bytes long (the length of the FDT header);
/// otherwise, resizing the buffer to any non-zero length will fail.
impl FdtBuffer for [u8] {
    fn data(&self) -> &[u8] {
        &self[..fdt_slice_active_len(self)]
    }

    fn data_mut(&mut self) -> &mut [u8] {
        let len = fdt_slice_active_len(self);
        &mut self[..len]
    }

    fn try_resize(&mut self, new_len: usize) -> Result<(), BufferError> {
        if self.len() < size_of::<FdtHeader>() {
            if new_len == 0 {
                return Ok(());
            }
            return Err(BufferError::OutOfSpace {
                requested: new_len,
                capacity: 0,
            });
        }
        if new_len > self.len() {
            return Err(BufferError::OutOfSpace {
                requested: new_len,
                capacity: self.len(),
            });
        }
        let len_u32 = u32::try_from(new_len).map_err(|_e| BufferError::FdtLimitExceeded {
            requested: new_len as u64,
        })?;
        let current_len = fdt_slice_active_len(self);
        if new_len > current_len {
            self[current_len..new_len].fill(0);
        }

        let (header, _) =
            FdtHeader::mut_from_prefix(self).expect("buffer length checked against FdtHeader size");
        header.totalsize.set(len_u32);

        Ok(())
    }
}

/// Uses `totalsize` from the FDT header as the active length and any remaining
/// bytes as spare capacity.
///
/// The array needs to be at least 40 bytes long (the length of the FDT header);
/// otherwise, resizing the buffer to any non-zero length will fail.
impl<const N: usize> FdtBuffer for [u8; N] {
    fn data(&self) -> &[u8] {
        self.as_slice().data()
    }

    fn data_mut(&mut self) -> &mut [u8] {
        self.as_mut_slice().data_mut()
    }

    fn try_resize(&mut self, new_len: usize) -> Result<(), BufferError> {
        self.as_mut_slice().try_resize(new_len)
    }
}

#[cfg(feature = "alloc")]
impl FdtBuffer for Vec<u8> {
    fn data(&self) -> &[u8] {
        self
    }

    fn data_mut(&mut self) -> &mut [u8] {
        self
    }

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
    fn data(&self) -> &[u8] {
        self
    }

    fn data_mut(&mut self) -> &mut [u8] {
        self
    }

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

#[cfg(feature = "heapless09")]
impl<const N: usize> FdtBuffer for HeaplessVec<u8, N> {
    fn data(&self) -> &[u8] {
        self
    }

    fn data_mut(&mut self) -> &mut [u8] {
        self
    }

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

        let buf: &mut [u8] = &mut raw;
        assert_eq!(buf.data().len(), 40);

        assert_eq!(buf.try_resize(50), Ok(()));
        assert_eq!(buf.data().len(), 50);
        assert_eq!(buf.data()[40..50], [0; 10]);

        assert_eq!(
            buf.try_resize(150),
            Err(BufferError::OutOfSpace {
                requested: 150,
                capacity: 100,
            })
        );
        assert_eq!(buf.data().len(), 50);

        buf.try_resize(45).unwrap();
        assert_eq!(buf.data().len(), 45);

        buf.try_resize(10).unwrap();
        assert_eq!(buf.data().len(), 10);

        buf.try_resize(0).unwrap();
        assert_eq!(buf.data(), []);

        buf.try_resize(50).unwrap();
        assert_eq!(buf.data().len(), 50);
    }

    #[test]
    fn small_slice_buffer() {
        let mut raw = [0u8; 20];
        let buf: &mut [u8] = &mut raw;
        assert_eq!(buf.data(), []);
        assert_eq!(buf.try_resize(0), Ok(()));
        for req in [10, 30] {
            assert_eq!(
                buf.try_resize(req),
                Err(BufferError::OutOfSpace {
                    requested: req,
                    capacity: 0,
                })
            );
        }
    }

    #[test]
    fn array_buffer() {
        let mut buf = [0u8; 100];
        buf[size_of::<u32>()..2 * size_of::<u32>()].copy_from_slice(&40u32.to_be_bytes());

        assert_eq!(buf.data().len(), 40);
        assert_eq!(buf.try_resize(50), Ok(()));
        assert_eq!(buf.data().len(), 50);
        assert_eq!(
            buf.try_resize(150),
            Err(BufferError::OutOfSpace {
                requested: 150,
                capacity: 100,
            })
        );

        buf.try_resize(45).unwrap();
        assert_eq!(buf.data().len(), 45);
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

    #[cfg(feature = "heapless09")]
    #[test]
    fn heapless_buffer() {
        let mut array = HeaplessVec::<u8, 15>::from_slice(&[0u8; 10]).unwrap();

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
