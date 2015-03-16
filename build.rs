#![deny(warnings)]

extern crate gcc;

fn main() {
    gcc::compile_library(
        "libnumcpus.a",
        &["extern/num_cpus.c"]
    );
}
