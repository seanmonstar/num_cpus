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
    use std::io::BufReader;
    use std::io::BufRead;
    use std::fs::File;
    use std::collections::HashSet;

    let file = match File::open("/proc/cpuinfo") {
        Ok(val) => val,
        Err(_) => {return get_num_cpus()},
    };
    let reader = BufReader::new(file);
    let mut set = HashSet::new();
    let mut coreid: u32 = 0;
    let mut physid: u32 = 0;
    let mut chgcount = 0;
    for line in reader.lines().filter_map(|result| result.ok()) {
        let parts: Vec<&str> = line.split(':').map(|s| s.trim()).collect();
        if parts.len() != 2 {
            continue
        }
        if parts[0] == "core id" || parts[0] == "physical id" {
            let value = match parts[1].trim().parse() {
              Ok(val) => val,
              Err(_) => break,
            };
            match parts[0] {
                "core id"     => coreid = value,
                "physical id" => physid = value,
                _ => {},
            }
            chgcount += 1;
        }
        if chgcount == 2 {
            set.insert((physid, coreid));
            chgcount = 0;
        }
    }
    let count = set.len();
    if count == 0 { get_num_cpus() } else { count }
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

#[cfg(feature = "nightly")]
#[feature(asm, naked_functions)]
#[naked]
#[inline(always)]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn popcnt(bits: u64) -> u64 {
    let ret: u64;
    unsafe {
        asm!("popcntq %rdi, %rax"
             : "={rax}"(ret)
             : "{rdi}"(bits)
        );
    };
    ret
}

#[cfg(not(feature = "nightly"))]
#[inline(always)]
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn popcnt(n: u64) -> u64 {
    let mut c: u64 = (n & 0x5555555555555555) + ((n >> 1) & 0x5555555555555555);
    c = (c & 0x3333333333333333) + ((c >> 2) & 0x3333333333333333);
    c = (c & 0x0f0f0f0f0f0f0f0f) + ((c >> 4) & 0x0f0f0f0f0f0f0f0f);
    c = (c & 0x00ff00ff00ff00ff) + ((c >> 8) & 0x00ff00ff00ff00ff);
    c = (c & 0x0000ffff0000ffff) + ((c >> 16) & 0x0000ffff0000ffff);
    c = (c & 0x00000000ffffffff) + ((c >> 32) & 0x00000000ffffffff);
    c
}

#[inline(always)]
#[cfg(all(target_os = "linux", not(target_arch = "x86_64")))]
fn popcnt(n: u64) -> u64 {
    let mut c: u64 = (n & 0x5555555555555555) + ((n >> 1) & 0x5555555555555555);
    c = (c & 0x3333333333333333) + ((c >> 2) & 0x3333333333333333);
    c = (c & 0x0f0f0f0f0f0f0f0f) + ((c >> 4) & 0x0f0f0f0f0f0f0f0f);
    c = (c & 0x00ff00ff00ff00ff) + ((c >> 8) & 0x00ff00ff00ff00ff);
    c = (c & 0x0000ffff0000ffff) + ((c >> 16) & 0x0000ffff0000ffff);
    c = (c & 0x00000000ffffffff) + ((c >> 32) & 0x00000000ffffffff);
    c
}

#[cfg(target_os = "linux")]
fn get_num_cpus() -> usize {
    let mut set:  libc::cpu_set_t = unsafe { std::mem::zeroed() };
    if unsafe { libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &mut set) } == 0 {
        let ptr = unsafe { std::mem::transmute::<&libc::cpu_set_t, &u64>(&set) };
        popcnt(*ptr) as usize
    } else {
        unsafe {
            libc::sysconf(libc::_SC_NPROCESSORS_ONLN) as usize
        }
    }
}

#[cfg(any(
    target_os = "nacl",
    target_os = "macos",
    target_os = "ios",
    target_os = "android",
    target_os = "solaris",
    target_os = "fuchsia")
)]
fn get_num_cpus() -> usize {
    unsafe {
        libc::sysconf(libc::_SC_NPROCESSORS_ONLN) as usize
    }
}
#[cfg(any(target_os = "emscripten", target_os = "redox", target_os = "haiku"))]
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
