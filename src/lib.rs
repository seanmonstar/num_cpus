//! A crate with utilities to determine the number of CPUs available on the
//! current system.
//!
//! Sometimes the CPU will exaggerate the number of CPUs it contains, because it can use
//! [processor tricks] to deliver increased performance when there are more threads. This 
//! crate provides methods to get both the logical and physical numbers of cores.
//!
//! This information can be used as a guide to how many tasks can be run in parallel.
//! There are many properties of the system architecture that will affect parallelism,
//! for example memory access speeds (for all the caches and RAM) and the physical
//! architecture of the processor, so the number of CPUs should be used as a rough guide
//! only.
//!
//!
//! ## Examples
//!
//! Fetch the number of logical CPUs.
//!
//! ```
//! let cpus = num_cpus::get();
//! ```
//!
//! See [`rayon::Threadpool`] for an example of where the number of CPUs could be
//! used when setting up parallel jobs (Where the threadpool example uses a fixed
//! number 8, it could use the number of CPUs).
//!
//! [processor tricks]: https://en.wikipedia.org/wiki/Simultaneous_multithreading
//! [`rayon::ThreadPool`]: https://docs.rs/rayon/1.*/rayon/struct.ThreadPool.html
#![cfg_attr(test, deny(warnings))]
#![deny(missing_docs)]
#![doc(html_root_url = "https://docs.rs/num_cpus/1.11.1")]
#![allow(non_snake_case)]

#[cfg(not(windows))]
extern crate libc;

#[cfg(target_os = "hermit")]
extern crate hermit_abi;

#[cfg(test)]
#[macro_use]
extern crate doc_comment;

#[cfg(test)]
doctest!("../README.md");

/// Returns the number of available CPUs of the current system.
///
/// This function will get the number of logical cores. Sometimes this is different from the number
/// of physical cores (See [Simultaneous multithreading on Wikipedia][smt]).
///
/// # Examples
///
/// ```
/// let cpus = num_cpus::get();
/// if cpus > 1 {
///     println!("We are on a multicore system with {} CPUs", cpus);
/// } else {
///     println!("We are on a single core system");
/// }
/// ```
///
/// # Note
///
/// This will check [sched affinity] on Linux, showing a lower number of CPUs if the current 
/// thread does not have access to all the computer's CPUs. 
///
/// [smt]: https://en.wikipedia.org/wiki/Simultaneous_multithreading
/// [sched affinity]: http://www.gnu.org/software/libc/manual/html_node/CPU-Affinity.html
#[inline]
pub fn get() -> usize {
    get_num_cpus()
}

/// Returns the number of physical cores of the current system.
///
/// # Note
///
/// Physical count is supported only on Linux, mac OS and Windows platforms.
/// On other platforms, or if the physical count fails on supported platforms,
/// this function returns the same as [`get()`], which is the number of logical
/// CPUS.
///
/// # Examples
///
/// ```
/// let logical_cpus = num_cpus::get();
/// let physical_cpus = num_cpus::get_physical();
/// if logical_cpus > physical_cpus {
///     println!("We have simultaneous multithreading with about {:.2} \
///               logical cores to 1 physical core.", 
///               (logical_cpus as f64) / (physical_cpus as f64));
/// } else if logical_cpus == physical_cpus {
///     println!("Either we don't have simultaneous multithreading, or our \
///               system doesn't support getting the number of physical CPUs.");
/// } else {
///     println!("We have less logical CPUs than physical CPUs, maybe we only have access to \
///               some of the CPUs on our system.");
/// }
/// ```
///
/// [`get()`]: fn.get.html
#[inline]
pub fn get_physical() -> usize {
    get_num_physical_cpus()
}


#[cfg(not(any(target_os = "linux", target_os = "windows", target_os="macos")))]
#[inline]
fn get_num_physical_cpus() -> usize {
    // Not implemented, fall back
    get_num_cpus()
}

#[cfg(target_os = "windows")]
fn get_num_physical_cpus() -> usize {
    match get_num_physical_cpus_windows() {
        Some(num) => num,
        None => get_num_cpus()
    }
}

