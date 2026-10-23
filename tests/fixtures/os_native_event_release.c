/** @file Concurrent native event failure/reuse exercises the actual reservation publication. */
#define _POSIX_C_SOURCE 200809L
#include "Os_Host.h"
#include "Os.h"
#include <errno.h>
#include <pthread.h>
#include <stdio.h>
#include <string.h>
#include <sys/eventfd.h>

static pthread_barrier_t creators;
static unsigned calls;
static boolean fail_first;
typedef struct {
    Os_HostHandleType first;
    Os_HostHandleType second;
} Creator;
static Creator results[2];
void StartupHook(void) {}
void ShutdownHook(StatusType reason) { (void)reason; }

int __real_eventfd(unsigned initial, int flags);
int __wrap_eventfd(unsigned initial, int flags)
{
    const unsigned index = __atomic_fetch_add(&calls, 1u, __ATOMIC_RELAXED);
    if (index < 2u) {
        const int status = pthread_barrier_wait(&creators);
        if ((status != 0) && (status != PTHREAD_BARRIER_SERIAL_THREAD)) {
            return -1;
        }
    }
    if ((index == 0u) && fail_first) {
        errno = EMFILE;
        return -1;
    }
    return __real_eventfd(initial, flags);
}

static void *create(void *argument)
{
    Creator *result = argument;
    result->first = Os_HostOpenManualEvent();
    if ((result->first != NULL) && fail_first) {
        result->second = Os_HostOpenManualEvent();
    }
    return NULL;
}

int main(int argc, char **argv)
{
    pthread_t workers[2];
    unsigned failed = 0u;
    unsigned successful = 0u;
    if ((argc != 2) || ((strcmp(argv[1], "failure") != 0) && (strcmp(argv[1], "success") != 0))) {
        return 1;
    }
    fail_first = strcmp(argv[1], "failure") == 0;
    if ((pthread_barrier_init(&creators, NULL, 2u) != 0) ||
        (pthread_create(&workers[0], NULL, create, &results[0]) != 0) ||
        (pthread_create(&workers[1], NULL, create, &results[1]) != 0)) {
        return 2;
    }
    if ((pthread_join(workers[0], NULL) != 0) || (pthread_join(workers[1], NULL) != 0) ||
        (pthread_barrier_destroy(&creators) != 0)) {
        return 3;
    }
    for (unsigned index = 0u; index < 2u; ++index) {
        if (results[index].first == NULL) {
            ++failed;
        } else {
            ++successful;
            if ((Os_HostWait(results[index].first, 0u) != 0) ||
                (Os_HostClose(results[index].first) != 1)) {
                return 4;
            }
        }
        if (results[index].second != NULL) {
            ++successful;
            if (Os_HostClose(results[index].second) != 1) {
                return 5;
            }
        }
    }
    if ((successful != 2u) || (failed != (fail_first ? 1u : 0u))) {
        return 6;
    }
    puts("concurrent native event construction and failure reuse observed");
    return 0;
}
