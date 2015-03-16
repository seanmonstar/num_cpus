#if !defined(__WIN32__)
#include <unistd.h>
#else
#include <windows.h>
#endif


static int
num_cpus() {
#if defined(__WIN32__)
    SYSTEM_INFO sysinfo;
    GetSystemInfo(&sysinfo);

    return (int) sysinfo.dwNumberOfProcessors;
#elif defined(__BSD__)
    /* swiped from http://stackoverflow.com/questions/150355/
     *        programmatically-find-the-number-of-cores-on-a-machine */

    unsigned int numCPU;
    int mib[4];
    size_t len = sizeof(numCPU);

    /* set the mib for hw.ncpu */
    mib[0] = CTL_HW;
    mib[1] = HW_AVAILCPU;  // alternatively, try HW_NCPU;

    /* get the number of CPUs from the system */
    sysctl(mib, 2, &numCPU, &len, NULL, 0);

    if( numCPU < 1 ) {
            mib[1] = HW_NCPU;
            sysctl( mib, 2, &numCPU, &len, NULL, 0 );
    
            if( numCPU < 1 ) {
                        numCPU = 1;
                    }
                }
        return numCPU;
#elif defined(__GNUC__)
    return sysconf(_SC_NPROCESSORS_ONLN);
#endif
}

int
crates_io_get_num_cpus() {
    return num_cpus();
}
