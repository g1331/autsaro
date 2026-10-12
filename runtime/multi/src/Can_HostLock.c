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
static CRITICAL_SECTION state_lock;
static BOOL CALLBACK Can_InitLock(PINIT_ONCE once, PVOID parameter, PVOID *context) {
    (void)once;
    (void)parameter;
    (void)context;
    return InitializeCriticalSectionEx(&can_lock, 0u, 0u);
}
static BOOL CALLBACK Can_InitStateLock(PINIT_ONCE once, PVOID parameter, PVOID *context) {
    (void)once;
    (void)parameter;
    (void)context;
    return InitializeCriticalSectionEx(&state_lock, 0u, 0u);
}
static void Can_EnterRecursive(PINIT_ONCE once, PINIT_ONCE_FN init, LPCRITICAL_SECTION lock) {
    if (InitOnceExecuteOnce(once, init, NULL, NULL) == 0) {
        /* A void caller cannot continue without its shared-state lock. */
        abort();
    }
    EnterCriticalSection(lock);
}
void Can_Lock(void) { Can_EnterRecursive(&can_lock_once, Can_InitLock, &can_lock); }
int Can_TryLock(void) {
    int result = -1;
    if (InitOnceExecuteOnce(&can_lock_once, Can_InitLock, NULL, NULL) != 0) {
        result = TryEnterCriticalSection(&can_lock) != 0 ? 1 : 0;
    }
    return result;
}
void Can_Unlock(void) { LeaveCriticalSection(&can_lock); }
void Can_HostStateLock(void) {
    static INIT_ONCE state_lock_once = INIT_ONCE_STATIC_INIT;
    Can_EnterRecursive(&state_lock_once, Can_InitStateLock, &state_lock);
}
void Can_HostStateUnlock(void) { LeaveCriticalSection(&state_lock); }
#else
#include <errno.h>
#include <pthread.h>
typedef struct {
    pthread_mutex_t mutex;
    int ready;
} Can_RecursiveLock;
static pthread_once_t can_lock_once = PTHREAD_ONCE_INIT;
static Can_RecursiveLock can_lock;
static Can_RecursiveLock state_lock;
static void Can_InitRecursive(Can_RecursiveLock *lock) {
    pthread_mutexattr_t attributes;
    int initialized = 0;
    int result = pthread_mutexattr_init(&attributes);
    if (result == 0) {
        result = pthread_mutexattr_settype(&attributes, PTHREAD_MUTEX_RECURSIVE);
        if (result == 0) {
            result = pthread_mutex_init(&lock->mutex, &attributes);
            initialized = (result == 0) ? 1 : 0;
        }
        if (pthread_mutexattr_destroy(&attributes) != 0) {
            result = -1;
        }
        if ((result != 0) && (initialized != 0)) {
            /* Never publish a partially constructed lock; retire it if possible. */
            if (pthread_mutex_destroy(&lock->mutex) != 0) {
                result = -1;
            }
        }
    }
    lock->ready = (result == 0) ? 1 : 0;
}
static void Can_InitLock(void) { Can_InitRecursive(&can_lock); }
static void Can_InitStateLock(void) { Can_InitRecursive(&state_lock); }
static void Can_EnterRecursive(pthread_once_t *once, void (*init)(void), Can_RecursiveLock *lock) {
    if ((pthread_once(once, init) != 0) || (lock->ready == 0) ||
        (pthread_mutex_lock(&lock->mutex) != 0)) {
        /* Returning would let a void caller access unprotected shared state. */
        abort();
    }
}
static void Can_LeaveRecursive(Can_RecursiveLock *lock) {
    if (pthread_mutex_unlock(&lock->mutex) != 0) {
        /* The lock ownership state is no longer reliable. */
        abort();
    }
}
void Can_Lock(void) { Can_EnterRecursive(&can_lock_once, Can_InitLock, &can_lock); }
int Can_TryLock(void) {
    int result = -1;
    if ((pthread_once(&can_lock_once, Can_InitLock) == 0) && (can_lock.ready != 0)) {
        int locked = pthread_mutex_trylock(&can_lock.mutex);
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
void Can_Unlock(void) { Can_LeaveRecursive(&can_lock); }
void Can_HostStateLock(void) {
    static pthread_once_t state_lock_once = PTHREAD_ONCE_INIT;
    Can_EnterRecursive(&state_lock_once, Can_InitStateLock, &state_lock);
}
void Can_HostStateUnlock(void) { Can_LeaveRecursive(&state_lock); }
#endif
