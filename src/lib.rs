//! Replaces the deprecated functionality of std::os::num_cpus.
#![cfg_attr(test, deny(warnings))]
#![deny(missing_docs)]
#![allow(non_snake_case)]

extern crate libc;
#[cfg(windows)]
extern crate winapi;
#[cfg(windows)]
extern crate kernel32;

/// Returns the number of CPUs of the current machine.
#[inline]
pub fn get() -> usize {
    get_num_cpus()
}

#[cfg(windows)]
fn get_num_cpus() -> usize {
    unsafe {
        let mut sysinfo: winapi::SYSTEM_INFO = ::std::mem::uninitialized();
        kernel32::GetSystemInfo(&mut sysinfo);
        sysinfo.dwNumberOfProcessors as usize
    }
}

#[cfg(
    any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "bitrig",
        target_os = "openbsd",
        target_os = "netbsd"
    )
)]
fn get_num_cpus() -> usize {
    use libc::{c_int, c_uint};
    use libc::sysctl;
    use std::ptr;

    //XXX: uplift to libc?
    const CTL_HW: c_int = 6;
    const HW_AVAILCPU: c_int = 25;
    const HW_NCPU: c_int = 3;

    let mut cpus: c_uint = 0;
    let mut CPUS_SIZE = ::std::mem::size_of::<c_uint>();
    let mut mib: [c_int; 4] = [CTL_HW, HW_AVAILCPU, 0, 0];

    unsafe {
        sysctl(mib.as_mut_ptr(), 2,
               &mut cpus as *mut _ as *mut _, &mut CPUS_SIZE as *mut _ as *mut _,
               ptr::null_mut(), 0);
    }

    if cpus < 1 {
        mib[1] = HW_NCPU;
        unsafe {
            sysctl(mib.as_mut_ptr(), 2,
                   &mut cpus as *mut _ as *mut _, &mut CPUS_SIZE as *mut _ as *mut _,
                   ptr::null_mut(), 0);
        }
        if cpus < 1 {
            cpus = 1;
        }
    }

    cpus as usize
}

#[cfg(
    any(
        target_os = "linux",
        target_os = "nacl",
        target_os = "macos",
        target_os = "ios"
    )
)]
fn get_num_cpus() -> usize {
    unsafe {
        libc::sysconf(libc::_SC_NPROCESSORS_ONLN) as usize
    }
}

#[cfg(target_os= "android")]
fn get_num_cpus() -> usize {
    //to-do: replace with libc::_SC_NPROCESSORS_ONLN once available
    unsafe {
        libc::sysconf(97) as usize
    }
}

/// Returns the number of physical CPUs of the current machine.
/// Currently only Mac OSX is supported.
#[inline]
pub fn get_physical() -> usize {
    get_physical_num_cpus()
}

#[cfg(target_os="macos")]
fn get_physical_num_cpus() -> usize {
    use libc::size_t;
    use libc::sysctlbyname;
    use std::ptr;
    use libc::c_void;

    static HW_PHYSICALCPU: &'static [i8] = &[104i8, 119, 46, 112, 104, 121, 115, 105, 99, 97, 108, 99, 112, 117, 0];

    unsafe {
        let name = HW_PHYSICALCPU.as_ptr();
        let mut count = 0;
        let mut count_len = ::std::mem::size_of::<size_t>();
        sysctlbyname(name, &mut count as *mut _ as *mut c_void, &mut count_len as *mut _, ptr::null_mut(), 0);
        count as usize
    }
}

#[test]
fn lower_bound() {
    assert!(get() > 0);
}


#[test]
fn upper_bound() {
    assert!(get() < 236_451);
}
