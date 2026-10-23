/** @file Real Linux eventfd consumer with syscall error and interruption controls. */
#include "Os_Host.h"
#include "Os.h"
#include <errno.h>
#include <poll.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

static const char *scenario;
static unsigned calls;
static int bad_errno;

void StartupHook(void) {}
void ShutdownHook(StatusType reason) { (void)reason; }

int __real_poll(struct pollfd *fds, nfds_t count, int timeout);
ssize_t __real_read(int fd, void *data, size_t length);
ssize_t __real_write(int fd, const void *data, size_t length);

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
    if (strncmp(scenario, "write-", 6u) == 0) {
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
    if ((CloseHandle(event) == FALSE) || (CloseHandle(manual) == FALSE)) {
        return 7;
    }
    puts("event success/reset/failure/interruption observed");
    return 0;
}
