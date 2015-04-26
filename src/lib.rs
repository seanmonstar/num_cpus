//! Replaces the deprecated functionality of std::os::num_cpus.
#![cfg_attr(test, deny(warnings))]
#![deny(missing_docs)]

extern crate libc;

/// Returns the number of CPUs of the current machine.
#[inline]
pub fn get() -> usize {
    get_num_cpus()
}

#[cfg(windows)]
fn get_num_cpus() -> usize {
    unsafe {
        let mut sysinfo: libc::SYSTEM_INFO = ::std::mem::uninitialized();
        libc::GetSystemInfo(&mut sysinfo);
        sysinfo.dwNumberOfProcessors as usize
    }
}

#[cfg(
    any(
        target_os = "freebsd",
        target_os = "dragonfly",
        target_os = "bitrig",
        target_os = "openbsd"
    )
)]
fn get_num_cpus() -> usize {
    use libc::{c_int, c_uint};
    use libc::funcs::bsd44::sysctl;

    //XXX: uplift to libc?
    const CTL_HW: c_int = 6;
    const HW_AVAILCPU: c_int = 25;
    const HW_NCPU: c_int = 3;

    let mut cpus: c_uint = 0;
    let mut CPUS_SIZE = ::std::mem::size_of::<c_uint>();
    let mut mib: [c_int; 4] = [CTL_HW, HW_AVAILCPU, 0, 0];

    unsafe {
        sysctl(mib.as_mut_ptr(), 2, &mut cpus, &mut CPUS_SIZE, &mut (), 0);
    }

    if cpus < 1 {
        mib[1] = HW_NCPU;
        unsafe {
            sysctl(mib.as_mut_ptr(), 2, &mut cpus, &mut CPUS_SIZE, &mut (), 0);
        }
        if cpus < 1 {
            cpus = 1;
        }
    }

    cpus as usize
}

#[cfg(target_os= "linux")]
fn get_num_cpus() -> usize {
    //to-do: replace with libc::_SC_NPROCESSORS_ONLN once available
    unsafe {
        libc::sysconf(84) as usize
    }
}
#[cfg(target_os = "nacl")]
fn get_num_cpus() -> usize {
    //to-do: replace with libc::_SC_NPROCESSORS_ONLN once available
    unsafe {
        libc::sysconf(1) as usize
    }
}

#[cfg(
    any(
        target_os = "macos",
        target_os = "ios"
    )
)]
fn get_num_cpus() -> usize {
    //to-do: replace with libc::_SC_NPROCESSORS_ONLN once available
    unsafe {
        libc::sysconf(58) as usize
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
