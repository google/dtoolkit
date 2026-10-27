// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Validation functions for Device Tree node and property names.

const MAX_NAME_LEN: usize = 31;

const fn is_valid_node_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, ',' | '.' | '_' | '+' | '-')
}

const fn is_valid_property_char(c: char) -> bool {
    is_valid_node_char(c) || matches!(c, '?' | '#')
}

/// Validates a node name according to the Devicetree Specification.
///
/// If `strict_length` is `true`, the node name (before `@`) must not exceed 31
/// characters.
///
/// Returns `true` if valid, `false` otherwise.
pub(crate) fn is_valid_node_name(name: &str, strict_length: bool) -> bool {
    // The root node is a special case with an empty name.
    if name.is_empty() {
        return true;
    }

    let mut parts = name.split('@');
    let node_name = parts.next().unwrap_or("");
    let unit_address = parts.next();

    if parts.next().is_some() {
        return false;
    }

    if node_name.is_empty() || (strict_length && node_name.len() > MAX_NAME_LEN) {
        return false;
    }

    if !node_name.chars().all(is_valid_node_char) {
        return false;
    }

    // Check unit-address if present
    if let Some(addr) = unit_address {
        if addr.is_empty() {
            return false;
        }
        if !addr.chars().all(is_valid_node_char) {
            return false;
        }
    }

    true
}

/// Validates a property name according to the Devicetree Specification.
///
/// If `strict_length` is `true`, the property name must not exceed 31
/// characters.
///
/// Returns `true` if valid, `false` otherwise.
pub(crate) fn is_valid_property_name(name: &str, strict_length: bool) -> bool {
    if name.is_empty() || (strict_length && name.len() > MAX_NAME_LEN) {
        return false;
    }

    name.chars().all(is_valid_property_char)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_node_names() {
        assert!(is_valid_node_name("", true));
        assert!(is_valid_node_name("cpu", true));
        assert!(is_valid_node_name("cpu@0", true));
        assert!(is_valid_node_name("memory@10000000", true));
        assert!(is_valid_node_name("serial@3f201000", true));
        assert!(is_valid_node_name("a,b.c_d+e-f", true));
        assert!(is_valid_node_name(
            "this-is-a-very-long-node-name-that-exceeds-thirty-one-characters",
            false
        ));
    }

    #[test]
    fn invalid_node_names() {
        assert!(!is_valid_node_name("cpu@0@1", false)); // Too many '@'
        assert!(!is_valid_node_name("cpu@", false)); // Empty unit address
        assert!(!is_valid_node_name("@0", false)); // Empty node name
        assert!(!is_valid_node_name("cpu name", false)); // Invalid space
        assert!(!is_valid_node_name("cpu#name", false)); // Invalid '#'
        assert!(!is_valid_node_name(
            "this-is-a-very-long-node-name-that-exceeds-thirty-one-characters",
            true
        )); // > 31 chars
    }

    #[test]
    fn valid_property_names() {
        assert!(is_valid_property_name("reg", true));
        assert!(is_valid_property_name("#address-cells", true));
        assert!(is_valid_property_name("device_type", true));
        assert!(is_valid_property_name("compatible", true));
        assert!(is_valid_property_name("a,b.c_d+e-f?#", true));
        assert!(is_valid_property_name(
            "snps,dis-u2-freeclk-exists-quirk",
            false
        ));
    }

    #[test]
    fn invalid_property_names() {
        assert!(!is_valid_property_name("", false)); // Empty
        assert!(!is_valid_property_name("my prop", false)); // Invalid space
        assert!(!is_valid_property_name("my@prop", false)); // Invalid '@'
        assert!(!is_valid_property_name(
            "this-is-a-very-long-property-name-that-exceeds-thirty-one-characters",
            true
        )); // > 31 chars
    }
}
