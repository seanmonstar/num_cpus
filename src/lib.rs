//! Replaces the deprecated functionality of std::os::num_cpus.

#![cfg_attr(test, deny(warnings))]
#![deny(missing_docs)]

extern crate libc;

/// Returns the number of CPUs of the current machine.
pub fn get() -> usize {
    unsafe {
        crates_io_get_num_cpus() as usize
    }
}

extern {
    fn crates_io_get_num_cpus() -> libc::c_int;
}

#[test]
fn it_works() {
    assert!(get() > 0);
}
