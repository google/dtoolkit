## [0.4.1] - 2026-09-25

### Bug Fixes

- Use glob imports for `zerocopy_derive` to avoid duplicate import errors. ([#77](https://github.com/google/dtoolkit/pull/77))

### Documentation

- Add links to structs mentioned in the `overlay` rustdoc ([#79](https://github.com/google/dtoolkit/pull/79))

## [0.4.0] - 2026-09-25

### Features

- [**breaking**] Allow using FdtMut with other buffer types ([#47](https://github.com/google/dtoolkit/pull/47)). This is a breaking change, as now FdtMut has a new required generic parameter `B: FdtBuffer`.
  - *(fdt_mut)* Support the `heapless` library ([#67](https://github.com/google/dtoolkit/pull/67))
- *(fdt_mut)* [**breaking**] Support shifting data when growing properties ([#48](https://github.com/google/dtoolkit/pull/48)). This removes `FdtMutError::ShiftingRequired` variant which is no longer needed.
- *(fdt_mut)* Add `FdtMut::compact` for removing NOPs ([#53](https://github.com/google/dtoolkit/pull/53))
- *(fdt_mut)* Support for removing nodes ([#66](https://github.com/google/dtoolkit/pull/66))
- *(fdt_mut)* Add `add_property` method ([#68](https://github.com/google/dtoolkit/pull/68))
- *(model)* Add `DeviceTree::find_node` read-only lookup ([#55](https://github.com/google/dtoolkit/pull/55))
- *(model)* Add `DeviceTreeNode::add_child_mut` ([#56](https://github.com/google/dtoolkit/pull/56))
- Added support for DT overlays:
  - *(overlay)* Add read-only DT overlay inspection capabilities ([#57](https://github.com/google/dtoolkit/pull/57))
  - *(overlay)* Implement base node merging and phandle traversal ([#58](https://github.com/google/dtoolkit/pull/58))
  - *(overlay)* Implement phandle offsetting ([#59](https://github.com/google/dtoolkit/pull/59))
  - *(overlay)* Add `OverlayApplier` and target resolution ([#60](https://github.com/google/dtoolkit/pull/60))
  - *(overlay)* Support resolving local fixups ([#61](https://github.com/google/dtoolkit/pull/61))
  - *(overlay)* Support external fixups and symbols ([#62](https://github.com/google/dtoolkit/pull/62))
- Add `ToPropertyValue` trait ([#70](https://github.com/google/dtoolkit/pull/70))
- Add `FromPropertyValue` trait ([#72](https://github.com/google/dtoolkit/pull/72))
- Derive PartialEq, Eq, and Hash for core data structures and errors ([#50](https://github.com/google/dtoolkit/pull/50))

### Bug Fixes

- *(standard)* Typos in docs and `Status::Disabled` string repr ([#51](https://github.com/google/dtoolkit/pull/51))
- *(fdt_mut)* Panic on mutating properties during iteration ([#63](https://github.com/google/dtoolkit/pull/63))

### Other

- Depend on zerocopy-derive directly instead of zerocopy/derive to fix double import issues ([#73](https://github.com/google/dtoolkit/pull/73))

### Documentation

- Add more examples to crate-level docs ([#64](https://github.com/google/dtoolkit/pull/64))

## [0.3.0] - 2026-06-26

### Features

- Introduced in-place FDT editing functionality. This is incomplete and currently supports: 
  - Modifying values without reallocation ([#31](https://github.com/google/dtoolkit/pull/31)) 
  - Shrinking and growing properties ([#32](https://github.com/google/dtoolkit/pull/32))
  - Removing properties directly by replacing with NOPs ([#42](https://github.com/google/dtoolkit/pull/42))

### Bug Fixes

- *(standard nodes)* Use proper name of the `alloc-ranges` property ([#43](https://github.com/google/dtoolkit/pull/43))
- *(standard nodes)* Return error instead of panic when size-cells and address-cells are both 0 ([#44](https://github.com/google/dtoolkit/pull/44))

### Refactor

- [**breaking**] Use GATs (Generic Associated Types) in `Node` and `Property` traits ([#29](https://github.com/google/dtoolkit/pull/29))
  - This allows to specify more precise lifetimes, and allows to implement the `Node` trait for the owned API (the `model` module) directly rather than via reference only
  - This shouldn't be breaking for typical use cases, unless you implement the `Node` or `Propery` traits in your code

## [0.2.1] - 2026-06-16

### Bug Fixes

- Validate `off_mem_rsvmap` and `off_dt_struct` in the FDT parser ([#38](https://github.com/google/dtoolkit/pull/38))
- Return error when accessing data at invalid offsets in the FDT parser instead of panicking ([#40](https://github.com/google/dtoolkit/pull/40))

## [0.2.0] - 2026-06-01

### Features

- [**breaking**] Validate node and property names ([#35](https://github.com/google/dtoolkit/pull/35))

### Refactor

- [**breaking**] Implement generic `From<T>` for `DeviceTreeNode` ([#28](https://github.com/google/dtoolkit/pull/28))

## [0.1.1] - 2026-01-12

### Miscellaneous

- Add docs.rs metadata to Cargo.toml and use `doc_cfg` ([#26](https://github.com/google/dtoolkit/pull/26))

## [0.1.0] - 2026-01-09

### Features

- First version published on crates.io
