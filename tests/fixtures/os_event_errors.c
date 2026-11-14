/** @file Real Linux handle lifecycle with syscall error and interruption controls. */
#include "Os_Host.h"
#include "Os.h"
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <stdio.h>
#include <string.h>
#include <sys/eventfd.h>
#include <sys/timerfd.h>
#include <time.h>
#include <unistd.h>

static const char *scenario;
static unsigned calls;
static int bad_errno;
static int replacement = -1;
static unsigned clock_calls;
static unsigned deadline_ms = 100u;
static int deadline_fd = -1;
static unsigned deadline_closed;

void StartupHook(void) {}
void ShutdownHook(StatusType reason) { (void)reason; }

int __real_timerfd_create(int clock, int flags);
int __real_timerfd_settime(int fd,
                           int flags,
                           const struct itimerspec *value,
                           struct itimerspec *previous);
int __real_poll(struct pollfd *fds, nfds_t count, int timeout);
ssize_t __real_read(int fd, void *data, size_t length);
ssize_t __real_write(int fd, const void *data, size_t length);
int __real_close(int fd);
int __real_pthread_mutex_destroy(pthread_mutex_t *mutex);
int __real_clock_gettime(clockid_t clock, struct timespec *value);
int __real___poll_chk(struct pollfd *fds, nfds_t count, int timeout, size_t bytes);
ssize_t __real___read_chk(int fd, void *data, size_t length, size_t bytes);

int __wrap_timerfd_create(int clock, int flags)
{
    if (scenario != NULL && strncmp(scenario, "deadline-", 9u) == 0) {
        ++calls;
        bad_errno |=
            errno != 0 || clock != CLOCK_MONOTONIC || flags != (TFD_CLOEXEC | TFD_NONBLOCK);
        if (strcmp(scenario, "deadline-create-error") == 0) {
            errno = EMFILE;
            return -1;
        }
        deadline_fd = __real_timerfd_create(clock, flags);
        return deadline_fd;
    }
    return __real_timerfd_create(clock, flags);
}

int __wrap_timerfd_settime(int fd,
                           int flags,
                           const struct itimerspec *value,
                           struct itimerspec *previous)
{
    if (scenario != NULL && strcmp(scenario, "poll-deadline") == 0) {
        bad_errno |= flags != TFD_TIMER_ABSTIME || value->it_value.tv_sec != 0 ||
                     value->it_value.tv_nsec != 110000000L || value->it_interval.tv_sec != 0 ||
                     value->it_interval.tv_nsec != 0 || previous != NULL;
    }
    if (scenario != NULL && strcmp(scenario, "deadline-arm-error") == 0) {
        ++calls;
        bad_errno |= errno != 0;
        errno = EINVAL;
        return -1;
    }
    return __real_timerfd_settime(fd, flags, value, previous);
}

