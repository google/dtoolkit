// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A read-only API for inspecting a device tree property.

use core::ffi::CStr;

use zerocopy::{FromBytes, big_endian};

use super::{FDT_TAGSIZE, Fdt, FdtToken};
use crate::error::{FdtError, FdtErrorKind};

/// A property of a device tree node.
#[derive(Debug, PartialEq)]
pub struct FdtProperty<'a> {
    name: &'a str,
    value: &'a [u8],
    value_offset: usize,
}

impl<'a> FdtProperty<'a> {
    /// Returns the name of this property.
    #[must_use]
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Returns the value of this property.
    #[must_use]
    pub fn value(&self) -> &'a [u8] {
        self.value
    }
    /// Returns the value of this property as a `u32`.
    ///
    /// # Errors
    ///
    /// Returns an [`FdtErrorKind::InvalidLength`] if the property's value is
    /// not 4 bytes long.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::fdt::Fdt;
    /// # let dtb = include_bytes!("../../tests/dtb/test_props.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let node = fdt.find_node("/test-props").unwrap().unwrap();
    /// let prop = node.property("u32-prop").unwrap().unwrap();
    /// assert_eq!(prop.as_u32().unwrap(), 0x12345678);
    /// ```
    pub fn as_u32(&self) -> Result<u32, FdtError> {
        big_endian::U32::ref_from_bytes(self.value)
            .map(|val| val.get())
            .map_err(|_e| FdtError::new(FdtErrorKind::InvalidLength, self.value_offset))
    }

    /// Returns the value of this property as a `u64`.
    ///
    /// # Errors
    ///
    /// Returns an [`FdtErrorKind::InvalidLength`] if the property's value is
    /// not 8 bytes long.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::fdt::Fdt;
    /// # let dtb = include_bytes!("../../tests/dtb/test_props.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let node = fdt.find_node("/test-props").unwrap().unwrap();
    /// let prop = node.property("u64-prop").unwrap().unwrap();
    /// assert_eq!(prop.as_u64().unwrap(), 0x1122334455667788);
    /// ```
    pub fn as_u64(&self) -> Result<u64, FdtError> {
        big_endian::U64::ref_from_bytes(self.value)
            .map(|val| val.get())
            .map_err(|_e| FdtError::new(FdtErrorKind::InvalidLength, self.value_offset))
    }

    /// Returns the value of this property as a string.
    ///
    /// # Errors
    ///
    /// Returns an [`FdtErrorKind::InvalidString`] if the property's value is
    /// not a null-terminated string or contains invalid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::fdt::Fdt;
    /// # let dtb = include_bytes!("../../tests/dtb/test_props.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let node = fdt.find_node("/test-props").unwrap().unwrap();
    /// let prop = node.property("str-prop").unwrap().unwrap();
    /// assert_eq!(prop.as_str().unwrap(), "hello world");
    /// ```
    pub fn as_str(&self) -> Result<&'a str, FdtError> {
        let cstr = CStr::from_bytes_with_nul(self.value)
            .map_err(|_| FdtError::new(FdtErrorKind::InvalidString, self.value_offset))?;
        cstr.to_str()
            .map_err(|_| FdtError::new(FdtErrorKind::InvalidString, self.value_offset))
    }

    /// Returns an iterator over the strings in this property.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::fdt::Fdt;
    /// # let dtb = include_bytes!("../../tests/dtb/test_props.dtb");
    /// let fdt = Fdt::new(dtb).unwrap();
    /// let node = fdt.find_node("/test-props").unwrap().unwrap();
    /// let prop = node.property("str-list-prop").unwrap().unwrap();
    /// let mut str_list = prop.as_str_list();
    /// assert_eq!(str_list.next(), Some("first"));
    /// assert_eq!(str_list.next(), Some("second"));
    /// assert_eq!(str_list.next(), Some("third"));
    /// assert_eq!(str_list.next(), None);
    /// ```
    pub fn as_str_list(&self) -> impl Iterator<Item = &'a str> {
        FdtStringListIterator { value: self.value }
    }
}

/// An iterator over the properties of a device tree node.
pub(crate) enum FdtPropIter<'a> {
    Start { fdt: &'a Fdt<'a>, offset: usize },
    Running { fdt: &'a Fdt<'a>, offset: usize },
    Error,
}

impl<'a> Iterator for FdtPropIter<'a> {
    type Item = Result<FdtProperty<'a>, FdtError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Start { fdt, offset } => {
                let mut offset = *offset;
                offset += FDT_TAGSIZE; // Skip FDT_BEGIN_NODE
                offset = match fdt.find_string_end(offset) {
                    Ok(offset) => offset,
                    Err(e) => {
                        *self = Self::Error;
                        return Some(Err(e));
                    }
                };
                offset = Fdt::align_tag_offset(offset);
                *self = Self::Running { fdt, offset };
                self.next()
            }
            Self::Running { fdt, offset } => match Self::try_next(fdt, offset) {
                Some(Ok(val)) => Some(Ok(val)),
                Some(Err(e)) => {
                    *self = Self::Error;
                    Some(Err(e))
                }
                None => None,
            },
            Self::Error => None,
        }
    }
}

impl<'a> FdtPropIter<'a> {
    fn try_next(fdt: &'a Fdt<'a>, offset: &mut usize) -> Option<Result<FdtProperty<'a>, FdtError>> {
        loop {
            let token = match fdt.read_token(*offset) {
                Ok(token) => token,
                Err(e) => return Some(Err(e)),
            };
            match token {
                FdtToken::Prop => {
                    let len = match big_endian::U32::ref_from_prefix(
                        &fdt.data[*offset + FDT_TAGSIZE..],
                    ) {
                        Ok((val, _)) => val.get() as usize,
                        Err(_) => {
                            return Some(Err(FdtError::new(FdtErrorKind::InvalidLength, *offset)));
                        }
                    };
                    let nameoff = match big_endian::U32::ref_from_prefix(
                        &fdt.data[*offset + 2 * FDT_TAGSIZE..],
                    ) {
                        Ok((val, _)) => val.get() as usize,
                        Err(_) => {
                            return Some(Err(FdtError::new(FdtErrorKind::InvalidLength, *offset)));
                        }
                    };
                    let prop_offset = *offset + 3 * FDT_TAGSIZE;
                    *offset = Fdt::align_tag_offset(prop_offset + len);
                    let name = match fdt.string(nameoff) {
                        Ok(name) => name,
                        Err(e) => return Some(Err(e)),
                    };
                    let value = fdt.data.get(prop_offset..prop_offset + len)?;
                    return Some(Ok(FdtProperty {
                        name,
                        value,
                        value_offset: prop_offset,
                    }));
                }
                FdtToken::Nop => *offset += FDT_TAGSIZE,
                _ => return None,
            }
        }
    }
}

struct FdtStringListIterator<'a> {
    value: &'a [u8],
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
