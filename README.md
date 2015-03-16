# num_cpus

A replacement for the deprecated `std::os::num_cpus`.

## Usage

Add to Cargo.toml:

```
[dependencies]
num_cpus = "*"
```

In your `main.rs` or `lib.rs`:

```rust
extern crate num_cpus;

// elsewhere
let num = num_cpus::get();
```

## License

MIT
