//! Replaces the deprecated functionality of std::os::num_cpus.
#![cfg_attr(test, deny(warnings))]
#![deny(missing_docs)]
#![allow(non_snake_case)]

#[cfg(not(windows))]
extern crate libc;

/// Returns the number of CPUs of the current machine.
#[inline]
pub fn get() -> usize {
    get_num_cpus()
}

#[cfg(windows)]
fn get_num_cpus() -> usize {
    #[repr(C)]
    struct SYSTEM_INFO {
        wProcessorArchitecture: u16,
        wReserved: u16,
        dwPageSize: u32,
        lpMinimumApplicationAddress: *mut u8,
        lpMaximumApplicationAddress: *mut u8,
        dwActiveProcessorMask: *mut u8,
        dwNumberOfProcessors: u32,
        dwProcessorType: u32,
        dwAllocationGranularity: u32,
        wProcessorLevel: u16,
        wProcessorRevision: u16,
    }

    extern "system" {
        fn GetSystemInfo(lpSystemInfo: *mut SYSTEM_INFO);
    }

    unsafe {
        let mut sysinfo: SYSTEM_INFO = std::mem::uninitialized();
        GetSystemInfo(&mut sysinfo);
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
        target_os = "ios",
        target_os = "android",
    )
)]
fn get_num_cpus() -> usize {
    unsafe {
        libc::sysconf(libc::_SC_NPROCESSORS_ONLN) as usize
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
