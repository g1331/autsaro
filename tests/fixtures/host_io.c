/** @file Independent byte I/O, bounded formatting and partial-write consumers. */
#include "Os_HostIo.h"
#include "Os_Host.h"
#include "Os.h"
#include <string.h>
#ifndef _WIN32
#include <errno.h>
#include <signal.h>
#include <unistd.h>
#endif

void StartupHook(void) {}
void ShutdownHook(StatusType reason) { (void)reason; }

static const char *scenario;
#ifndef _WIN32
static unsigned calls;
ssize_t __real_write(int descriptor, const void *data, size_t length);
ssize_t __real_read(int descriptor, void *data, size_t length);
ssize_t __wrap_write(int descriptor, const void *data, size_t length)
{
    ++calls;
    if (strcmp(scenario, "interrupted") == 0 && calls == 1u) {
        errno = EINTR;
        return -1;
    }
    if (strcmp(scenario, "zero") == 0) {
        return 0;
    }
    if (strcmp(scenario, "partial-error") == 0 && calls > 1u) {
        errno = EIO;
        return -1;
    }
    if (strcmp(scenario, "partial") == 0 || strcmp(scenario, "partial-error") == 0) {
        length = length > 2u ? 2u : length;
    }
    return __real_write(descriptor, data, length);
}
ssize_t __wrap_read(int descriptor, void *data, size_t length)
{
    ++calls;
    if (strcmp(scenario, "read-interrupted") == 0 && calls == 1u) {
        errno = EINTR;
        return -1;
    }
    if (strcmp(scenario, "read-error") == 0) {
        errno = EIO;
        return -1;
    }
    return __real_read(descriptor, data, length);
}
#endif

static int format(void)
{
    char storage[128];
    Os_HostTextType text = {storage, sizeof(storage), 0u, 0};
    Os_HostTextField(&text, "zero=", 0u);
    Os_HostTextField(&text, " max=", UINT64_MAX);
    Os_HostTextAppend(&text, " hex=");
    Os_HostTextHex(&text, UINT64_MAX, 16u, 0);
    Os_HostTextAppend(&text, " fault=");
    Os_HostTextHex(&text, 0xa5u, 8u, 1);
    Os_HostTextByte(&text, '\0');
    Os_HostTextByte(&text, '\r');
    Os_HostTextByte(&text, '\n');
    return Os_HostTextWrite(&text) == 1 ? 0 : 10;
}

static int refusal(void)
{
    char storage[4] = {'x', 'x', 'x', 'x'};
    Os_HostTextType text = {storage, 3u, 0u, 0};
    if (strcmp(scenario, "capacity") == 0) {
        Os_HostTextAppend(&text, "abcd");
    } else if (strcmp(scenario, "hex-width") == 0) {
        Os_HostTextHex(&text, 0u, 17u, 0);
    } else if (strcmp(scenario, "hex-value") == 0) {
        Os_HostTextHex(&text, 256u, 2u, 0);
    } else if (strcmp(scenario, "hex-case") == 0) {
        Os_HostTextHex(&text, 0u, 1u, 2);
    } else {
        return 11;
    }
    return text.failed != 0 && storage[3] == 'x' && Os_HostTextWrite(&text) == 0 ? 0 : 12;
}

static int broken_pipe(void)
{
#ifdef _WIN32
    HANDLE reader, writer;
    if (!CreatePipe(&reader, &writer, NULL, 0u) || !CloseHandle(reader) ||
        !SetStdHandle(STD_OUTPUT_HANDLE, writer)) {
        return 20;
    }
#else
    int pipe_fds[2];
    if (signal(SIGPIPE, SIG_DFL) == SIG_ERR) {
        return 19;
    }
    sigset_t blocked, previous, pending;
    const int preserve = strcmp(scenario, "pending-pipe") == 0;
    if (pipe(pipe_fds) != 0 || close(pipe_fds[0]) != 0 || dup2(pipe_fds[1], 1) != 1 ||
        close(pipe_fds[1]) != 0) {
        return 20;
    }
    if (preserve != 0) {
        sigemptyset(&blocked);
        sigaddset(&blocked, SIGPIPE);
        if (pthread_sigmask(SIG_BLOCK, &blocked, &previous) != 0 || raise(SIGPIPE) != 0) {
            return 21;
        }
    }
#endif
    if (Os_HostWriteBytes(0, "x", 1u) != 0) {
        return 22;
    }
#ifndef _WIN32
    if (sigpending(&pending) != 0 || sigismember(&pending, SIGPIPE) != preserve) {
        return 23;
    }
    if (preserve != 0) {
        int received;
        if (sigwait(&blocked, &received) != 0 || received != SIGPIPE ||
            pthread_sigmask(SIG_SETMASK, &previous, NULL) != 0) {
            return 24;
        }
    }
#endif
    return 0;
}

int main(int argc, char **argv)
{
    char byte = 'x';
    int count;
    if (argc != 2) {
        return 2;
    }
    scenario = argv[1];
    if (strcmp(scenario, "format") == 0) {
        return format();
    }
    if (strcmp(scenario, "capacity") == 0 || strncmp(scenario, "hex-", 4u) == 0) {
        return refusal();
    }
    if (strcmp(scenario, "broken-pipe") == 0 || strcmp(scenario, "pending-pipe") == 0) {
        return broken_pipe();
    }
    if (strncmp(scenario, "read-", 5u) == 0) {
        if (strcmp(scenario, "read-error") == 0) {
            return Os_HostReadByte(&byte) == -1 && byte == 'x' ? 0 : 30;
        }
        while ((count = Os_HostReadByte(&byte)) == 1) {
            if (Os_HostWriteBytes(0, &byte, 1u) == 0) {
                return 31;
            }
        }
        return count == 0 ? 0 : 32;
    }
    count = Os_HostWriteBytes(0, "abcdef", 6u);
    return count == ((strcmp(scenario, "zero") == 0 || strcmp(scenario, "partial-error") == 0) ? 0
                                                                                               : 1)
               ? 0
               : 40;
}
