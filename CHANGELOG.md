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
