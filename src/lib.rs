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

/// Returns the number of physical cores of the current machine.
/// If not possible on the particular architecture returns same as get() which
/// is the logical CPUs.
#[inline]
pub fn get_physical() -> usize {
    get_num_physical_cpus()
}


#[cfg(not(target_os = "linux"))]
#[inline]
fn get_num_physical_cpus() -> usize {
    // Not implemented, fallback
    get_num_cpus()
}

#[cfg(target_os = "linux")]
fn get_num_physical_cpus() -> usize {
    return read_cpuinfo().unwrap_or_else(|()| get_num_cpus());

    // Like try!(), but always map the error to ()
    macro_rules! try_ {
        ($e:expr) => {
            try!($e.map_err(|_| ()))
        }
    }

    fn read_cpuinfo() -> Result<usize, ()> {
        use std::fs::File;
        use std::io::BufReader;
        use std::io::BufRead;

        let file = try_!(File::open("/proc/cpuinfo"));
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        // vector of physical id, core id pairs
        let mut cores = Vec::new();
        let mut cur_physical = 0;
        loop {
            line.clear();
            try_!(reader.read_line(&mut line));
            if line.is_empty() {
                break;
            }
            let mut parts = line.split(':').map(|s| s.trim());

            // assumption: physical id is listed before core id, if it is present.
            match parts.next() {
                Some("physical id") => {
                    if let Some(id) = parts.next() {
                        cur_physical = try_!(id.parse::<u32>());
                    }
                }
                Some("core id") => {
                    let id = try!(parts.next().ok_or(()));
                    let core_id = try_!(id.parse::<u32>());

                    // insert the tuple in sorted order
                    let t = (cur_physical, core_id);
                    match cores.binary_search(&t) {
                        Ok(_) => { }
                        Err(i) => cores.insert(i, t),
                    }
                }
                Some(_) | None => { }
            }
        }
        let count = cores.len();
        if count == 0 {
            Err(())
        } else {
            Ok(count)
        }
    }
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

#[cfg(any(target_os = "freebsd",
          target_os = "dragonfly",
          target_os = "bitrig",
          target_os = "netbsd"))]
fn get_num_cpus() -> usize {
    let mut cpus: libc::c_uint = 0;
    let mut cpus_size = std::mem::size_of_val(&cpus);

    unsafe {
        cpus = libc::sysconf(libc::_SC_NPROCESSORS_ONLN) as libc::c_uint;
    }
    if cpus < 1 {
        let mut mib = [libc::CTL_HW, libc::HW_NCPU, 0, 0];
        unsafe {
            libc::sysctl(mib.as_mut_ptr(),
                         2,
                         &mut cpus as *mut _ as *mut _,
                         &mut cpus_size as *mut _ as *mut _,
                         0 as *mut _,
                         0);
        }
        if cpus < 1 {
            cpus = 1;
        }
    }
    cpus as usize
}

#[cfg(target_os = "openbsd")]
fn get_num_cpus() -> usize {
    let mut cpus: libc::c_uint = 0;
    let mut cpus_size = std::mem::size_of_val(&cpus);
    let mut mib = [libc::CTL_HW, libc::HW_NCPU, 0, 0];

    unsafe {
        libc::sysctl(mib.as_mut_ptr(),
                     2,
                     &mut cpus as *mut _ as *mut _,
                     &mut cpus_size as *mut _ as *mut _,
                     0 as *mut _,
                     0);
    }
    if cpus < 1 {
        cpus = 1;
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
        target_os = "solaris",
        target_os = "fuchsia",
    )
)]
fn get_num_cpus() -> usize {
    unsafe {
        libc::sysconf(libc::_SC_NPROCESSORS_ONLN) as usize
    }
}
#[cfg(target_os = "emscripten")]
fn get_num_cpus() -> usize {
    1
}

#[test]
fn lower_bound() {
    assert!(get() > 0);
    assert!(get_physical() > 0);
}


#[test]
fn upper_bound() {
    assert!(get() < 236_451);
    assert!(get_physical() < 236_451);
}
