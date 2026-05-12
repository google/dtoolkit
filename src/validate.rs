// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Validation functions for Device Tree node and property names.

fn is_valid_node_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == ',' || c == '.' || c == '_' || c == '+' || c == '-'
}

fn is_valid_property_char(c: char) -> bool {
    is_valid_node_char(c) || c == '?' || c == '#'
}

/// Validates a node name according to the Devicetree Specification.
///
/// Returns `true` if valid, `false` otherwise.
pub(crate) fn is_valid_node_name(name: &str) -> bool {
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

    if node_name.is_empty() || node_name.len() > 31 {
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
/// Returns `true` if valid, `false` otherwise.
pub(crate) fn is_valid_property_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 31 {
        return false;
    }

    name.chars().all(is_valid_property_char)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_node_names() {
        assert!(is_valid_node_name(""));
        assert!(is_valid_node_name("cpu"));
        assert!(is_valid_node_name("cpu@0"));
        assert!(is_valid_node_name("memory@10000000"));
        assert!(is_valid_node_name("serial@3f201000"));
        assert!(is_valid_node_name("a,b.c_d+e-f"));
    }

    #[test]
    fn test_invalid_node_names() {
        assert!(!is_valid_node_name("cpu@0@1")); // Too many '@'
        assert!(!is_valid_node_name("cpu@")); // Empty unit address
        assert!(!is_valid_node_name("@0")); // Empty node name
        assert!(!is_valid_node_name("cpu name")); // Invalid space
        assert!(!is_valid_node_name("cpu#name")); // Invalid '#'
        assert!(!is_valid_node_name(
            "this-is-a-very-long-node-name-that-exceeds-thirty-one-characters"
        )); // > 31 chars
    }

    #[test]
    fn test_valid_property_names() {
        assert!(is_valid_property_name("reg"));
        assert!(is_valid_property_name("#address-cells"));
        assert!(is_valid_property_name("device_type"));
        assert!(is_valid_property_name("compatible"));
        assert!(is_valid_property_name("a,b.c_d+e-f?#"));
    }

    #[test]
    fn test_invalid_property_names() {
        assert!(!is_valid_property_name("")); // Empty
        assert!(!is_valid_property_name("my prop")); // Invalid space
        assert!(!is_valid_property_name("my@prop")); // Invalid '@'
        assert!(!is_valid_property_name(
            "this-is-a-very-long-property-name-that-exceeds-thirty-one-characters"
        )); // > 31 chars
    }
}
