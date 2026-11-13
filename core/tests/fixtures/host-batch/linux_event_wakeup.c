/** @file Hold output publication until the tick producer arms its eventfd wait. */
#include "Os_Host.h"
#include "Os_Target.h"
#include <poll.h>
#include <pthread.h>
#include <unistd.h>

static pthread_mutex_t publication_lock = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t publication_changed = PTHREAD_COND_INITIALIZER;
static int publication_waiting;
static int waiter_armed;
static OS_HOST_THREAD_LOCAL int tick_waiter;

StatusType __real_Os_TargetWaitTick(uint64 ticket,
                                    uint32 timeout_ms,
                                    Os_TickCompletionType *record);
StatusType __real_Os_TargetNotifyNative(void);
int __real_poll(struct pollfd *fds, nfds_t count, int timeout);
int __real___poll_chk(struct pollfd *fds, nfds_t count, int timeout, size_t bytes);

StatusType __wrap_Os_TargetWaitTick(uint64 ticket, uint32 timeout_ms, Os_TickCompletionType *record)
{
    tick_waiter = 1;
    const StatusType status = __real_Os_TargetWaitTick(ticket, timeout_ms, record);
    tick_waiter = 0;
    return status;
}

StatusType __wrap_Os_TargetNotifyNative(void)
{
    if (pthread_mutex_lock(&publication_lock) != 0) {
        _exit(91);
    }
    publication_waiting = 1;
    while (waiter_armed == 0) {
        if (pthread_cond_wait(&publication_changed, &publication_lock) != 0) {
            _exit(92);
        }
    }
    if (pthread_mutex_unlock(&publication_lock) != 0) {
        _exit(93);
    }
    return __real_Os_TargetNotifyNative();
}

int __wrap_poll(struct pollfd *fds, nfds_t count, int timeout)
{
    if ((tick_waiter != 0) && (count == 2u) && (timeout > 0)) {
        if (pthread_mutex_lock(&publication_lock) != 0) {
            _exit(94);
        }
        if ((publication_waiting != 0) && (waiter_armed == 0)) {
            waiter_armed = 1;
            if (pthread_cond_signal(&publication_changed) != 0) {
                _exit(95);
            }
            static const char marker[] = "EVENT_WAKE waiter=armed publication=released\n";
            if (write(STDERR_FILENO, marker, sizeof(marker) - 1u) != sizeof(marker) - 1u) {
                _exit(96);
            }
        }
        if (pthread_mutex_unlock(&publication_lock) != 0) {
            _exit(97);
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
