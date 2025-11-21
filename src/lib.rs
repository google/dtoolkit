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

#![no_std]
#![warn(missing_docs, rustdoc::missing_crate_level_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]
