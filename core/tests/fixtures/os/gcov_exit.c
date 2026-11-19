/** @file Publish GCC counters at observed Linux test exits. */
#include <gcov.h>

extern void __real__exit(int status) __attribute__((noreturn));
void __wrap__exit(int status) __attribute__((noreturn));

void __wrap__exit(int status)
{
    /* Successful OS shutdown uses _exit after stopping its workers. The DET
     * contract fixture also observes synchronous, single-threaded halts. */
    if ((status == 0)
#ifdef AUTOSAR_GCOV_DET_HALT
        || (status == 134)
#endif
#ifdef AUTOSAR_GCOV_DISABLED_FIRST_START
        /* The lifecycle fixture exits with E_OS_DISABLEDINT before starting workers. */
        || (status == 9)
#endif
    ) {
        __gcov_dump();
    }
    __real__exit(status);
}
