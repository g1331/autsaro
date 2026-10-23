/** @file Real Linux handle lifecycle with syscall error and interruption controls. */
#include "Os_Host.h"
#include "Os.h"
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <stdio.h>
#include <string.h>
#include <sys/eventfd.h>
#include <time.h>
#include <unistd.h>

static const char *scenario;
static unsigned calls;
static int bad_errno;
static int replacement = -1;
static unsigned clock_calls;

void StartupHook(void) {}
void ShutdownHook(StatusType reason) { (void)reason; }

int __real_poll(struct pollfd *fds, nfds_t count, int timeout);
ssize_t __real_read(int fd, void *data, size_t length);
ssize_t __real_write(int fd, const void *data, size_t length);
int __real_close(int fd);
int __real_pthread_mutex_destroy(pthread_mutex_t *mutex);
int __real_clock_gettime(clockid_t clock, struct timespec *value);

int __wrap_clock_gettime(clockid_t clock, struct timespec *value)
{
    if ((scenario != NULL) && (strncmp(scenario, "clock-", 6u) == 0)) {
        ++clock_calls;
        unsigned failed_call = 0u;
        if ((strcmp(scenario, "clock-any-initial") == 0) ||
            (strcmp(scenario, "clock-all-initial") == 0)) {
            failed_call = 1u;
        } else if ((strcmp(scenario, "clock-single-later") == 0) ||
                   (strcmp(scenario, "clock-all-later") == 0)) {
            failed_call = 2u;
        } else if (strcmp(scenario, "clock-any-later") == 0) {
            failed_call = 4u;
        } else {
            /* The clean controls use the real clock on every call. */
        }
        if (clock_calls == failed_call) {
            ++calls;
            errno = EIO;
            return -1;
        }
    }
    return __real_clock_gettime(clock, value);
}

int __wrap_pthread_mutex_destroy(pthread_mutex_t *mutex)
{
    if ((scenario != NULL) && (strcmp(scenario, "mutex-close-error") == 0)) {
        ++calls;
        if (calls == 1u) {
            return EBUSY;
        }
    }
    return __real_pthread_mutex_destroy(mutex);
}

int __wrap_close(int fd)
{
    if ((scenario != NULL) && (strcmp(scenario, "event-close-error-reuse") == 0)) {
        ++calls;
        bad_errno |= errno != 0;
        const int result = __real_close(fd);
        if (result == 0) {
            errno = EINTR;
            return -1;
        }
        return result;
    }
    if ((scenario != NULL) && (strcmp(scenario, "close-interrupted") == 0)) {
        ++calls;
        if (calls == 1u) {
            if (__real_close(fd) != 0) {
                return -1;
            }
            replacement = eventfd(0u, EFD_CLOEXEC | EFD_NONBLOCK);
            if (replacement != fd) {
                bad_errno = 1;
            }
            errno = EINTR;
            return -1;
        }
    }
    return __real_close(fd);
}

int __wrap_poll(struct pollfd *fds, nfds_t count, int timeout)
{
    if ((scenario != NULL) && (strncmp(scenario, "poll-", 5u) == 0)) {
        ++calls;
        bad_errno |= errno != 0;
        if (strncmp(scenario, "poll-error", 10u) == 0) {
            errno = EBADF;
            return -1;
        }
        if (strcmp(scenario, "poll-invalid") == 0) {
            fds[0].revents = POLLNVAL;
            return 1;
        }
        if (strcmp(scenario, "poll-hangup") == 0) {
            fds[0].revents = POLLHUP;
            return 1;
        }
        if (strcmp(scenario, "poll-revents-error") == 0) {
            fds[0].revents = POLLERR | POLLIN;
            return 1;
        }
        if (calls == 1u) {
            errno = EINTR;
            return -1;
        }
    }
    return __real_poll(fds, count, timeout);
}

ssize_t __wrap_read(int fd, void *data, size_t length)
{
    const char *operation = scenario;
    if ((operation != NULL) && (strncmp(operation, "wait-", 5u) == 0)) {
        operation += 5;
    }
    if ((operation != NULL) && (strncmp(operation, "read-", 5u) == 0)) {
        ++calls;
        bad_errno |= errno != 0;
        if (strcmp(operation, "read-error") == 0) {
            errno = EIO;
            return -1;
        }
        if (strcmp(operation, "read-zero") == 0) {
            return 0;
        }
        if (strcmp(operation, "read-short") == 0) {
            return 1;
        }
        if (strcmp(operation, "read-empty") == 0) {
            errno = EAGAIN;
            return -1;
        }
        if (calls == 1u) {
            errno = EINTR;
            return -1;
        }
    }
    return __real_read(fd, data, length);
}

ssize_t __wrap_write(int fd, const void *data, size_t length)
{
    if ((scenario != NULL) && (strncmp(scenario, "write-", 6u) == 0)) {
        ++calls;
        bad_errno |= errno != 0;
        if (strcmp(scenario, "write-error") == 0) {
            errno = EIO;
            return -1;
        }
        if (calls == 1u) {
            errno = EINTR;
            return -1;
        }
    }
    return __real_write(fd, data, length);
}

