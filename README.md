# dtoolkit

[![Rust CI](https://github.com/google/dtoolkit/actions/workflows/rust.yml/badge.svg)](https://github.com/google/dtoolkit/actions/workflows/rust.yml)
[![crates.io](https://img.shields.io/crates/v/dtoolkit.svg)](https://crates.io/crates/dtoolkit)
[![Documentation](https://docs.rs/dtoolkit/badge.svg)](https://docs.rs/dtoolkit)

A library for parsing and manipulating Flattened Device Tree (FDT) blobs.

This library provides a comprehensive API for working with FDTs, including:

- A read-only API for parsing and traversing FDTs without memory allocation.
- A read-write API for creating and modifying FDTs in memory.
- Support for applying device tree overlays.
- Outputting device trees in DTS source format.

The library is written purely in Rust and is `#![no_std]` compatible. If
you don't need the Device Tree manipulation functionality, the library is
also no-`alloc`-compatible.

## License

This software is distributed under the terms of both the MIT license and the
Apache License (Version 2.0).

See LICENSE for details.

## Contributing

If you want to contribute to the project, see details of
[how we accept contributions](CONTRIBUTING.md).

## Fuzzing

Fuzzing helps prevent unexpected panics and ensures correct behavior on
unexpected inputs. The project uses `cargo-fuzz`, which is not part of the
default Rust installation.  It is recommended to run the relevant fuzzer for a
while before pushing a change.

Install `cargo-fuzz` with:

```sh
cargo +nightly install cargo-fuzz
```

Run a fuzz target by name from the repository root:

```sh
cargo +nightly fuzz run <fuzzer name>
```

For example, to run the `parse` fuzzer:

```sh
cargo +nightly fuzz run parse
```

## Disclaimer

This is not an officially supported Google product. This project is not
eligible for the [Google Open Source Software Vulnerability Rewards
Program](https://bughunters.google.com/open-source-security).
