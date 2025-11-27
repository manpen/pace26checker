# PACE 2026 Checker Crate

This crate implements linters and checkers for the [PACE26 challenge](https://pacechallenge.org/2026/).
It is implemented as a library to be used by other tools;
however, we also offer a very simple binary to manually check instances and solutions.

## Installation

### Fetching from CI

We also build static binaries with every CI run. You can retrieve them by:
 - In the tab [Actions](https://github.com/manpen/pace26checker/actions) click on the latest run of the `master` branch
 - On the summary page find the **Artifacts** and select your system.

### Building from sources

The tool is implemented in Rust and requires that you have a [recent Rust compiler installed](https://rust-lang.org/tools/install/).
Then compiling the tools boils down to executing `cargo build --release` which places the binary in `target/release/checker`.

## Usage

The primary use case will be to check solutions against instances. This can be done by executing:

```bash
checker {path-to-instance} {path-to-solution}
```

You may also lint instances by omitting the solution path.
