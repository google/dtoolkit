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

use crate::Property;
use crate::error::ModelError;

/// A mutable, in-memory representation of a device tree property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceTreeProperty {
    name: String,
    value: Vec<u8>,
}

impl<'a> Property<'a> for &'a DeviceTreeProperty {
    fn name(&self) -> &'a str {
        &self.name
    }

    fn value(&self) -> &'a [u8] {
        &self.value
    }
}

impl DeviceTreeProperty {
    /// Creates a new `DeviceTreeProperty` with the given name and value.
    ///
    /// # Errors
    ///
    /// Returns a [`ModelError::InvalidPropertyName`] if the property name is
    /// invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use dtoolkit::Property;
    /// use dtoolkit::model::DeviceTreeProperty;
    ///
    /// let prop = DeviceTreeProperty::new("my-prop", vec![1, 2, 3, 4]).unwrap();
    /// assert_eq!((&prop).name(), "my-prop");
    /// assert_eq!((&prop).value(), &[1, 2, 3, 4]);
    /// ```
    pub fn new(name: impl Into<String>, value: impl Into<Vec<u8>>) -> Result<Self, ModelError> {
        let name = name.into();
        if !crate::validate::is_valid_property_name(&name) {
            return Err(ModelError::InvalidPropertyName(name));
        }
        Ok(Self {
            name,
            value: value.into(),
        })
    }

    /// Creates a new `DeviceTreeProperty` with the given name and value without
    /// validation.
    #[must_use]
    pub fn new_unchecked(name: impl Into<String>, value: impl Into<Vec<u8>>) -> Self {
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
    /// let mut prop = DeviceTreeProperty::new("my-prop", vec![1, 2, 3, 4]).unwrap();
    /// prop.set_value(vec![5, 6, 7, 8]);
    /// assert_eq!((&prop).value(), &[5, 6, 7, 8]);
    /// ```
    pub fn set_value(&mut self, value: impl Into<Vec<u8>>) {
        self.value = value.into();
    }
}

impl<'a, T: Property<'a>> From<T> for DeviceTreeProperty {
    fn from(prop: T) -> Self {
        let name = prop.name().to_string();
        let value = prop.value().to_vec();
        DeviceTreeProperty { name, value }
    }
}
