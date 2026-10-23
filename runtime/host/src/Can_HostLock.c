/** @file Host driver lock and strict SchM exclusive areas with nesting detection. */
#include "Can_HostLock.h"
#include <stddef.h>
#if defined(_WIN32)
#include <windows.h>
#else
#include <pthread.h>
#include <unistd.h>
#endif
#define CAN_HOST_FATAL_STATUS    134
#define CAN_HOST_DRIVER_OWNER    1u
#define CAN_HOST_AREA_OWNER_BASE 2u

/* Every unrecoverable lock state stops here: a void caller must not continue. */
static void Can_Fatal(void)
{
#if defined(_WIN32)
    (void)TerminateProcess(GetCurrentProcess(), CAN_HOST_FATAL_STATUS);
    /* Self-termination never returns on success; trap if the platform refuses it. */
    __builtin_trap();
#else
    _exit(CAN_HOST_FATAL_STATUS);
#endif
}
/* SchM exclusive-area accounting. A strict area (driver, Det, ...) protects
 * module data only: a thread holding one must not enter another area. */
static __thread unsigned
    strict_held; /**< Actor-local owner token (zero when free); no Init reset. */
static void Can_NoteEnter(unsigned owner)
{
    if (strict_held != 0u) {
        Can_Fatal();
    }
    strict_held = owner;
}
static void Can_CheckExit(unsigned owner)
{
    /* Default POSIX mutexes require us to reject wrong-thread/wrong-area exits first. */
    if (strict_held != owner) {
        Can_Fatal();
    }
}
#if defined(_WIN32)
static CRITICAL_SECTION can_lock; /**< Process lock, once-initialized; no ECU Init reset. */
static CRITICAL_SECTION
    area_locks[CAN_HOST_AREA_COUNT]; /**< Once-initialized area mutexes; no ECU Init reset. */
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
static void Can_EnterLock(PINIT_ONCE once, PINIT_ONCE_FN init, LPCRITICAL_SECTION lock)
{
    if (InitOnceExecuteOnce(once, init, NULL, NULL) == 0) {
        Can_Fatal();
    }
    EnterCriticalSection(lock);
}
void Can_Lock(void)
{
    static INIT_ONCE can_lock_once = INIT_ONCE_STATIC_INIT;
    Can_NoteEnter(CAN_HOST_DRIVER_OWNER);
    Can_EnterLock(&can_lock_once, Can_InitLock, &can_lock);
}
void Can_Unlock(void)
{
    Can_CheckExit(CAN_HOST_DRIVER_OWNER);
    LeaveCriticalSection(&can_lock);
    strict_held = 0u;
}
void Can_HostAreaEnter(Can_HostAreaType area)
{
    static INIT_ONCE area_once = INIT_ONCE_STATIC_INIT;
    if ((unsigned)area >= (unsigned)CAN_HOST_AREA_COUNT) {
        Can_Fatal();
    }
    Can_NoteEnter((unsigned)area + CAN_HOST_AREA_OWNER_BASE);
    Can_EnterLock(&area_once, Can_InitAreas, &area_locks[area]);
}
void Can_HostAreaExit(Can_HostAreaType area)
{
    if ((unsigned)area >= (unsigned)CAN_HOST_AREA_COUNT) {
        Can_Fatal();
    }
    Can_CheckExit((unsigned)area + CAN_HOST_AREA_OWNER_BASE);
    LeaveCriticalSection(&area_locks[area]);
    strict_held = 0u;
}
#else
typedef struct {
    pthread_mutex_t mutex;
    int ready;
} Can_HostLockType;
static Can_HostLockType can_lock; /**< Process lock, pthread_once-initialized; no ECU Init reset. */
static Can_HostLockType
    area_locks[CAN_HOST_AREA_COUNT]; /**< Once-initialized area mutexes; no ECU Init reset. */
static void Can_InitMutex(Can_HostLockType *lock)
{
    /* Strict ownership rejects recursion before entering the platform mutex. */
    lock->ready = (pthread_mutex_init(&lock->mutex, NULL) == 0) ? 1 : 0;
}
static void Can_InitLock(void) { Can_InitMutex(&can_lock); }
static void Can_InitAreas(void)
{
    size_t area;
    for (area = 0u; area < (size_t)CAN_HOST_AREA_COUNT; ++area) {
        Can_InitMutex(&area_locks[area]);
    }
}
static void Can_EnterLock(pthread_once_t *once, void (*init)(void), Can_HostLockType *lock)
{
    if ((pthread_once(once, init) != 0) || (lock->ready == 0) ||
        (pthread_mutex_lock(&lock->mutex) != 0)) {
        Can_Fatal();
    }
}
static void Can_LeaveLock(Can_HostLockType *lock)
{
    if (pthread_mutex_unlock(&lock->mutex) != 0) {
        /* The lock ownership state is no longer reliable. */
        Can_Fatal();
    }
}
void Can_Lock(void)
{
    static pthread_once_t can_lock_once = PTHREAD_ONCE_INIT;
    Can_NoteEnter(CAN_HOST_DRIVER_OWNER);
    Can_EnterLock(&can_lock_once, Can_InitLock, &can_lock);
}
void Can_Unlock(void)
{
    Can_CheckExit(CAN_HOST_DRIVER_OWNER);
    Can_LeaveLock(&can_lock);
    strict_held = 0u;
}
void Can_HostAreaEnter(Can_HostAreaType area)
{
    static pthread_once_t area_once = PTHREAD_ONCE_INIT;
    if ((unsigned)area >= (unsigned)CAN_HOST_AREA_COUNT) {
        Can_Fatal();
    }
    Can_NoteEnter((unsigned)area + CAN_HOST_AREA_OWNER_BASE);
    Can_EnterLock(&area_once, Can_InitAreas, &area_locks[area]);
}
void Can_HostAreaExit(Can_HostAreaType area)
{
    if ((unsigned)area >= (unsigned)CAN_HOST_AREA_COUNT) {
        Can_Fatal();
    }
    Can_CheckExit((unsigned)area + CAN_HOST_AREA_OWNER_BASE);
    Can_LeaveLock(&area_locks[area]);
    strict_held = 0u;
}
#endif
