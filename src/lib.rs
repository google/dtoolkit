// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A library for parsing and manipulating Flattened Device Tree (FDT) blobs.
//!
//! This library provides a comprehensive API for working with FDTs, including:
//!
//! - A read-only API for parsing and traversing FDTs without memory allocation.
//! - A read-write API for creating and modifying FDTs in memory.
//! - Support for applying device tree overlays.
//! - Outputting device trees in DTS source format.
//!
//! The library is written purely in Rust and is `#![no_std]` compatible. If
//! you don't need the Device Tree manipulation functionality, the library is
//! also no-`alloc`-compatible.
//!
//! ## Read-Only API
//!
//! The read-only API is centered around the [`Fdt`](fdt::Fdt) struct, which
//! provides a safe, zero-copy view of an FDT blob. You can use this API
//! to traverse the device tree, inspect nodes and properties, and read
//! property values.
//!
//! Note that because the [`Fdt`](fdt::Fdt) struct is zero-copy, certain
//! operations such as node or property lookups run in linear time. If you need
//! to perform these operations often, and you can spare extra memory, it might
//! be beneficial to convert from [`Fdt`](fdt::Fdt) to
//! [`DeviceTree`](model::DeviceTree) first.
//!
//! ## Read-Write API
//!
//! The read-write API is centered around the [`DeviceTree`](model::DeviceTree)
//! struct, which provides a mutable, in-memory representation of a device tree.
//! You can use this API to create new device trees from scratch, modify
//! existing ones, and serialize them back to an FDT blob.
//!
//! Internally it is built upon hash maps, meaning that most lookup and
//! modification operations run in constant time.
//!
//! # Examples
//!
//! ```
//! use dtoolkit::fdt::Fdt;
//! use dtoolkit::model::{DeviceTree, DeviceTreeNode, DeviceTreeProperty};
//!
//! // Create a new device tree from scratch.
//! let mut tree = DeviceTree::new();
//!
//! // Add a child node to the root.
//! let child = DeviceTreeNode::builder("child")
//!     .property(DeviceTreeProperty::new("my-property", "hello\0"))
//!     .build();
//! tree.root.add_child(child);
//!
//! // Serialize the device tree to a DTB.
//! let dtb = tree.to_dtb();
//!
//! // Parse the DTB with the read-only API.
//! let fdt = Fdt::new(&dtb).unwrap();
//!
//! // Find the child node and read its property.
//! let child_node = fdt.find_node("/child").unwrap().unwrap();
//! let prop = child_node.property("my-property").unwrap().unwrap();
//! assert_eq!(prop.as_str().unwrap(), "hello");
//!
//! // Display the DTS
//! println!("{}", fdt);
//! ```

#![cfg_attr(not(test), no_std)]
#![warn(missing_docs, rustdoc::missing_crate_level_docs)]
#![deny(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "write")]
extern crate alloc;

pub mod error;
pub mod fdt;
pub mod memreserve;
#[cfg(feature = "write")]
pub mod model;
pub mod standard;
