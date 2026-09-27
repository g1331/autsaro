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
    InitializeCriticalSection(&can_lock);
    return TRUE;
}
void Can_Lock(void) {
    if (InitOnceExecuteOnce(&can_lock_once, Can_InitLock, NULL, NULL) == 0) {
        abort();
    }
    EnterCriticalSection(&can_lock);
}
int Can_TryLock(void) {
    if (InitOnceExecuteOnce(&can_lock_once, Can_InitLock, NULL, NULL) == 0) {
        abort();
    }
    return TryEnterCriticalSection(&can_lock) != 0;
}
void Can_Unlock(void) { LeaveCriticalSection(&can_lock); }
#else
#include <pthread.h>
static pthread_once_t can_lock_once = PTHREAD_ONCE_INIT;
static pthread_mutex_t can_lock;
static void Can_InitLock(void) {
    pthread_mutexattr_t attributes;
    if (pthread_mutexattr_init(&attributes) != 0) {
        abort();
    }
    if (pthread_mutexattr_settype(&attributes, PTHREAD_MUTEX_RECURSIVE) != 0) {
        abort();
    }
    if (pthread_mutex_init(&can_lock, &attributes) != 0) {
        abort();
    }
    if (pthread_mutexattr_destroy(&attributes) != 0) {
        abort();
    }
}
void Can_Lock(void) {
    if ((pthread_once(&can_lock_once, Can_InitLock) != 0) || (pthread_mutex_lock(&can_lock) != 0)) {
        abort();
    }
}
int Can_TryLock(void) {
    if (pthread_once(&can_lock_once, Can_InitLock) != 0) {
        abort();
    }
    return pthread_mutex_trylock(&can_lock) == 0;
}
void Can_Unlock(void) {
    if (pthread_mutex_unlock(&can_lock) != 0) {
        abort();
    }
}
#endif
