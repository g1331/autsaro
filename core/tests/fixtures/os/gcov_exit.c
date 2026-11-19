/** @file Publish GCC counters on successful Linux test shutdown. */
#include <gcov.h>

extern void __real__exit(int status) __attribute__((noreturn));
void __wrap__exit(int status) __attribute__((noreturn));

void __wrap__exit(int status)
{
    /* Successful OS shutdown uses _exit after stopping its workers. Leave
     * asynchronous fatal exits alone because dumping counters can block. */
    if (status == 0) {
        __gcov_dump();
    }
    __real__exit(status);
}
