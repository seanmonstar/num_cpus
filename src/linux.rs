use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::mem;
use std::mem::MaybeUninit;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Once;

macro_rules! debug {
    ($($args:expr),*) => ({
        if false {
        //if true {
            println!($($args),*);
        }
    });
}

macro_rules! some {
    ($e:expr) => {{
        match $e {
            Some(v) => v,
            None => {
                debug!("NONE: {:?}", stringify!($e));
                return None;
            }
        }
    }};
}

pub fn get_num_cpus() -> usize {
    match cgroups_num_cpus() {
        Some(n) => n,
        None => logical_cpus(),
    }
}

fn logical_cpus() -> usize {
    let mut set = MaybeUninit::<libc::cpu_set_t>::uninit();
    if unsafe { libc::sched_getaffinity(0, mem::size_of::<libc::cpu_set_t>(), set.as_mut_ptr()) } == 0 {
        let mut count: u32 = 0;
        for i in 0..libc::CPU_SETSIZE as usize {
            if unsafe { libc::CPU_ISSET(i, &set.as_ptr().read()) } {
                count += 1;
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

pub fn get_num_physical_cpus() -> usize {
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

/// Cached CPUs calculated from cgroups.
///
/// If 0, check logical cpus.
// Allow deprecation warnings, we want to work on older rustc
#[allow(warnings)]
static CGROUPS_CPUS: AtomicUsize = ::std::sync::atomic::ATOMIC_USIZE_INIT;

fn cgroups_num_cpus() -> Option<usize> {
    #[allow(warnings)]
    static ONCE: Once = ::std::sync::ONCE_INIT;

    ONCE.call_once(init_cgroups);

    let cpus = CGROUPS_CPUS.load(Ordering::Acquire);

    if cpus > 0 {
        Some(cpus)
    } else {
        None
    }
}

fn init_cgroups() {
    // Should only be called once
    debug_assert!(CGROUPS_CPUS.load(Ordering::SeqCst) == 0);

    match load_cgroups("/proc/self/cgroup", "/proc/self/mountinfo") {
        Some(quota) => {
            if quota == 0 {
                return;
            }

            let logical = logical_cpus();
            let count = ::std::cmp::min(quota, logical);

            CGROUPS_CPUS.store(count, Ordering::SeqCst);
        }
        None => {}
    }
}

fn load_cgroups<P1, P2>(cgroup_proc: P1, mountinfo_proc: P2) -> Option<usize>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let subsys = some!(Subsys::load_cpu(cgroup_proc));
    let mntinfo = some!(MountInfo::load_cpu(mountinfo_proc));
    let cgroup = some!(Cgroup::translate(mntinfo, subsys));
    cgroup.cpu_quota()
}

struct Cgroup {
    base: PathBuf,
}

struct MountInfo {
    root: String,
    mount_point: String,
}

struct Subsys {
    base: String,
}

impl Cgroup {
    const fn new(dir: PathBuf) -> Self {
        Self { base: dir }
    }

    fn translate(mntinfo: MountInfo, subsys: Subsys) -> Option<Self> {
        // Translate the subsystem directory via the host paths.
        debug!(
            "subsys = {:?}; root = {:?}; mount_point = {:?}",
            subsys.base, mntinfo.root, mntinfo.mount_point
        );

        let rel_from_root = some!(Path::new(&subsys.base).strip_prefix(&mntinfo.root).ok());

        debug!("rel_from_root: {:?}", rel_from_root);

        // join(mp.MountPoint, relPath)
        let mut path = PathBuf::from(mntinfo.mount_point);
        path.push(rel_from_root);
        Some(Self::new(path))
    }

    fn cpu_quota(&self) -> Option<usize> {
        let quota_us = some!(self.quota_us());
        let period_us = some!(self.period_us());

        // protect against dividing by zero
        if period_us == 0 {
            return None;
        }

        // Ceil the division, since we want to be able to saturate
        // the available CPUs, and flooring would leave a CPU un-utilized.

        Some((quota_us as f64 / period_us as f64).ceil() as usize)
    }

    fn quota_us(&self) -> Option<usize> {
        self.param("cpu.cfs_quota_us")
    }

    fn period_us(&self) -> Option<usize> {
        self.param("cpu.cfs_period_us")
    }

    fn param(&self, param: &str) -> Option<usize> {
        let mut file = some!(File::open(self.base.join(param)).ok());

        let mut buf = String::new();
        some!(file.read_to_string(&mut buf).ok());

        buf.trim().parse().ok()
    }
}

impl MountInfo {
    fn load_cpu<P: AsRef<Path>>(proc_path: P) -> Option<Self> {
        let file = some!(File::open(proc_path).ok());
        let file = BufReader::new(file);

        file.lines()
            .filter_map(Result::ok)
            .find_map(Self::parse_line)
    }

    fn parse_line(line: String) -> Option<Self> {
        let mut fields = line.split(' ');

        let mnt_root = some!(fields.nth(3));
        let mnt_point = some!(fields.next());

        if fields.nth(3) != Some("cgroup") {
            return None;
        }

        let super_opts = some!(fields.nth(1));

        // We only care about the 'cpu' option
        if !super_opts.split(',').any(|opt| opt == "cpu") {
            return None;
        }

        Some(Self {
            root: mnt_root.to_owned(),
            mount_point: mnt_point.to_owned(),
        })
    }
}

impl Subsys {
    fn load_cpu<P: AsRef<Path>>(proc_path: P) -> Option<Self> {
        let file = some!(File::open(proc_path).ok());
        let file = BufReader::new(file);

        file.lines()
            .filter_map(std::result::Result::ok)
            .find_map(Self::parse_line)
    }

    fn parse_line(line: String) -> Option<Self> {
        // Example format:
        // 11:cpu,cpuacct:/
        let mut fields = line.split(':');

        let sub_systems = some!(fields.nth(1));

        if !sub_systems.split(',').any(|sub| sub == "cpu") {
            return None;
        }

        fields.next().map(|path| Self {
            base: path.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Cgroup, MountInfo, Subsys};
    use std::path::{Path, PathBuf};

    static FIXTURES_PROC: &str = "fixtures/cgroups/proc/cgroups";

    static FIXTURES_CGROUPS: &str = "fixtures/cgroups/cgroups";

    macro_rules! join {
        ($base:expr, $($path:expr),+) => ({
            Path::new($base)
                $(.join($path))+
        })
    }

    #[test]
    fn test_load_mountinfo() {
        let path = join!(FIXTURES_PROC, "mountinfo");

        let mnt_info = MountInfo::load_cpu(path).unwrap();

        assert_eq!(mnt_info.root, "/");
        assert_eq!(mnt_info.mount_point, "/sys/fs/cgroup/cpu,cpuacct");
    }

    #[test]
    fn test_load_subsys() {
        let path = join!(FIXTURES_PROC, "cgroup");

        let subsys = Subsys::load_cpu(path).unwrap();

        assert_eq!(subsys.base, "/");
    }

    #[test]
    fn test_cgroup_mount() {
        let cases = &[
            ("/", "/sys/fs/cgroup/cpu", "/", Some("/sys/fs/cgroup/cpu")),
            (
                "/docker/01abcd",
                "/sys/fs/cgroup/cpu",
                "/docker/01abcd",
                Some("/sys/fs/cgroup/cpu"),
            ),
            (
                "/docker/01abcd",
                "/sys/fs/cgroup/cpu",
                "/docker/01abcd/",
                Some("/sys/fs/cgroup/cpu"),
            ),
            (
                "/docker/01abcd",
                "/sys/fs/cgroup/cpu",
                "/docker/01abcd/large",
                Some("/sys/fs/cgroup/cpu/large"),
            ),
            // fails
            ("/docker/01abcd", "/sys/fs/cgroup/cpu", "/", None),
            ("/docker/01abcd", "/sys/fs/cgroup/cpu", "/docker", None),
            ("/docker/01abcd", "/sys/fs/cgroup/cpu", "/elsewhere", None),
            (
                "/docker/01abcd",
                "/sys/fs/cgroup/cpu",
                "/docker/01abcd-other-dir",
                None,
            ),
        ];

        for &(root, mount_point, subsys, expected) in cases.iter() {
            let mnt_info = MountInfo {
                root: root.into(),
                mount_point: mount_point.into(),
            };
            let subsys = Subsys {
                base: subsys.into(),
            };

            let actual = Cgroup::translate(mnt_info, subsys).map(|c| c.base);
            let expected = expected.map(PathBuf::from);
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_cgroup_cpu_quota() {
        let cgroup = Cgroup::new(join!(FIXTURES_CGROUPS, "good"));
        assert_eq!(cgroup.cpu_quota(), Some(6));
    }

    #[test]
    fn test_cgroup_cpu_quota_divide_by_zero() {
        let cgroup = Cgroup::new(join!(FIXTURES_CGROUPS, "zero-period"));
        assert!(cgroup.quota_us().is_some());
        assert_eq!(cgroup.period_us(), Some(0));
        assert_eq!(cgroup.cpu_quota(), None);
    }

    #[test]
    fn test_cgroup_cpu_quota_ceil() {
        let cgroup = Cgroup::new(join!(FIXTURES_CGROUPS, "ceil"));
        assert_eq!(cgroup.cpu_quota(), Some(2));
    }
}