int __wrap_clock_gettime(clockid_t clock, struct timespec *value)
{
    if ((scenario != NULL) && (strcmp(scenario, "poll-deadline") == 0)) {
        value->tv_sec = 0;
        value->tv_nsec = (long)deadline_ms * 1000000L;
        return 0;
    }
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
            /* One shared event wait checks its clock before and after poll. */
            failed_call = 3u;
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
    if (scenario != NULL && strncmp(scenario, "deadline-", 9u) == 0 && fd == deadline_fd) {
        ++deadline_closed;
        bad_errno |= errno != 0;
        const int result = __real_close(fd);
        if (result == 0 && strcmp(scenario, "deadline-close-error") == 0) {
            errno = EIO;
            return -1;
        }
        if (result == 0 && strcmp(scenario, "deadline-close-interrupted") == 0) {
            replacement = eventfd(0u, EFD_CLOEXEC | EFD_NONBLOCK);
            bad_errno |= replacement != fd;
            errno = EINTR;
            return -1;
        }
        return result;
    }
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
    if (scenario != NULL && strncmp(scenario, "deadline-poll-", 14u) == 0) {
        ++calls;
        bad_errno |= count != 2u || timeout != -1 || fds[1].fd != deadline_fd;
        fds[1].revents = strcmp(scenario, "deadline-poll-invalid") == 0  ? POLLNVAL
                         : strcmp(scenario, "deadline-poll-hangup") == 0 ? POLLHUP
                                                                         : POLLERR | POLLIN;
        return 1;
    }
    if ((scenario != NULL) && (strncmp(scenario, "poll-", 5u) == 0)) {
        ++calls;
        bad_errno |= errno != 0;
        if (strcmp(scenario, "poll-deadline") == 0) {
            bad_errno |= timeout != -1;
            if (calls == 1u) {
                deadline_ms += 4u;
                errno = EINTR;
                return -1;
            }
            deadline_ms += 6u;
            return 0;
        }
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

int __wrap___poll_chk(struct pollfd *fds, nfds_t count, int timeout, size_t bytes)
{
    if (count > bytes / sizeof(*fds)) {
        return __real___poll_chk(fds, count, timeout, bytes);
    }
    return __wrap_poll(fds, count, timeout);
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

ssize_t __wrap___read_chk(int fd, void *data, size_t length, size_t bytes)
{
    if (length > bytes) {
        return __real___read_chk(fd, data, length, bytes);
    }
    return __wrap_read(fd, data, length);
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
    const int multiple = strncmp(scenario, "any-", 4u) == 0;
    if (multiple != 0) {
        scenario += 4;
    }
    if (strcmp(scenario, "fortify-read") == 0) {
        char byte = 0;
        (void)__wrap___read_chk(-1, &byte, 2u, sizeof(byte));
        return 15;
    }
    if (strcmp(scenario, "fortify-poll") == 0) {
        struct pollfd one = {.fd = -1, .events = 0, .revents = 0};
        (void)__wrap___poll_chk(&one, 2u, 0, sizeof(one));
        return 16;
    }
    errno = EAGAIN;
    if (strcmp(scenario, "event-order") == 0) {
        HANDLE observed[2] = {manual, event};
        if ((Os_HostSetEvent(manual) == FALSE) ||
            (WaitForMultipleObjects(2u, observed, FALSE, 0u) != WAIT_OBJECT_0) ||
            (WaitForMultipleObjects(2u, observed, FALSE, 0u) != WAIT_OBJECT_0) ||
            (ResetEvent(manual) == FALSE) ||
            (WaitForMultipleObjects(2u, observed, FALSE, 0u) != 1u) ||
            (WaitForMultipleObjects(2u, observed, FALSE, 0u) != WAIT_TIMEOUT)) {
            return 17;
        }
        calls = 1u;
    } else if (strncmp(scenario, "clock-", 6u) == 0) {
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
        replacement = -1;
        event = NULL;
    } else if (strncmp(scenario, "deadline-", 9u) == 0) {
        if (ResetEvent(event) == FALSE) {
            return 23;
        }
        const DWORD result = WaitForSingleObject(event, 1u);
        const DWORD expected =
            strcmp(scenario, "deadline-close-interrupted") == 0 ? WAIT_TIMEOUT : WAIT_FAILED;
        if (result != expected) {
            return 18;
        }
        if (deadline_fd >= 0 && deadline_closed != 1u) {
            return 19;
        }
        if (replacement >= 0) {
            if (fcntl(replacement, F_GETFD) < 0) {
                return 20;
            }
        } else if (deadline_fd >= 0 && (fcntl(deadline_fd, F_GETFD) != -1 || errno != EBADF)) {
            return 21;
        }
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
        const DWORD milliseconds = (strcmp(scenario, "poll-error-infinite") == 0) ? INFINITE
                                   : (strcmp(scenario, "poll-deadline") == 0)     ? 10u
                                                                                  : 0u;
        HANDLE observed[2] = {event, manual};
        const DWORD result = (multiple != 0)
                                 ? WaitForMultipleObjects(2u, observed, FALSE, milliseconds)
                                 : WaitForSingleObject(event, milliseconds);
        const DWORD expected =
            ((strcmp(scenario, "wait-read-empty") == 0) || (strcmp(scenario, "poll-deadline") == 0))
                ? WAIT_TIMEOUT
            : ((strcmp(scenario, "poll-interrupted") == 0) ||
               (strcmp(scenario, "wait-read-interrupted") == 0))
                ? WAIT_OBJECT_0
                : WAIT_FAILED;
        if (result != expected) {
            return 5;
        }
    }
    scenario = NULL;
    if (replacement >= 0 && __real_close(replacement) != 0) {
        return 22;
    }
    if ((bad_errno != 0) || (calls == 0u)) {
        return 6;
    }
    if (((event != NULL) && (CloseHandle(event) == FALSE)) || (CloseHandle(manual) == FALSE)) {
        return 7;
    }
    puts("event success/reset/failure/interruption observed");
    return 0;
}