#[cfg(target_os = "windows")]
fn get_num_physical_cpus_windows() -> Option<usize> {
    // Inspired by https://msdn.microsoft.com/en-us/library/ms683194

    use std::ptr;
    use std::mem;

    #[allow(non_upper_case_globals)]
    const RelationProcessorCore: u32 = 0;

    #[repr(C)]
    #[allow(non_camel_case_types)]
    struct SYSTEM_LOGICAL_PROCESSOR_INFORMATION {
        mask: usize,
        relationship: u32,
        _unused: [u64; 2]
    }

    extern "system" {
        fn GetLogicalProcessorInformation(
            info: *mut SYSTEM_LOGICAL_PROCESSOR_INFORMATION,
            length: &mut u32
        ) -> u32;
    }

    // First we need to determine how much space to reserve.

    // The required size of the buffer, in bytes.
    let mut needed_size = 0;

    unsafe {
        GetLogicalProcessorInformation(ptr::null_mut(), &mut needed_size);
    }

    let struct_size = mem::size_of::<SYSTEM_LOGICAL_PROCESSOR_INFORMATION>() as u32;

    // Could be 0, or some other bogus size.
    if needed_size == 0 || needed_size < struct_size || needed_size % struct_size != 0 {
        return None;
    }

    let count = needed_size / struct_size;

    // Allocate some memory where we will store the processor info.
    let mut buf = Vec::with_capacity(count as usize);

    let result;

    unsafe {
        result = GetLogicalProcessorInformation(buf.as_mut_ptr(), &mut needed_size);
    }

    // Failed for any reason.
    if result == 0 {
        return None;
    }

    let count = needed_size / struct_size;

    unsafe {
        buf.set_len(count as usize);
    }

    let phys_proc_count = buf.iter()
        // Only interested in processor packages (physical processors.)
        .filter(|proc_info| proc_info.relationship == RelationProcessorCore)
        .count();

    if phys_proc_count == 0 {
        None
    } else {
        Some(phys_proc_count)
    }
}

#[cfg(target_os = "linux")]
fn get_num_physical_cpus() -> usize {
    use std::collections::HashMap;
    use std::fs::File;
    use std::io::BufRead;
    use std::io::BufReader;

    let file = match File::open("/proc/cpuinfo") {
        Ok(val) => val,
        Err(_) => return get_num_cpus(),
    };
    let reader = BufReader::new(file);
    let mut map = HashMap::new();
    let mut physid: u32 = 0;
    let mut cores: usize = 0;
    let mut chgcount = 0;
    for line in reader.lines().filter_map(|result| result.ok()) {
        let mut it = line.split(':');
        let (key, value) = match (it.next(), it.next()) {
            (Some(key), Some(value)) => (key.trim(), value.trim()),
            _ => continue,
        };
        if key == "physical id" {
            match value.parse() {
                Ok(val) => physid = val,
                Err(_) => break,
            };
            chgcount += 1;
        }
        if key == "cpu cores" {
            match value.parse() {
                Ok(val) => cores = val,
                Err(_) => break,
            };
            chgcount += 1;
        }
        if chgcount == 2 {
            map.insert(physid, cores);
            chgcount = 0;
        }
    }
    let count = map.into_iter().fold(0, |acc, (_, cores)| acc + cores);

    if count == 0 {
        get_num_cpus()
    } else {
        count
    }
}

#[cfg(all(windows, not(feature = "extended")))]
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
        let mut sysinfo: SYSTEM_INFO = std::mem::zeroed();
        GetSystemInfo(&mut sysinfo);
        sysinfo.dwNumberOfProcessors as usize
    }
}

#[cfg(any(target_os = "freebsd",
          target_os = "dragonfly",
          target_os = "netbsd"))]
