/** @file Observable delegation for the test-only pre-shutdown observer. */
#include "linux_watchdog_capture.c"
#include <stdarg.h>
#include <string.h>

enum { DELEGATED = 77, INVALID_ARGUMENT = 78 };

int __real_prctl(int option, ...);
int __wrap_prctl(int option, ...);

int __wrap_prctl(int option, ...)
{
    va_list arguments;
    unsigned long parent;
    unsigned long unused1;
    unsigned long unused2;
    unsigned long unused3;
    va_start(arguments, option);
    parent = va_arg(arguments, unsigned long);
    unused1 = va_arg(arguments, unsigned long);
    unused2 = va_arg(arguments, unsigned long);
    unused3 = va_arg(arguments, unsigned long);
    va_end(arguments);
    if ((option != PR_SET_PTRACER) || (parent != (unsigned long)getppid()) || (unused1 != 0UL) ||
        (unused2 != 0UL) || (unused3 != 0UL)) {
        _exit(INVALID_ARGUMENT);
    }
    if (write(STDOUT_FILENO, "P", 1U) != 1) {
        _exit(INVALID_ARGUMENT);
    }
    return __real_prctl(option, parent, unused1, unused2, unused3);
}

static void delegated(void)
{
    if (write(STDOUT_FILENO, "D", 1U) != 1) {
        _exit(INVALID_ARGUMENT);
    }
    _exit(DELEGATED);
}

void __real_Os_BackendShutdown(StatusType error)
{
    (void)error;
    delegated();
}
void __real_Os_BackendRequestShutdown(StatusType error)
{
    (void)error;
    delegated();
}
void __real_Ecu_TargetAbortNative(StatusType error)
{
    (void)error;
    delegated();
}

int main(int argc, char **argv)
{
    if (argc != 2) {
        return INVALID_ARGUMENT;
    }
    if (strcmp(argv[1], "backend") == 0) {
        __wrap_Os_BackendShutdown(E_OS_STATE);
    }
    if (strcmp(argv[1], "request") == 0) {
        __wrap_Os_BackendRequestShutdown(E_OS_STATE);
    }
    if (strcmp(argv[1], "abort") == 0) {
        __wrap_Ecu_TargetAbortNative(E_OS_STATE);
    }
    if (strcmp(argv[1], "abort-zero") == 0) {
        __wrap_Ecu_TargetAbortNative(E_OK);
    }
    if (strcmp(argv[1], "other") == 0) {
        __wrap_Os_BackendShutdown(E_OS_ID);
    }
    return INVALID_ARGUMENT;
}
