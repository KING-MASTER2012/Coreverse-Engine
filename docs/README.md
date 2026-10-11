# Coreverse Engine Documentation

This directory is the home of the technical documentation. It is intentionally
minimal today: the mdBook site (`book.toml` at the repository root) has not been
set up yet, and the `Deploy Documentation` workflow skips itself until it exists.

Until then, the up-to-date sources of truth are:

| Document | Contents |
| :------- | :------- |
| [`README.md`](../README.md) | Overview, architecture, current status, build/test/format instructions, CI |
| [`PROGRESS.md`](../PROGRESS.md) | What changed in each step and the prioritized to-do list |
| [`devkit/scripts/setup/config/README.md`](../devkit/scripts/setup/config/README.md) | Toolchain versions (`tool-versions.json`) and project paths used by bootstrap |

## Planned pages

* Engine architecture
* Rust/C++ FFI architecture (`ffi` crate, `cv-ffi` wrappers)
* Rendering architecture (`RenderDevice`, frame sync, validation)
* Math crate guide (backends, features, conventions)
* Build system and platform support
* Editor architecture
* Contribution guidelines (see also [`.github/CONTRIBUTING.md`](../.github/CONTRIBUTING.md))

Documentation should describe what is implemented, not what is planned.
