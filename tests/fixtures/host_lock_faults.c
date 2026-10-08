#include "Can.h"
#include "CanIf.h"
#include "Can_HostLock.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef _WIN32
#include <windows.h>
#else
#include <errno.h>
#include <pthread.h>
#endif

static const char *failure = "normal";
static unsigned attrs_closed;
static unsigned mutexes_closed;
const EcuPolicyConfig Ecu_Policy = {.tx_confirmation = ECU_TX_SYNCHRONOUS};

void Test_Abort(void) {
    (void)printf("LOCK_FATAL attrs=%u mutexes=%u\n", attrs_closed, mutexes_closed);
    exit(91);
}
#ifdef _WIN32
BOOL WINAPI Test_Once(PINIT_ONCE once, PINIT_ONCE_FN callback, PVOID parameter, LPVOID *context) {
    return strcmp(failure, "once") == 0 ? FALSE
                                        : InitOnceExecuteOnce(once, callback, parameter, context);
}
BOOL WINAPI Test_Init(LPCRITICAL_SECTION lock, DWORD spin, DWORD flags) {
    return strcmp(failure, "mutex-init") == 0 ? FALSE
                                              : InitializeCriticalSectionEx(lock, spin, flags);
}
BOOL WINAPI Test_Try(LPCRITICAL_SECTION lock) {
    return strcmp(failure, "busy") == 0 ? FALSE : TryEnterCriticalSection(lock);
}
#else
int Test_Once(pthread_once_t *once, void (*callback)(void)) {
    return strcmp(failure, "once") == 0 ? EINVAL : pthread_once(once, callback);
}
int Test_AttrInit(pthread_mutexattr_t *attributes) {
    return strcmp(failure, "attr-init") == 0 ? EINVAL : pthread_mutexattr_init(attributes);
}
int Test_AttrType(pthread_mutexattr_t *attributes, int type) {
    return strcmp(failure, "attr-type") == 0 ? EINVAL : pthread_mutexattr_settype(attributes, type);
}
int Test_AttrClose(pthread_mutexattr_t *attributes) {
    int result = pthread_mutexattr_destroy(attributes);
    ++attrs_closed;
    return strcmp(failure, "attr-close") == 0 ? EINVAL : result;
}
int Test_Init(pthread_mutex_t *lock, const pthread_mutexattr_t *attributes) {
    return strcmp(failure, "mutex-init") == 0 ? EINVAL : pthread_mutex_init(lock, attributes);
}
int Test_Close(pthread_mutex_t *lock) {
    ++mutexes_closed;
    return pthread_mutex_destroy(lock);
}
int Test_Lock(pthread_mutex_t *lock) {
    return strcmp(failure, "lock") == 0 ? EINVAL : pthread_mutex_lock(lock);
}
int Test_Try(pthread_mutex_t *lock) {
    if (strcmp(failure, "busy") == 0) {
        return EBUSY;
    }
    return strcmp(failure, "try-error") == 0 ? EINVAL : pthread_mutex_trylock(lock);
}
int Test_Unlock(pthread_mutex_t *lock) {
    return strcmp(failure, "unlock") == 0 ? EINVAL : pthread_mutex_unlock(lock);
}
#endif
void CanIf_TxConfirmation(PduIdType id) { (void)id; }
void CanIf_ControllerModeIndication(uint8_t id, Can_ControllerStateType mode) {
    (void)id;
    (void)mode;
}
void CanIf_ControllerBusOff(uint8_t id) { (void)id; }
EcuStatus CanIf_HostRxIndication(uint32_t id, uint8_t dlc, const uint8_t data[8], uint64_t now) {
    (void)id;
    (void)dlc;
    (void)data;
    (void)now;
    return ECU_OK;
}
static EcuStatus emit(uint32_t id, uint8_t dlc, const uint8_t data[8]) {
    (void)id;
    (void)dlc;
    (void)data;
    return ECU_OK;
}
int main(int argc, char **argv) {
    uint8_t bytes[8] = {1u};
    Can_PduType pdu = {.id = 1u, .length = 1u, .sdu = bytes};
    Can_ConfigType config = {.sink = emit};
    if (argc < 2) {
        return 1;
    }
    if (strcmp(argv[1], "normal") == 0) {
        Can_Init(&config);
        Can_Lock();
        Can_Lock();
        Can_Unlock();
        Can_Unlock();
        if (Can_TryLock() != 1) {
            return 2;
        }
        Can_Unlock();
        return 0;
    }
    if (strcmp(argv[1], "busy") == 0 || strcmp(argv[1], "try-error") == 0 ||
        strcmp(argv[1], "lock") == 0 || strcmp(argv[1], "unlock") == 0) {
        Can_Init(&config);
        (void)Can_SetControllerMode(0u, CAN_CS_STARTED);
    }
    failure = argv[1];
    if (argc == 3) {
        Can_Lock();
        Can_Unlock();
        return 3; /* No fatal lock path may return to its caller. */
    }
    if (strcmp(failure, "busy") == 0) {
        if (Can_TryLock() != 0 || Can_Write(0u, &pdu) != CAN_BUSY ||
            Can_TransmitPdu(0u, 1u, 1u, bytes) != ECU_ERR_CAN_BUSY) {
            return 4;
        }
    } else if (Can_TryLock() != -1 || Can_Write(0u, &pdu) != E_NOT_OK ||
               Can_TransmitPdu(0u, 1u, 1u, bytes) != ECU_ERR_CONTROLLER) {
        return 5;
    }
    return 0;
}
