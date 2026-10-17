# Coreverse Engine Documentation

This directory holds the sources of the documentation book (mdBook): the pages
are in [`src/`](src/SUMMARY.md) and the configuration is `book.toml` at the
repository root. Build it locally with `mdbook build` (output: `./book`, git-ignored)
or `mdbook serve`; the `Deploy Documentation` workflow publishes it to GitHub Pages.

Besides the book, these are the up-to-date sources of truth:

| Document                                                                            | Contents                                                                       |
|:------------------------------------------------------------------------------------|:-------------------------------------------------------------------------------|
| [`README.md`](../README.md)                                                         | Overview, architecture, current status, build/test/format instructions, CI     |
| [`PROGRESS.md`](../PROGRESS.md)                                                     | What changed in each step and the prioritized to-do list                       |
| [`devkit/scripts/setup/config/README.md`](../devkit/scripts/setup/config/README.md) | Toolchain versions (`tool-versions.json`) and project paths used by bootstrap  |

## Pages

Introduction, architecture, building and testing, FFI, rendering, math, editor and CI.
For contribution guidelines see [`.github/CONTRIBUTING.md`](../.github/CONTRIBUTING.md).

Documentation should describe what is implemented, not what is planned.
