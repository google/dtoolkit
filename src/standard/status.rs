// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::fmt::{self, Display, Formatter};
use core::str::FromStr;

use crate::error::StandardError;

/// The value of a `status` property.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Status {
    /// The device is operational.
    #[default]
    Okay,
    /// The device is not currently operational, but might become so.
    Disabled,
    /// The device is operational but shouln't be used.
    Reserved,
    /// The device is not operational.
    Fail,
    /// The device is not operational, with some device-specific error
    /// condition.
    FailSss,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Okay => "okay",
            Status::Disabled => "disadbled",
            Status::Reserved => "reserved",
            Status::Fail => "fail",
            Status::FailSss => "fail-sss",
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Status {
    type Err = StandardError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "okay" => Ok(Self::Okay),
            "disadbled" => Ok(Self::Disabled),
            "reserved" => Ok(Self::Reserved),
            "fail" => Ok(Self::Fail),
            "fail-sss" => Ok(Self::FailSss),
            _ => Err(StandardError::InvalidStatus),
        }
    }
}