fn get_num_cpus() -> usize {
    use std::ptr;

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
                         ptr::null_mut(),
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
    use std::ptr;

    let mut cpus: libc::c_uint = 0;
    let mut cpus_size = std::mem::size_of_val(&cpus);
    let mut mib = [libc::CTL_HW, libc::HW_NCPU, 0, 0];

    unsafe {
        libc::sysctl(mib.as_mut_ptr(),
                     2,
                     &mut cpus as *mut _ as *mut _,
                     &mut cpus_size as *mut _ as *mut _,
                     ptr::null_mut(),
                     0);
    }
    if cpus < 1 {
        cpus = 1;
    }
    cpus as usize
}


#[cfg(target_os = "macos")]
fn get_num_physical_cpus() -> usize {
    use std::ffi::CStr;
    use std::ptr;

    let mut cpus: i32 = 0;
    let mut cpus_size = std::mem::size_of_val(&cpus);

    let sysctl_name = CStr::from_bytes_with_nul(b"hw.physicalcpu\0")
        .expect("byte literal is missing NUL");

    unsafe {
        if 0 != libc::sysctlbyname(sysctl_name.as_ptr(),
                                   &mut cpus as *mut _ as *mut _,
                                   &mut cpus_size as *mut _ as *mut _,
                                   ptr::null_mut(),
                                   0) {
            return get_num_cpus();
        }
    }
    cpus as usize
}

#[cfg(target_os = "linux")]
fn get_num_cpus() -> usize {
    let mut set:  libc::cpu_set_t = unsafe { std::mem::zeroed() };
    if unsafe { libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &mut set) } == 0 {
        let mut count: u32 = 0;
        for i in 0..libc::CPU_SETSIZE as usize {
            if unsafe { libc::CPU_ISSET(i, &set) } {
                count += 1
            }
        }
        count as usize
    } else {
        let cpus = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) };
        if cpus < 1 {
            1
        } else {
            cpus as usize
        }
    }
}

#[cfg(any(
    target_os = "nacl",
    target_os = "macos",
    target_os = "ios",
    target_os = "android",
    target_os = "solaris",
    target_os = "illumos",
    target_os = "fuchsia")
)]
fn get_num_cpus() -> usize {
    // On ARM targets, processors could be turned off to save power.
    // Use `_SC_NPROCESSORS_CONF` to get the real number.
    #[cfg(any(target_arch = "arm", target_arch = "aarch64"))]
    const CONF_NAME: libc::c_int = libc::_SC_NPROCESSORS_CONF;
    #[cfg(not(any(target_arch = "arm", target_arch = "aarch64")))]
    const CONF_NAME: libc::c_int = libc::_SC_NPROCESSORS_ONLN;

    let cpus = unsafe { libc::sysconf(CONF_NAME) };
    if cpus < 1 {
        1
    } else {
        cpus as usize
    }
}

#[cfg(target_os = "haiku")]
fn get_num_cpus() -> usize {
    use std::mem;

    #[allow(non_camel_case_types)]
    type bigtime_t = i64;
    #[allow(non_camel_case_types)]
    type status_t = i32;

    #[repr(C)]
    pub struct system_info {
        pub boot_time: bigtime_t,
        pub cpu_count: u32,
        pub max_pages: u64,
        pub used_pages: u64,
        pub cached_pages: u64,
        pub block_cache_pages: u64,
        pub ignored_pages: u64,
        pub needed_memory: u64,
        pub free_memory: u64,
        pub max_swap_pages: u64,
        pub free_swap_pages: u64,
        pub page_faults: u32,
        pub max_sems: u32,
        pub used_sems: u32,
        pub max_ports: u32,
        pub used_ports: u32,
        pub max_threads: u32,
        pub used_threads: u32,
        pub max_teams: u32,
        pub used_teams: u32,
        pub kernel_name: [::std::os::raw::c_char; 256usize],
        pub kernel_build_date: [::std::os::raw::c_char; 32usize],
        pub kernel_build_time: [::std::os::raw::c_char; 32usize],
        pub kernel_version: i64,
        pub abi: u32,
    }

    extern {
        fn get_system_info(info: *mut system_info) -> status_t;
    }

    let mut info: system_info = unsafe { mem::zeroed() };
    let status = unsafe { get_system_info(&mut info as *mut _) };
    if status == 0 {
        info.cpu_count as usize
    } else {
        1
    }
}

