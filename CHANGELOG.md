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
