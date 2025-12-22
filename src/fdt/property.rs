// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A read-only API for inspecting a device tree property.

use core::ffi::CStr;
use core::fmt::{self, Display, Formatter};
use core::ops::{BitOr, Shl};

use zerocopy::{FromBytes, big_endian};

use super::{FDT_TAGSIZE, Fdt, FdtToken};
use crate::error::{FdtError, FdtErrorKind, FdtParseError};

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
    pub fn as_u32(&self) -> Result<u32, FdtParseError> {
        big_endian::U32::ref_from_bytes(self.value)
            .map(|val| val.get())
            .map_err(|_e| FdtParseError::new(FdtErrorKind::InvalidLength, self.value_offset))
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
    pub fn as_u64(&self) -> Result<u64, FdtParseError> {
        big_endian::U64::ref_from_bytes(self.value)
            .map(|val| val.get())
            .map_err(|_e| FdtParseError::new(FdtErrorKind::InvalidLength, self.value_offset))
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
    pub fn as_str(&self) -> Result<&'a str, FdtParseError> {
        let cstr = CStr::from_bytes_with_nul(self.value)
            .map_err(|_| FdtParseError::new(FdtErrorKind::InvalidString, self.value_offset))?;
        cstr.to_str()
            .map_err(|_| FdtParseError::new(FdtErrorKind::InvalidString, self.value_offset))
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
    pub fn as_str_list(&self) -> impl Iterator<Item = &'a str> + use<'a> {
        FdtStringListIterator { value: self.value }
    }

    pub(crate) fn as_prop_encoded_array<const N: usize>(
        &self,
        fields_cells: [usize; N],
    ) -> Result<impl Iterator<Item = [Cells<'a>; N]> + use<'a, N>, FdtError> {
        let chunk_cells = fields_cells.iter().sum();
        let chunk_bytes = chunk_cells * size_of::<u32>();
        if !self.value.len().is_multiple_of(chunk_bytes) {
            return Err(FdtError::PropEncodedArraySizeMismatch {
                size: self.value.len(),
                chunk: chunk_cells,
            });
        }
        Ok(self.value.chunks_exact(chunk_bytes).map(move |chunk| {
            let mut cells = <[big_endian::U32]>::ref_from_bytes(chunk)
                .expect("chunk should be a multiple of 4 bytes because of chunks_exact");
            fields_cells.map(|field_cells| {
                let field;
                (field, cells) = cells.split_at(field_cells);
                Cells(field)
            })
        }))
    }

    pub(crate) fn fmt(&self, f: &mut Formatter, indent: usize) -> fmt::Result {
        write!(f, "{:indent$}{}", "", self.name, indent = indent)?;

        if self.value.is_empty() {
            writeln!(f, ";")?;
            return Ok(());
        }

        let is_printable = self
            .value
            .iter()
            .all(|&ch| ch.is_ascii_graphic() || ch == b' ' || ch == 0);
        let has_empty = self.value.windows(2).any(|window| window == [0, 0]);
        if is_printable && self.value.ends_with(&[0]) && !has_empty {
            let mut strings = self.as_str_list();
            if let Some(first) = strings.next() {
                write!(f, " = \"{first}\"")?;
                for s in strings {
                    write!(f, ", \"{s}\"")?;
                }
                writeln!(f, ";")?;
                return Ok(());
            }
        }

        if self.value.len().is_multiple_of(4) {
            write!(f, " = <")?;
            for (i, chunk) in self.value.chunks_exact(4).enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                let val = u32::from_be_bytes(
                    chunk
                        .try_into()
                        .expect("u32::from_be_bytes() should always succeed with 4 bytes"),
                );
                write!(f, "0x{val:02x}")?;
            }
            writeln!(f, ">;")?;
        } else {
            write!(f, " = [")?;
            for (i, byte) in self.value.iter().enumerate() {
                if i > 0 {
                    write!(f, " ")?;
                }
                write!(f, "{byte:02x}")?;
            }
            writeln!(f, "];")?;
        }

        Ok(())
    }
}

impl Display for FdtProperty<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.fmt(f, 0)
    }
}

/// An iterator over the properties of a device tree node.
pub(crate) enum FdtPropIter<'a> {
    Start { fdt: Fdt<'a>, offset: usize },
    Running { fdt: Fdt<'a>, offset: usize },
    Error,
}

impl<'a> Iterator for FdtPropIter<'a> {
    type Item = Result<FdtProperty<'a>, FdtParseError>;

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
                *self = Self::Running { fdt: *fdt, offset };
                self.next()
            }
            Self::Running { fdt, offset } => match Self::try_next(*fdt, offset) {
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
    fn try_next(
        fdt: Fdt<'a>,
        offset: &mut usize,
    ) -> Option<Result<FdtProperty<'a>, FdtParseError>> {
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
                            return Some(Err(FdtParseError::new(
                                FdtErrorKind::InvalidLength,
                                *offset,
                            )));
                        }
                    };
                    let nameoff = match big_endian::U32::ref_from_prefix(
                        &fdt.data[*offset + 2 * FDT_TAGSIZE..],
                    ) {
                        Ok((val, _)) => val.get() as usize,
                        Err(_) => {
                            return Some(Err(FdtParseError::new(
                                FdtErrorKind::InvalidLength,
                                *offset,
                            )));
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

/// An integer value split into several big-endian u32 parts.
///
/// This is generally used in prop-encoded-array properties.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Cells<'a>(pub(crate) &'a [big_endian::U32]);

impl Cells<'_> {
    /// Converts the value to the given integer type.
    ///
    /// # Errors
    ///
    /// Returns `FdtError::TooManyCells` if the value has too many cells to fit
    /// in the given type.
    pub fn to_int<T: Default + From<u32> + Shl<usize, Output = T> + BitOr<Output = T>>(
        self,
    ) -> Result<T, FdtError> {
        if size_of::<T>() < self.0.len() * size_of::<u32>() {
            Err(FdtError::TooManyCells {
                cells: self.0.len(),
            })
        } else if let [size] = self.0 {
            Ok(size.get().into())
        } else {
            let mut value = Default::default();
            for cell in self.0 {
                value = value << 32 | cell.get().into();
            }
            Ok(value)
        }
    }
}

impl Display for Cells<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("0x")?;
        for part in self.0 {
            write!(f, "{part:08x}")?;
        }
        Ok(())
    }
}
