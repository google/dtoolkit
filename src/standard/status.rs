// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use core::fmt::{self, Display, Formatter};

use crate::error::StandardError;

/// The value of a `status` property.
///
/// Device-specific failure codes borrow from the property's value.
///
/// ```
/// use dtoolkit::standard::Status;
///
/// assert_eq!(
///     Status::try_from("fail-cache"),
///     Ok(Status::Fail(Some("cache")))
/// );
/// assert_eq!(Status::try_from("fail"), Ok(Status::Fail(None)));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Status<'a> {
    /// The device is operational.
    #[default]
    Okay,
    /// The device is not currently operational, but might become so.
    Disabled,
    /// The device is operational but shouldn't be used.
    Reserved,
    /// The device is not operational, optionally with a device-specific error
    /// code (the suffix of a `fail-` status).
    Fail(Option<&'a str>),
}

impl Display for Status<'_> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Okay => f.write_str("okay"),
            Self::Disabled => f.write_str("disabled"),
            Self::Reserved => f.write_str("reserved"),
            Self::Fail(None) => f.write_str("fail"),
            Self::Fail(Some(code)) => write!(f, "fail-{code}"),
        }
    }
}

impl<'a> TryFrom<&'a str> for Status<'a> {
    type Error = StandardError;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        match s {
            "okay" => Ok(Self::Okay),
            "disabled" => Ok(Self::Disabled),
            "reserved" => Ok(Self::Reserved),
            "fail" => Ok(Self::Fail(None)),
            _ => s
                .strip_prefix("fail-")
                .map(|code| Self::Fail(Some(code)))
                .ok_or(StandardError::InvalidStatus),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_default() {
        assert_eq!(Status::default(), Status::Okay);
    }

    #[test]
    fn status_device_specific_failure() {
        for (value, code) in [
            ("fail-cache", "cache"),
            ("fail-42", "42"),
            ("fail-device-specific", "device-specific"),
            ("fail-", ""),
        ] {
            let status = Status::try_from(value).unwrap();
            assert_eq!(status, Status::Fail(Some(code)));
            assert_eq!(status.to_string(), value);
        }
    }

    #[test]
    fn status_parse_and_display() {
        let cases = [
            (Status::Okay, "okay"),
            (Status::Disabled, "disabled"),
            (Status::Reserved, "reserved"),
            (Status::Fail(None), "fail"),
            (Status::Fail(Some("sss")), "fail-sss"),
        ];

        for (status, s) in cases {
            assert_eq!(Status::try_from(s), Ok(status));
            assert_eq!(status.to_string(), s);
        }
    }

    #[test]
    fn status_invalid() {
        for value in [
            "",
            "unknown",
            "failed",
            "fail_cache",
            "FAIL-cache",
            " fail-cache",
        ] {
            assert_eq!(Status::try_from(value), Err(StandardError::InvalidStatus));
        }
    }
}
