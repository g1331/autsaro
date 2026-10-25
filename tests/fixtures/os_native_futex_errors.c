/** @file Checked futex hints: real mismatch/timeout/wake and classified error returns. */
#include "Os_Host.h"
#include "Os.h"
#include "Os_LinuxNative.h"
#include <asm/unistd.h>
#include <errno.h>
#include <linux/futex.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

extern long Os_TestRealNativeCall(
    long number, const volatile void *first, long second, long third, const void *fourth);
extern void Os_TestFutexWait(const volatile int32_t *cell, int32_t expected);
extern void Os_TestFutexPark(void);
long Os_TestNativeCall(
    long number, const volatile void *first, long second, long third, const void *fourth);
static const char *mode;
static unsigned calls;
static const volatile void *expected_cell;
static const volatile void *observed_cell;
static long outcome;

static void refuse(int code, const char *stage)
{
    if (fprintf(stderr, "futex rejected stage=%s calls=%u\n", stage, calls) < 0) {
        _exit(97);
    }
    if (fflush(stderr) != 0) {
        _exit(97);
    }
    _exit(code);
}

static long injected(long result)
{
    outcome = result;
    if (fprintf(stderr, "futex injected mode=%s call=%u result=%ld\n", mode, calls, result) < 0) {
        _exit(97);
    }
    if (fflush(stderr) != 0) {
        _exit(97);
    }
    return result;
}

long Os_TestNativeCall(
    long number, const volatile void *first, long second, long third, const void *fourth)
{
    if (number != __NR_futex) {
        return Os_TestRealNativeCall(number, first, second, third, fourth);
    }
    ++calls;
    const int parking = strcmp(mode, "park-error") == 0;
    if (first == NULL || (parking == 0 && first != expected_cell) ||
        (observed_cell != NULL && observed_cell != first)) {
        refuse(60, "cell");
    }
    observed_cell = first;
    if (strncmp(mode, "wake-", 5u) == 0) {
        if (second != FUTEX_WAKE_PRIVATE || third != INT32_MAX || fourth != NULL || calls != 1u) {
            refuse(60, "wake-arguments");
        }
        if (strcmp(mode, "wake-error") == 0) {
            return injected(-EINVAL);
        }
    } else {
        const struct timespec *interval = fourth;
        const long expected = strcmp(mode, "wait-mismatch") == 0 ? 1L : 0L;
        if (second != FUTEX_WAIT_PRIVATE || third != expected || interval == NULL ||
            interval->tv_sec != 0 || interval->tv_nsec != 10000000L) {
            refuse(60, "wait-arguments");
        }
        if (parking != 0) {
            static const long sequence[] = {0, -EAGAIN, -EINTR, -ETIMEDOUT, -EFAULT};
            if (calls > sizeof(sequence) / sizeof(sequence[0])) {
                refuse(54, "unexpected-retry");
            }
            return injected(sequence[calls - 1u]);
        }
        if (calls != 1u) {
            refuse(60, "wait-count");
        }
        if (strcmp(mode, "wait-error") == 0) {
            return injected(-EFAULT);
        }
        if (strcmp(mode, "wait-interrupted") == 0) {
            return injected(-EINTR);
        }
    }
    outcome = Os_TestRealNativeCall(number, first, second, third, fourth);
    const long expected = strcmp(mode, "wake-empty") == 0      ? 0L
                          : strcmp(mode, "wait-mismatch") == 0 ? -EAGAIN
                                                               : -ETIMEDOUT;
    if (outcome != expected) {
        refuse(61, "kernel-outcome");
    }
    return outcome;
}

void StartupHook(void) {}
void ShutdownHook(StatusType reason) { (void)reason; }

int main(int argc, char **argv)
{
    static volatile int32_t cell;
    if (argc != 2) {
        return 2;
    }
    mode = argv[1];
    expected_cell = &cell;
    if (strcmp(mode, "park-error") == 0) {
        Os_TestFutexPark();
        refuse(53, "park-returned");
    } else if (strncmp(mode, "wake-", 5u) == 0) {
        Os_LinuxFutexWake(&cell);
        if (strcmp(mode, "wake-error") == 0) {
            refuse(51, "wake-error-returned");
        }
    } else {
        Os_TestFutexWait(&cell, strcmp(mode, "wait-mismatch") == 0 ? 1 : 0);
        if (strcmp(mode, "wait-error") == 0) {
            refuse(52, "wait-error-returned");
        }
    }
    if (calls != 1u) {
        return 62;
    }
    if (printf("futex completed mode=%s calls=%u result=%ld\n", mode, calls, outcome) < 0) {
        return 62;
    }
    return 0;
}
