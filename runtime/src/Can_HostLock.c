#if !defined(_WIN32)
#define _XOPEN_SOURCE 700
#endif
#include "Can_HostLock.h"
#include <stddef.h>
#include <stdlib.h>
#if defined(_WIN32)
#include <windows.h>
static INIT_ONCE can_lock_once = INIT_ONCE_STATIC_INIT;
static CRITICAL_SECTION can_lock;
static BOOL CALLBACK Can_InitLock(PINIT_ONCE once, PVOID parameter, PVOID *context) {
    (void)once;
    (void)parameter;
    (void)context;
    return InitializeCriticalSectionEx(&can_lock, 0u, 0u);
}
void Can_Lock(void) {
    if (InitOnceExecuteOnce(&can_lock_once, Can_InitLock, NULL, NULL) == 0) {
        /* A void caller cannot continue without its shared-state lock. */
        abort();
    }
    EnterCriticalSection(&can_lock);
}
int Can_TryLock(void) {
    int result = -1;
    if (InitOnceExecuteOnce(&can_lock_once, Can_InitLock, NULL, NULL) != 0) {
        result = TryEnterCriticalSection(&can_lock) != 0 ? 1 : 0;
    }
    return result;
}
void Can_Unlock(void) { LeaveCriticalSection(&can_lock); }
#else
#include <errno.h>
#include <pthread.h>
static pthread_once_t can_lock_once = PTHREAD_ONCE_INIT;
static pthread_mutex_t can_lock;
static int can_lock_ready;
static void Can_InitLock(void) {
    pthread_mutexattr_t attributes;
    int initialized = 0;
    int result = pthread_mutexattr_init(&attributes);
    if (result == 0) {
        result = pthread_mutexattr_settype(&attributes, PTHREAD_MUTEX_RECURSIVE);
        if (result == 0) {
            result = pthread_mutex_init(&can_lock, &attributes);
            initialized = result == 0 ? 1 : 0;
        }
        if (pthread_mutexattr_destroy(&attributes) != 0) {
            result = -1;
        }
        if ((result != 0) && (initialized != 0)) {
            /* Never publish a partially constructed lock; retire it if possible. */
            if (pthread_mutex_destroy(&can_lock) != 0) {
                result = -1;
            }
        }
    }
    can_lock_ready = result == 0 ? 1 : 0;
}
void Can_Lock(void) {
    if ((pthread_once(&can_lock_once, Can_InitLock) != 0) || (can_lock_ready == 0) ||
        (pthread_mutex_lock(&can_lock) != 0)) {
        /* Returning would let a void caller access unprotected shared state. */
        abort();
    }
}
int Can_TryLock(void) {
    int result = -1;
    if ((pthread_once(&can_lock_once, Can_InitLock) == 0) && (can_lock_ready != 0)) {
        int locked = pthread_mutex_trylock(&can_lock);
        if (locked == 0) {
            result = 1;
        } else if (locked == EBUSY) {
            result = 0;
        } else {
            /* Preserve synchronization errors separately from normal contention. */
        }
    }
    return result;
}
void Can_Unlock(void) {
    if (pthread_mutex_unlock(&can_lock) != 0) {
        /* The lock ownership state is no longer reliable. */
        abort();
    }
}
#endif
