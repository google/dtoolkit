# dtoolkit

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

## Disclaimer

This is not an officially supported Google product. This project is not
eligible for the [Google Open Source Software Vulnerability Rewards
Program](https://bughunters.google.com/open-source-security).