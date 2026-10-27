#include "Can_Host.h"
#include "Ecu_Config.h"
#include "CanIf.h"
#include "Can_HostLock.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <signal.h>
#include <assert.h>
#ifdef _WIN32
/* Platform_Types.h already owns boolean; keep the Win32 spelling apart. */
#define boolean Test_Win32Boolean
#include <windows.h>
#undef boolean
#else
#include <errno.h>
#include <pthread.h>
#endif

static const char *failure = "normal";
static int reject_unlock;
const EcuPolicyConfig Ecu_Policy = {.tx_confirmation = ECU_TX_SYNCHRONOUS};

static void fatal_status(unsigned status) __attribute__((noreturn));
static void fatal_status(unsigned status)
{
    if (status != 134u) {
        exit(92);
    }
    (void)printf("LOCK_FATAL\n");
    exit(91);
}
static void check_unlock(void)
{
    if (reject_unlock != 0) {
        (void)puts("UNEXPECTED_UNLOCK");
        exit(94);
    }
}
#ifdef _WIN32
void Test_Trap(void)
{
    (void)puts("LOCK_TRAP");
    exit(92);
}
BOOL WINAPI Test_Terminate(HANDLE process, UINT status)
{
    if (strcmp(failure, "terminate-refused") == 0) {
        return FALSE;
    }
    (void)process;
    fatal_status(status);
    return FALSE;
}
BOOL WINAPI Test_Once(PINIT_ONCE once, PINIT_ONCE_FN callback, PVOID parameter, LPVOID *context)
{
    return strcmp(failure, "once") == 0 ? FALSE
                                        : InitOnceExecuteOnce(once, callback, parameter, context);
}
void WINAPI Test_Leave(LPCRITICAL_SECTION lock)
{
    check_unlock();
    LeaveCriticalSection(lock);
}
BOOL WINAPI Test_Init(LPCRITICAL_SECTION lock, DWORD spin, DWORD flags)
{
    return strcmp(failure, "mutex-init") == 0 ? FALSE
                                              : InitializeCriticalSectionEx(lock, spin, flags);
}
#else
void Test_Exit(int status) { fatal_status((unsigned)status); }
int Test_Once(pthread_once_t *once, void (*callback)(void))
{
    return strcmp(failure, "once") == 0 ? EINVAL : pthread_once(once, callback);
}
int Test_Init(pthread_mutex_t *lock, const pthread_mutexattr_t *attributes)
{
    return strcmp(failure, "mutex-init") == 0 ? EINVAL : pthread_mutex_init(lock, attributes);
}
int Test_Lock(pthread_mutex_t *lock)
{
    if ((strcmp(failure, "once") == 0) || (strcmp(failure, "mutex-init") == 0)) {
        (void)puts("UNEXPECTED_LOCK");
        exit(95);
    }
    return strcmp(failure, "lock") == 0 ? EINVAL : pthread_mutex_lock(lock);
}
int Test_Unlock(pthread_mutex_t *lock)
{
    check_unlock();
    return strcmp(failure, "unlock") == 0 ? EINVAL : pthread_mutex_unlock(lock);
}
#endif
void CanIf_TxConfirmation(PduIdType id) { (void)id; }
void CanIf_ControllerModeIndication(uint8_t id, Can_ControllerStateType mode)
{
    (void)id;
    (void)mode;
}
void CanIf_ControllerBusOff(uint8_t id) { (void)id; }
void CanIf_RxIndication(const Can_HwType *mailbox, const PduInfoType *info)
{
    (void)mailbox;
    (void)info;
}
static EcuStatus emit(uint32_t id, uint8_t dlc, const uint8_t data[8])
{
    (void)id;
    (void)dlc;
    (void)data;
    return ECU_OK;
}
static unsigned counts[CAN_HOST_AREA_COUNT + 1u];
static void cleanup(void) { (void)puts("CRT_CLEANUP"); }
static void caught_abort(int code)
{
    (void)code;
    _Exit(92);
}
#ifdef _WIN32
static DWORD WINAPI compete(LPVOID argument)
#else
static void *compete(void *argument)
#endif
{
    (void)argument;
    for (unsigned iteration = 0u; iteration < 2000u; ++iteration) {
        Can_Lock();
        ++counts[0];
        Can_Unlock();
        for (unsigned area = 0u; area < (unsigned)CAN_HOST_AREA_COUNT; ++area) {
            Can_HostAreaEnter((Can_HostAreaType)area);
            ++counts[area + 1u];
            Can_HostAreaExit((Can_HostAreaType)area);
        }
    }
#ifdef _WIN32
    return 0u;
#else
    return NULL;
#endif
}
#ifdef _WIN32
static DWORD WINAPI release_foreign_owner(LPVOID argument)
#else
static void *release_foreign_owner(void *argument)
#endif
{
    (void)argument;
    Can_Unlock();
#ifdef _WIN32
    return 0u;
#else
    return NULL;
#endif
}
static void foreign_exit(void)
{
    Can_Lock();
#ifdef _WIN32
    HANDLE thread = CreateThread(NULL, 0u, release_foreign_owner, NULL, 0u, NULL);
    assert(thread != NULL);
    assert(WaitForSingleObject(thread, INFINITE) == WAIT_OBJECT_0);
    assert(CloseHandle(thread) != 0);
#else
    pthread_t thread;
    assert(pthread_create(&thread, NULL, release_foreign_owner, NULL) == 0);
    assert(pthread_join(thread, NULL) == 0);
#endif
}
static void parallel(void)
{
#ifdef _WIN32
    HANDLE threads[4];
    for (unsigned i = 0u; i < 4u; ++i) {
        threads[i] = CreateThread(NULL, 0u, compete, NULL, 0u, NULL);
        assert(threads[i] != NULL);
    }
    for (unsigned i = 0u; i < 4u; ++i) {
        assert(WaitForSingleObject(threads[i], INFINITE) == WAIT_OBJECT_0);
        assert(CloseHandle(threads[i]) != 0);
    }
#else
    pthread_t threads[4];
    for (unsigned i = 0u; i < 4u; ++i) {
        assert(pthread_create(&threads[i], NULL, compete, NULL) == 0);
    }
    for (unsigned i = 0u; i < 4u; ++i) {
        assert(pthread_join(threads[i], NULL) == 0);
    }
#endif
    for (unsigned i = 0u; i < (unsigned)CAN_HOST_AREA_COUNT + 1u; ++i) {
        assert(counts[i] == 8000u);
    }
}
int main(int argc, char **argv)
{
    Can_ConfigType config = {.sink = emit};
    assert(atexit(cleanup) == 0);
    assert(signal(SIGABRT, caught_abort) != SIG_ERR);
    if (argc == 2 && strcmp(argv[1], "parallel") == 0) {
        parallel();
        return 0;
    }
    if (argc < 2) {
        return 1;
    }
    /* Publish once before any foreign thread starts; valid exits remain a clean control. */
    reject_unlock = strncmp(argv[1], "exit-", 5u) == 0 || strcmp(argv[1], "invalid-exit") == 0 ||
                    strcmp(argv[1], "negative-exit") == 0;
    if (strcmp(argv[1], "normal") == 0) {
        Can_Init(&config);
        Can_Lock();
        Can_Unlock();
        /* Areas may be used one after another, each released before the next. */
        for (unsigned area = 0u; area < (unsigned)CAN_HOST_AREA_COUNT; ++area) {
            Can_HostAreaEnter((Can_HostAreaType)area);
            Can_HostAreaExit((Can_HostAreaType)area);
        }
        return 0;
    }
    if (strcmp(argv[1], "exit-other-thread") == 0) {
        foreign_exit();
        return 6;
    }
    if (strcmp(argv[1], "exit-driver") == 0) {
        Can_Unlock();
        return 6;
    }
    if (strcmp(argv[1], "exit-area") == 0) {
        Can_HostAreaExit(CAN_HOST_AREA_DET);
        return 6;
    }
    if (strcmp(argv[1], "exit-wrong") == 0) {
        Can_HostAreaEnter(CAN_HOST_AREA_COMM);
        Can_HostAreaExit(CAN_HOST_AREA_DET);
        return 6;
    }
    if (strcmp(argv[1], "exit-driver-as-area") == 0) {
        Can_Lock();
        Can_HostAreaExit(CAN_HOST_AREA_DET);
        return 6;
    }
    if (strcmp(argv[1], "exit-area-as-driver") == 0) {
        Can_HostAreaEnter(CAN_HOST_AREA_DET);
        Can_Unlock();
        return 6;
    }
    if (strcmp(argv[1], "invalid-enter") == 0) {
        Can_HostAreaEnter(CAN_HOST_AREA_COUNT);
        return 6;
    }
    if (strcmp(argv[1], "negative-enter") == 0) {
        Can_HostAreaEnter((Can_HostAreaType)-1);
        return 6;
    }
    if (strcmp(argv[1], "invalid-exit") == 0) {
        Can_HostAreaExit(CAN_HOST_AREA_COUNT);
        return 6;
    }
    if (strcmp(argv[1], "negative-exit") == 0) {
        Can_HostAreaExit((Can_HostAreaType)-1);
        return 6;
    }
    if (strcmp(argv[1], "nest-driver") == 0) {
        Can_Lock();
        Can_Lock();
        return 6;
    }
    /* A strict area (driver, Det) admits no other area, including itself. */
    if (strcmp(argv[1], "nest-strict") == 0) {
        Can_Lock();
        Can_HostAreaEnter(CAN_HOST_AREA_DET);
        return 6;
    }
    if (strcmp(argv[1], "nest-same") == 0) {
        Can_HostAreaEnter(CAN_HOST_AREA_DET);
        Can_HostAreaEnter(CAN_HOST_AREA_DET);
        return 6;
    }
    if (strcmp(argv[1], "lock") == 0 || strcmp(argv[1], "unlock") == 0) {
        Can_Init(&config);
        (void)Can_SetControllerMode(0u, CAN_CS_STARTED);
    }
    failure = argv[1];
    if (strcmp(failure, "terminate-refused") == 0) {
        Can_HostAreaEnter(CAN_HOST_AREA_COUNT);
        return 6;
    }
    if (argc == 3) {
        /* "area" drives a module area instance instead of the driver lock. */
        if (strcmp(argv[2], "area") == 0) {
            Can_HostAreaEnter(CAN_HOST_AREA_COMM);
            Can_HostAreaExit(CAN_HOST_AREA_COMM);
        } else {
            Can_Lock();
            Can_Unlock();
        }
        return 3; /* No fatal lock path may return to its caller. */
    }
    return 1;
}
