# num_cpus

[![crates.io](https://img.shields.io/crates/v/num_cpus.svg)](https://crates.io/crates/num_cpus)
[![CI Status](https://github.com/seanmonstar/num_cpus/actions/workflows/ci.yml/badge.svg)](https://github.com/seanmonstar/num_cpus/actions)

- [Documentation](https://docs.rs/num_cpus)
- [CHANGELOG](CHANGELOG.md)

Count the number of CPUs on the current machine.

## Usage

Add to Cargo.toml:

```toml
[dependencies]
num_cpus = "1.0"
```

In your `main.rs` or `lib.rs`:

```rust
extern crate num_cpus;

// count logical cores this process could try to use
let num = num_cpus::get();
```

## Rust version support

`num_cpus` supports Rust 1.13 and newer.

If your project can require Rust 1.59 or newer, similar support exists in the
Rust standard library, via the
[`std::thread::available_parallelism`](https://doc.rust-lang.org/std/thread/fn.available_parallelism.html)
function. `num_cpus` provides the same functionality for projects that need to
support older versions of Rust.
