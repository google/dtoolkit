// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::str;

use crate::error::{FdtParseError, PropertyError};
use crate::fdt::FdtProperty;

/// A mutable, in-memory representation of a device tree property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceTreeProperty {
    name: String,
    value: Vec<u8>,
}

impl DeviceTreeProperty {
    /// Creates a new `DeviceTreeProperty` with the given name and value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::model::DeviceTreeProperty;
    /// let prop = DeviceTreeProperty::new("my-prop", vec![1, 2, 3, 4]);
    /// assert_eq!(prop.name(), "my-prop");
    /// assert_eq!(prop.value(), &[1, 2, 3, 4]);
    /// ```
    #[must_use]
    pub fn new(name: impl Into<String>, value: impl Into<Vec<u8>>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Returns the name of this property.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the value of this property.
    #[must_use]
    pub fn value(&self) -> &[u8] {
        &self.value
    }

    /// Sets the value of this property.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::model::DeviceTreeProperty;
    /// let mut prop = DeviceTreeProperty::new("my-prop", vec![1, 2, 3, 4]);
    /// prop.set_value(vec![5, 6, 7, 8]);
    /// assert_eq!(prop.value(), &[5, 6, 7, 8]);
    /// ```
    pub fn set_value(&mut self, value: impl Into<Vec<u8>>) {
        self.value = value.into();
    }

    /// Returns the value of this property as a `u32`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::model::DeviceTreeProperty;
    /// let prop = DeviceTreeProperty::new("my-prop", 1234u32.to_be_bytes());
    /// assert_eq!(prop.as_u32(), Ok(1234));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the property's value is not 4 bytes long.
    pub fn as_u32(&self) -> Result<u32, PropertyError> {
        self.value
            .as_slice()
            .try_into()
            .map(u32::from_be_bytes)
            .map_err(|_| PropertyError::InvalidLength)
    }

    /// Returns the value of this property as a string.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dtoolkit::model::DeviceTreeProperty;
    /// let prop = DeviceTreeProperty::new("my-prop", "hello");
    /// assert_eq!(prop.as_str(), Ok("hello"));
    /// ```
    /// # Errors
    ///
    /// Returns an error if the property's value is not a valid UTF-8 string.
    pub fn as_str(&self) -> Result<&str, PropertyError> {
        str::from_utf8(&self.value)
            .map(|s| s.trim_end_matches('\0'))
            .map_err(|_| PropertyError::InvalidString)
    }
}

impl<'a> TryFrom<FdtProperty<'a>> for DeviceTreeProperty {
    type Error = FdtParseError;

    fn try_from(prop: FdtProperty<'a>) -> Result<Self, Self::Error> {
        let name = prop.name().to_string();
        let value = prop.value().to_vec();
        Ok(DeviceTreeProperty { name, value })
    }
}
