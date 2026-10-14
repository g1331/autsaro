#if !defined(_WIN32)
#define _XOPEN_SOURCE 700
#endif
#include "Can_HostLock.h"
#include <stddef.h>
#include <stdlib.h>
#if defined(_WIN32)
#include <windows.h>
#else
#include <errno.h>
#include <pthread.h>
#endif
/* Every unrecoverable lock state stops here: a void caller must not continue. */
static void Can_Fatal(void) { abort(); }
/* SchM exclusive-area accounting (ADR 0018). A strict area (driver, Det, ...) protects
 * module data only: a thread must hold no other area while it is inside one, and must
 * not enter another area from within it. The legacy shared state area may still
 * enclose strict areas (not the reverse) until its last user is migrated. */
static __thread unsigned strict_held;
static void Can_NoteEnter(int strict) {
    if (strict_held != 0u) {
        Can_Fatal();
    }
    if (strict != 0) {
        ++strict_held;
    }
}
static void Can_NoteExit(int strict) {
    if ((strict != 0) && (strict_held != 0u)) {
        --strict_held;
    }
}
#if defined(_WIN32)
static INIT_ONCE can_lock_once = INIT_ONCE_STATIC_INIT;
static CRITICAL_SECTION can_lock;
static CRITICAL_SECTION state_lock;
static CRITICAL_SECTION det_lock;
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
static BOOL CALLBACK Can_InitDetLock(PINIT_ONCE once, PVOID parameter, PVOID *context) {
    (void)once;
    (void)parameter;
    (void)context;
    return InitializeCriticalSectionEx(&det_lock, 0u, 0u);
}
static void Can_EnterRecursive(PINIT_ONCE once, PINIT_ONCE_FN init, LPCRITICAL_SECTION lock) {
    if (InitOnceExecuteOnce(once, init, NULL, NULL) == 0) {
        Can_Fatal();
    }
    EnterCriticalSection(lock);
}
void Can_Lock(void) {
    Can_NoteEnter(1);
    Can_EnterRecursive(&can_lock_once, Can_InitLock, &can_lock);
}
int Can_TryLock(void) {
    int result = -1;
    Can_NoteEnter(1);
    if (InitOnceExecuteOnce(&can_lock_once, Can_InitLock, NULL, NULL) != 0) {
        result = TryEnterCriticalSection(&can_lock) != 0 ? 1 : 0;
    }
    if (result != 1) {
        Can_NoteExit(1);
    }
    return result;
}
void Can_Unlock(void) {
    LeaveCriticalSection(&can_lock);
    Can_NoteExit(1);
}
void Can_HostDetLock(void) {
    static INIT_ONCE det_lock_once = INIT_ONCE_STATIC_INIT;
    Can_NoteEnter(1);
    Can_EnterRecursive(&det_lock_once, Can_InitDetLock, &det_lock);
}
void Can_HostDetUnlock(void) {
    LeaveCriticalSection(&det_lock);
    Can_NoteExit(1);
}
void Can_HostStateLock(void) {
    static INIT_ONCE state_lock_once = INIT_ONCE_STATIC_INIT;
    Can_NoteEnter(0);
    Can_EnterRecursive(&state_lock_once, Can_InitStateLock, &state_lock);
}
void Can_HostStateUnlock(void) { LeaveCriticalSection(&state_lock); }
#else
typedef struct {
    pthread_mutex_t mutex;
    int ready;
} Can_RecursiveLock;
static pthread_once_t can_lock_once = PTHREAD_ONCE_INIT;
static Can_RecursiveLock can_lock;
static Can_RecursiveLock state_lock;
static Can_RecursiveLock det_lock;
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
static void Can_InitDetLock(void) { Can_InitRecursive(&det_lock); }
static void Can_EnterRecursive(pthread_once_t *once, void (*init)(void), Can_RecursiveLock *lock) {
    if ((pthread_once(once, init) != 0) || (lock->ready == 0) ||
        (pthread_mutex_lock(&lock->mutex) != 0)) {
        Can_Fatal();
    }
}
static void Can_LeaveRecursive(Can_RecursiveLock *lock) {
    if (pthread_mutex_unlock(&lock->mutex) != 0) {
        /* The lock ownership state is no longer reliable. */
        Can_Fatal();
    }
}
void Can_Lock(void) {
    Can_NoteEnter(1);
    Can_EnterRecursive(&can_lock_once, Can_InitLock, &can_lock);
}
int Can_TryLock(void) {
    int result = -1;
    Can_NoteEnter(1);
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
    if (result != 1) {
        Can_NoteExit(1);
    }
    return result;
}
void Can_Unlock(void) {
    Can_LeaveRecursive(&can_lock);
    Can_NoteExit(1);
}
void Can_HostDetLock(void) {
    static pthread_once_t det_lock_once = PTHREAD_ONCE_INIT;
    Can_NoteEnter(1);
    Can_EnterRecursive(&det_lock_once, Can_InitDetLock, &det_lock);
}
void Can_HostDetUnlock(void) {
    Can_LeaveRecursive(&det_lock);
    Can_NoteExit(1);
}
void Can_HostStateLock(void) {
    static pthread_once_t state_lock_once = PTHREAD_ONCE_INIT;
    Can_NoteEnter(0);
    Can_EnterRecursive(&state_lock_once, Can_InitStateLock, &state_lock);
}
void Can_HostStateUnlock(void) { Can_LeaveRecursive(&state_lock); }
#endif
