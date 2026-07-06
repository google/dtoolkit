// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

#[cfg(feature = "arrayvec07")]
use arrayvec::ArrayVec;

/// A trait for buffers that can hold and mutate a Flattened Device Tree.
pub trait FdtBuffer: AsRef<[u8]> + AsMut<[u8]> {}

impl FdtBuffer for &mut [u8] {}

#[cfg(feature = "alloc")]
impl FdtBuffer for Vec<u8> {}

#[cfg(feature = "arrayvec07")]
impl<const N: usize> FdtBuffer for ArrayVec<u8, N> {}
