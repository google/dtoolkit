// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::ffi::CStr;
use core::str;

use zerocopy::{FromBytes, big_endian};

use crate::Property;
use crate::error::PropertyError;

/// A mutable, in-memory representation of a device tree property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceTreeProperty {
    name: String,
    value: Vec<u8>,
}

impl<'a> Property for &'a DeviceTreeProperty {
    type Str = &'a str;
    type StrList = crate::values::FdtStringListIterator<'a>;
    type PropEncodedArray<const N: usize> = crate::values::PropEncodedArrayIterator<'a, N>;
    type CellsItem = crate::Cells<'a>;

    fn name(&self) -> &'a str {
        &self.name
    }

    fn value(&self) -> &'a [u8] {
        &self.value
    }

    fn as_cells(&self) -> Result<crate::Cells<'a>, PropertyError> {
        Ok(crate::Cells(
            <[big_endian::U32]>::ref_from_bytes(&self.value)
                .map_err(|_| PropertyError::InvalidLength)?,
        ))
    }

    fn as_str(&self) -> Result<&'a str, PropertyError> {
        let cstr =
            CStr::from_bytes_with_nul(&self.value).map_err(|_| PropertyError::InvalidString)?;
        cstr.to_str().map_err(|_| PropertyError::InvalidString)
    }

    fn as_str_list(&self) -> crate::values::FdtStringListIterator<'a> {
        crate::values::FdtStringListIterator { value: &self.value }
    }

    fn as_prop_encoded_array<const N: usize>(
        &self,
        fields_cells: [usize; N],
    ) -> Result<crate::values::PropEncodedArrayIterator<'a, N>, PropertyError> {
        crate::values::PropEncodedArrayIterator::new(&self.value, fields_cells)
    }
}

impl DeviceTreeProperty {
    /// Creates a new `DeviceTreeProperty` with the given name and value.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::Property;
    /// use dtoolkit::model::DeviceTreeProperty;
    ///
    /// let prop = DeviceTreeProperty::new("my-prop", vec![1, 2, 3, 4]);
    /// assert_eq!((&prop).name(), "my-prop");
    /// assert_eq!((&prop).value(), &[1, 2, 3, 4]);
    /// ```
    #[must_use]
    pub fn new(name: impl Into<String>, value: impl Into<Vec<u8>>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Sets the value of this property.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::Property;
    /// use dtoolkit::model::DeviceTreeProperty;
    ///
    /// let mut prop = DeviceTreeProperty::new("my-prop", vec![1, 2, 3, 4]);
    /// prop.set_value(vec![5, 6, 7, 8]);
    /// assert_eq!((&prop).value(), &[5, 6, 7, 8]);
    /// ```
    pub fn set_value(&mut self, value: impl Into<Vec<u8>>) {
        self.value = value.into();
    }
}

impl DeviceTreeProperty {
    /// Creates a new [`DeviceTreeProperty`] from any type that implements
    /// [`Property`].
    pub fn from_property<T: Property>(prop: &T) -> Self {
        let name = prop.name().to_string();
        let value = prop.value().to_vec();
        DeviceTreeProperty { name, value }
    }
}
