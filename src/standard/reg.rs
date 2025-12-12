// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::fmt::{self, Display, Formatter};
use core::ops::{BitOr, Shl};

use zerocopy::big_endian;

use crate::error::FdtError;

/// The value of a `reg` property.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Reg<'a> {
    /// The address of the device within the address space of the parent bus.
    pub address: &'a [big_endian::U32],
    /// The size of the device within the address space of the parent bus.
    pub size: &'a [big_endian::U32],
}

impl Reg<'_> {
    /// Attempts to return the address as the given type, if it will fit.
    ///
    /// # Errors
    ///
    /// Returns `FdtError::AddressTooBig` if the size doesn't fit in `T`.
    pub fn address<T: Default + From<u32> + Shl<usize, Output = T> + BitOr<Output = T>>(
        self,
    ) -> Result<T, FdtError> {
        if size_of::<T>() < self.address.len() * size_of::<u32>() {
            Err(FdtError::AddressTooBig {
                cells: self.address.len(),
            })
        } else if let [address] = self.address {
            Ok(address.get().into())
        } else {
            let mut value = Default::default();
            for cell in self.address {
                value = (value << 32) | cell.get().into();
            }
            Ok(value)
        }
    }

    /// Attempts to return the size as the given type, if it will fit.
    ///
    /// # Errors
    ///
    /// Returns `FdtError::SizeTooBig` if the size doesn't fit in `T`.
    pub fn size<T: Default + From<u32> + Shl<usize, Output = T> + BitOr<Output = T>>(
        self,
    ) -> Result<T, FdtError> {
        if size_of::<T>() < self.size.len() * size_of::<u32>() {
            Err(FdtError::SizeTooBig {
                cells: self.size.len(),
            })
        } else if let [size] = self.size {
            Ok(size.get().into())
        } else {
            let mut value = Default::default();
            for cell in self.size {
                value = value << size_of::<u32>() | cell.get().into();
            }
            Ok(value)
        }
    }
}

impl Display for Reg<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("0x")?;
        for part in self.address {
            write!(f, "{part:08x}")?;
        }
        f.write_str(" 0x")?;
        for part in self.size {
            write!(f, "{part:08x}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_reg() {
        let reg = Reg {
            address: &[0x123_45678.into(), 0xabcd_0000.into()],
            size: &[0x1122_3344.into()],
        };
        assert_eq!(format!("{reg}"), "0x12345678abcd0000 0x11223344");
    }

    #[test]
    fn address_size() {
        let reg = Reg {
            address: &[0x123_45678.into(), 0xabcd_0000.into()],
            size: &[0x1122_3344.into()],
        };
        assert_eq!(
            reg.address::<u32>(),
            Err(FdtError::AddressTooBig { cells: 2 })
        );
        assert_eq!(reg.address::<u64>(), Ok(0x1234_5678_abcd_0000));
        assert_eq!(reg.size::<u32>(), Ok(0x1122_3344));
        assert_eq!(reg.size::<u64>(), Ok(0x1122_3344));
    }
}
