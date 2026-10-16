/** @file Host driver lock and strict SchM exclusive areas with nesting detection (ADR 0018). */
#if !defined(_WIN32)
#define _XOPEN_SOURCE 700
#endif
#include "Can_HostLock.h"
#include <stddef.h>
#include <stdlib.h>
#if defined(_WIN32)
#include <windows.h>
#else
#include <pthread.h>
#endif
/* Every unrecoverable lock state stops here: a void caller must not continue. */
static void Can_Fatal(void) { abort(); }
/* SchM exclusive-area accounting (ADR 0018). A strict area (driver, Det, ...) protects
 * module data only: a thread holding one must not enter another area. */
static __thread unsigned strict_held;
static void Can_NoteEnter(void)
{
    if (strict_held != 0u) {
        Can_Fatal();
    }
    ++strict_held;
}
static void Can_NoteExit(void)
{
    if (strict_held != 0u) {
        --strict_held;
    }
}
#if defined(_WIN32)
static CRITICAL_SECTION can_lock;
static CRITICAL_SECTION area_locks[CAN_HOST_AREA_COUNT];
static BOOL CALLBACK Can_InitLock(PINIT_ONCE once, PVOID parameter, PVOID *context)
{
    (void)once;
    (void)parameter;
    (void)context;
    return InitializeCriticalSectionEx(&can_lock, 0u, 0u);
}
static BOOL CALLBACK Can_InitAreas(PINIT_ONCE once, PVOID parameter, PVOID *context)
{
    BOOL initialized = TRUE;
    size_t area;
    (void)once;
    (void)parameter;
    (void)context;
    for (area = 0u; (area < (size_t)CAN_HOST_AREA_COUNT) && (initialized != FALSE); ++area) {
        initialized = InitializeCriticalSectionEx(&area_locks[area], 0u, 0u);
    }
    return initialized;
}
static void Can_EnterRecursive(PINIT_ONCE once, PINIT_ONCE_FN init, LPCRITICAL_SECTION lock)
{
    if (InitOnceExecuteOnce(once, init, NULL, NULL) == 0) {
        Can_Fatal();
    }
    EnterCriticalSection(lock);
}
void Can_Lock(void)
{
    static INIT_ONCE can_lock_once = INIT_ONCE_STATIC_INIT;
    Can_NoteEnter();
    Can_EnterRecursive(&can_lock_once, Can_InitLock, &can_lock);
}
void Can_Unlock(void)
{
    LeaveCriticalSection(&can_lock);
    Can_NoteExit();
}
void Can_HostAreaEnter(Can_HostAreaType area)
{
    static INIT_ONCE area_once = INIT_ONCE_STATIC_INIT;
    if (area >= CAN_HOST_AREA_COUNT) {
        Can_Fatal();
    }
    Can_NoteEnter();
    Can_EnterRecursive(&area_once, Can_InitAreas, &area_locks[area]);
}
void Can_HostAreaExit(Can_HostAreaType area)
{
    if (area >= CAN_HOST_AREA_COUNT) {
        Can_Fatal();
    }
    LeaveCriticalSection(&area_locks[area]);
    Can_NoteExit();
}
#else
typedef struct {
    pthread_mutex_t mutex;
    int ready;
} Can_RecursiveLock;
static Can_RecursiveLock can_lock;
static Can_RecursiveLock area_locks[CAN_HOST_AREA_COUNT];
static void Can_InitRecursive(Can_RecursiveLock *lock)
{
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
static void Can_InitAreas(void)
{
    size_t area;
    for (area = 0u; area < (size_t)CAN_HOST_AREA_COUNT; ++area) {
        Can_InitRecursive(&area_locks[area]);
    }
}
static void Can_EnterRecursive(pthread_once_t *once, void (*init)(void), Can_RecursiveLock *lock)
{
    if ((pthread_once(once, init) != 0) || (lock->ready == 0) ||
        (pthread_mutex_lock(&lock->mutex) != 0)) {
        Can_Fatal();
    }
}
static void Can_LeaveRecursive(Can_RecursiveLock *lock)
{
    if (pthread_mutex_unlock(&lock->mutex) != 0) {
        /* The lock ownership state is no longer reliable. */
        Can_Fatal();
    }
}
void Can_Lock(void)
{
    static pthread_once_t can_lock_once = PTHREAD_ONCE_INIT;
    Can_NoteEnter();
    Can_EnterRecursive(&can_lock_once, Can_InitLock, &can_lock);
}
void Can_Unlock(void)
{
    Can_LeaveRecursive(&can_lock);
    Can_NoteExit();
}
void Can_HostAreaEnter(Can_HostAreaType area)
{
    static pthread_once_t area_once = PTHREAD_ONCE_INIT;
    if (area >= CAN_HOST_AREA_COUNT) {
        Can_Fatal();
    }
    Can_NoteEnter();
    Can_EnterRecursive(&area_once, Can_InitAreas, &area_locks[area]);
}
void Can_HostAreaExit(Can_HostAreaType area)
{
    if (area >= CAN_HOST_AREA_COUNT) {
        Can_Fatal();
    }
    Can_LeaveRecursive(&area_locks[area]);
    Can_NoteExit();
}
#endif