int main(int argc, char **argv)
{
    HANDLE event = CreateEventA(NULL, FALSE, TRUE, NULL);
    HANDLE manual = CreateEventA(NULL, TRUE, TRUE, NULL);
    if ((argc != 2) || (event == NULL) || (manual == NULL)) {
        return 1;
    }
    /* Real descriptors establish auto-reset consumption and manual persistence. */
    if ((WaitForSingleObject(event, 0u) != WAIT_OBJECT_0) ||
        (WaitForSingleObject(event, 0u) != WAIT_TIMEOUT) ||
        (WaitForSingleObject(manual, 0u) != WAIT_OBJECT_0) ||
        (WaitForSingleObject(manual, 0u) != WAIT_OBJECT_0) || (ResetEvent(manual) == FALSE) ||
        (WaitForSingleObject(manual, 0u) != WAIT_TIMEOUT) || (Os_HostSetEvent(event) == FALSE)) {
        return 2;
    }
    scenario = argv[1];
    errno = EAGAIN;
    if (strncmp(scenario, "clock-", 6u) == 0) {
        const BOOL all = strstr(scenario, "-all-") != NULL ? 1 : 0;
        HANDLE observed =
            ((strstr(scenario, "-initial") != NULL) || (strcmp(scenario, "clock-all-later") == 0))
                ? event
                : manual;
        const DWORD result = strstr(scenario, "-single-") != NULL
                                 ? WaitForSingleObject(observed, 0u)
                                 : WaitForMultipleObjects(1u, &observed, all, 0u);
        const DWORD expected = strstr(scenario, "-normal") != NULL ? WAIT_TIMEOUT : WAIT_FAILED;
        if (result != expected) {
            printf("clock wait result=%lu expected=%lu calls=%u\n",
                   (unsigned long)result,
                   (unsigned long)expected,
                   clock_calls);
            return 14;
        }
        if (expected == WAIT_TIMEOUT) {
            calls = clock_calls;
        }
    } else if (strcmp(scenario, "mutex-reuse") == 0) {
        for (unsigned index = 0u; index < 128u; ++index) {
            HANDLE mutex = CreateMutexA(NULL, FALSE, NULL);
            if ((mutex == NULL) || (WaitForSingleObject(mutex, 0u) != WAIT_OBJECT_0) ||
                (ReleaseMutex(mutex) == FALSE) || (CloseHandle(mutex) == FALSE)) {
                return 8;
            }
        }
        calls = 1u;
    } else if (strcmp(scenario, "mutex-close-error") == 0) {
        HANDLE first = CreateMutexA(NULL, FALSE, NULL);
        if ((first == NULL) || (CloseHandle(first) != FALSE)) {
            return 11;
        }
        HANDLE second = CreateMutexA(NULL, FALSE, NULL);
        if ((second == NULL) || (first == second) || (CloseHandle(second) == FALSE) ||
            (CloseHandle(first) == FALSE)) {
            return 12;
        }
    } else if (strcmp(scenario, "event-close-error-reuse") == 0) {
        for (unsigned index = 0u; index < 128u; ++index) {
            HANDLE temporary = CreateEventA(NULL, FALSE, FALSE, NULL);
            if ((temporary == NULL) || (CloseHandle(temporary) != FALSE)) {
                return 13;
            }
        }
    } else if (strcmp(scenario, "close-interrupted") == 0) {
        if ((CloseHandle(event) != FALSE) || (CloseHandle(event) != FALSE) || (calls != 1u) ||
            (replacement < 0) || (fcntl(replacement, F_GETFD) < 0)) {
            return 9;
        }
        if (__real_close(replacement) != 0) {
            return 10;
        }
        event = NULL;
    } else if (strncmp(scenario, "write-", 6u) == 0) {
        const BOOL result = Os_HostSetEvent(event);
        if (result != ((strcmp(scenario, "write-error") == 0) ? FALSE : TRUE)) {
            return 3;
        }
    } else if (strncmp(scenario, "read-", 5u) == 0) {
        const BOOL result = ResetEvent(event);
        if (result != ((strcmp(scenario, "read-interrupted") == 0) ? TRUE : FALSE)) {
            return 4;
        }
    } else {
        const DWORD result = WaitForSingleObject(
            event, (strcmp(scenario, "poll-error-infinite") == 0) ? INFINITE : 0u);
        const DWORD expected = (strcmp(scenario, "wait-read-empty") == 0) ? WAIT_TIMEOUT
                               : ((strcmp(scenario, "poll-interrupted") == 0) ||
                                  (strcmp(scenario, "wait-read-interrupted") == 0))
                                   ? WAIT_OBJECT_0
                                   : WAIT_FAILED;
        if (result != expected) {
            return 5;
        }
    }
    scenario = NULL;
    if ((bad_errno != 0) || (calls == 0u)) {
        return 6;
    }
    if (((event != NULL) && (CloseHandle(event) == FALSE)) || (CloseHandle(manual) == FALSE)) {
        return 7;
    }
    puts("event success/reset/failure/interruption observed");
    return 0;
}