#[cfg(target_os = "hermit")]
fn get_num_cpus() -> usize {
    unsafe { hermit_abi::get_processor_count() }
}

#[cfg(not(any(
    target_os = "nacl",
    target_os = "macos",
    target_os = "ios",
    target_os = "android",
    target_os = "solaris",
    target_os = "illumos",
    target_os = "fuchsia",
    target_os = "linux",
    target_os = "openbsd",
    target_os = "freebsd",
    target_os = "dragonfly",
    target_os = "netbsd",
    target_os = "haiku",
    target_os = "hermit",
    windows,
)))]
fn get_num_cpus() -> usize {
    1
}

#[cfg(all(windows, feature = "extended"))]
fn get_num_cpus() -> usize {
    match get_num_logical_cpus_ex_windows() {
        Some(num) => num,
        None => 0 
    }
}

/// Returns the correct number of logical cores of the current Windows system
/// even when the system has more than 64 logical cores using the API
/// [GetLogicalProcessorInformationEx](https://docs.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-getlogicalprocessorinformationex)
///
/// # Note
///
/// Logical processor count is not complete if fetched on a Windows system
/// using GetLogicalProcessorInformation when the system has more than 64
/// logical cores. This information can be correctly gathered using the
/// GetLogicalProcessorInformationEx API which has detailed information of
/// every processor group in the system.
///
/// # Usage
/// 
/// Unfortunately, there is no way of telling if the count returned by
/// GetLogicalProcessorInformation was complete or not. Since the number
/// of installations with more than 64 cores is not common, this feature
/// is implemented as configurable feature called "extended". Compile this
/// package with the feature to use this implementation.
///
/// # Understanding GetLogicalProcessorInformationEx API
///
/// This API returns set of variable sized structs packed together and
/// needs manual processing by looking at the sizes of the parts. Also
/// the inner data struct which holds the logical processor mappings
/// itself is variable size array. This is best understood by the C++
/// code on Windows to use the API for fetching the logical processor
/// count. Some FFI tricks are perfromed to read the struct into rust
///
/// ```cpp
///    #include <sysinfoapi.h>
///    #include <cassert>
///
///    DWORD GetNumLogicalProcessorsEx()
///    {
///        const LOGICAL_PROCESSOR_RELATIONSHIP relationship = LOGICAL_PROCESSOR_RELATIONSHIP::RelationProcessorCore;
///        DWORD len = 0;
///        {
///            bool result = GetLogicalProcessorInformationEx(relationship, nullptr, &len);
///            assert(len > 0);
///        }
///
///        std::vector<BYTE> buffer(len, 0);
///        {
///            PSYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX pBuffer = reinterpret_cast<PSYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX>(&buffer[0]);
///            bool result = GetLogicalProcessorInformationEx(relationship, pBuffer, &len);
///            assert(result == true);
///        }
///
///        DWORD nprocs = 0;
///
///        DWORD byteOffset = 0;
///        while (byteOffset < len) {
///            PSYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX ptr = reinterpret_cast<PSYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX>(&buffer[0] + byteOffset);
///
///            switch (ptr->Relationship) {
///            case RelationProcessorCore: {
///                for (WORD group = 0; group < ptr->Processor.GroupCount; ++group) {
///                    nprocs += CountSetBits(ptr->Processor.GroupMask[group].Mask);
///                }
///            }
///            break;
///
///            default:
///                break;
///            }
///
///            byteOffset += ptr->Size;
///        }
///
///        return nprocs;
///    }
/// ```
///
/// [`get_num_logical_cpus_ex_windows()`]: fn.get.html
#[cfg(all(windows, feature = "extended"))]
fn get_num_logical_cpus_ex_windows() -> Option<usize> {
    use std::mem;
    use std::ptr;
    use std::slice;

    #[allow(non_upper_case_globals)]
    const RelationProcessorCore: u32 = 0;

    #[repr(C)]
    #[allow(non_camel_case_types)]
    #[allow(dead_code)]
    struct GROUP_AFFINITY {
        mask: usize,
        group: u16,
        reserved: [u16; 3],
    }

    #[repr(C)]
    #[allow(non_camel_case_types)]
    #[allow(dead_code)]
    struct PROCESSOR_RELATIONSHIP {
        flags: u8,
        efficiencyClass: u8,
        reserved: [u8; 20],
        groupCount: u16,
        groupMaskTenative: [GROUP_AFFINITY; 1],
    }

    #[repr(C)]
    #[allow(non_camel_case_types)]
    #[allow(dead_code)]
    struct SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX {
        relationship: u32,
        size: u32,
        processor: PROCESSOR_RELATIONSHIP,
    }

    extern "system" {
        fn GetLogicalProcessorInformationEx(
            relationship: u32,
            data: *mut u8,
            length: &mut u32,
        ) -> bool;
    }

    // First we need to determine how much space to reserve.

    // The required size of the buffer, in bytes.
    let mut needed_size = 0;

    unsafe {
        GetLogicalProcessorInformationEx(RelationProcessorCore, ptr::null_mut(), &mut needed_size);
    }

    // Could be 0, or some other bogus size.
    if needed_size == 0 {
        return None;
    }

    // Allocate memory where we will store the processor info.
    let mut buffer: Vec<u8> = vec![0 as u8; needed_size as usize];

    unsafe {
        let result: bool = GetLogicalProcessorInformationEx(
            RelationProcessorCore,
            buffer.as_mut_ptr(),
            &mut needed_size,
        );

        if result == false {
            return None;
        }
    }

    let mut n_logical_procs: usize = 0;

    let mut byte_offset: usize = 0;
    while byte_offset < needed_size as usize {
        unsafe {
            // interpret this byte-array as SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX struct
            let part_ptr_raw: *const u8 = buffer.as_ptr().offset(byte_offset as isize);
            let part_ptr: *const SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX =
                mem::transmute::<*const u8, *const SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX>(
                    part_ptr_raw,
                );
            let part: &SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX = &*part_ptr;

            // we are only interested in RelationProcessorCore information and hence
            // we have requested only for this kind of data (so we should not see other types of data)
            if part.relationship == RelationProcessorCore {
                // the number of GROUP_AFFINITY structs in the array will be specified in the 'groupCount'
                // we tenatively use the first element to get the pointer to it and reinterpret the
                // entire slice with the groupCount
                let groupmasks_slice: &[GROUP_AFFINITY] =
                    slice::from_raw_parts(
                        part.processor.groupMaskTenative.as_ptr(),
                        part.processor.groupCount as usize);

                // count the local logical processors of the group and accumulate
                let n_local_procs: usize = groupmasks_slice
                    .iter()
                    .map(|g| g.mask.count_ones() as usize)
                    .sum::<usize>();
                n_logical_procs += n_local_procs;
            }

            // set the pointer to the next part as indicated by the size of this part
            byte_offset += part.size as usize;
        }
    }

    Some(n_logical_procs)
}

#[cfg(test)]
mod tests {
    fn env_var(name: &'static str) -> Option<usize> {
        ::std::env::var(name).ok().map(|val| val.parse().unwrap())
    }

    #[test]
    fn test_get() {
        let num = super::get();
        if let Some(n) = env_var("NUM_CPUS_TEST_GET") {
            assert_eq!(num, n);
        } else {
            assert!(num > 0);
            assert!(num < 236_451);
        }
    }

    #[test]
    fn test_get_physical() {
        let num = super::get_physical();
        if let Some(n) = env_var("NUM_CPUS_TEST_GET_PHYSICAL") {
            assert_eq!(num, n);
        } else {
            assert!(num > 0);
            assert!(num < 236_451);
        }
    }

    #[cfg(all(windows, feature = "extended"))]
    #[test]
    fn test_get_num_logical_cpus_ex_windows() {
        let m = super::get();
        let result = super::get_num_logical_cpus_ex_windows();
        match result {
            None => assert_eq!(false, true),
            Some(n) => assert_eq!(true, n >= m)
        }
    }

}
