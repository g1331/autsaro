/** Independent publication/capacity/output oracle using the real ECU record implementation. */
#include "Ecu_ComRejection.h"
#include "Os_Host.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#if defined(_WIN32)
#define THREAD_RETURN DWORD WINAPI
#else
#include <pthread.h>
#define THREAD_RETURN void *
#endif
static int preempt;
static int output_failed;
static unsigned lines;
static unsigned markers;
static unsigned ends;
static unsigned seen[256];
Os_Atomic32Type Os_HostAtomicLoad(volatile Os_Atomic32Type *cell)
{
    return __atomic_load_n(cell, __ATOMIC_SEQ_CST);
}
Os_Atomic32Type Os_HostAtomicExchange(volatile Os_Atomic32Type *cell, Os_Atomic32Type value)
{
    return __atomic_exchange_n(cell, value, __ATOMIC_SEQ_CST);
}
Os_Atomic32Type Os_HostAtomicCompareExchange(volatile Os_Atomic32Type *cell,
                                             Os_Atomic32Type desired,
                                             Os_Atomic32Type expected)
{
    Os_Atomic32Type prior = expected;
    __atomic_compare_exchange_n(cell, &prior, desired, 0, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST);
    if ((preempt != 0) && (prior == expected)) {
        preempt = 0;
        /* Writer owns slot zero but has not copied or published; another ISR/Task may run. */
        Ecu_ComRecordRejection(7u, 3u);
        assert(Ecu_ComDrainRejections(FALSE) == 1 && lines == 1u && seen[7] == 1u);
        assert(Ecu_ComDrainRejections(FALSE) == 1 && lines == 1u);
        if (desired == 1 && output_failed == 2) {
            assert(Ecu_ComDrainRejections(TRUE) == 1 && markers == 1u && ends == 1u);
            exit(0); /* Finalization with an unfinished writer is explicitly incomplete. */
        }
    }
    return prior;
}
int Os_HostWriteBytes(int error_stream, const char *data, size_t length)
{
    char text[128];
    unsigned id, result, count;
    assert(error_stream == 1 && length < sizeof(text));
    if (output_failed == 1) {
        return 0;
    }
    memcpy(text, data, length);
    text[length] = '\0';
    if (sscanf(text, "COM_TX_REJECT pdu=%u result=%u", &id, &result) == 2) {
        assert(id < 256u && result == 3u);
        ++seen[id];
        ++lines;
    } else if (sscanf(text, "COM_TX_REJECT_END count=%u", &count) == 1) {
        assert(count == lines);
        ++ends;
    } else {
        assert(strcmp(text, "COM_TX_REJECT_INCOMPLETE\n") == 0);
        ++markers;
    }
    return 1;
}
static THREAD_RETURN writer(void *argument)
{
    unsigned index;
    unsigned id = (unsigned)(uintptr_t)argument;
    for (index = 0u; index < 256u; ++index) {
        Ecu_ComRecordRejection((PduIdType)id, 3u);
    }
    return 0;
}
int main(int argc, char **argv)
{
    unsigned index;
    assert(argc == 2);
    if (strcmp(argv[1], "preempt") == 0 || strcmp(argv[1], "pending") == 0) {
        preempt = 1;
        output_failed = strcmp(argv[1], "pending") == 0 ? 2 : 0;
        Ecu_ComRecordRejection(5u, 3u);
        assert(Ecu_ComDrainRejections(FALSE) == 1 && lines == 2u && seen[5] == 1u);
        assert(Ecu_ComDrainRejections(TRUE) == 1 && ends >= 1u);
        assert(markers == (output_failed == 2 ? 1u : 0u));
    } else if (strcmp(argv[1], "end-output") == 0) {
        output_failed = 1;
        assert(Ecu_ComDrainRejections(TRUE) == 0 && lines == 0u && ends == 0u);
    } else if (strcmp(argv[1], "output") == 0) {
        Ecu_ComRecordRejection(8u, 3u);
        output_failed = 1;
        assert(Ecu_ComDrainRejections(FALSE) == 0 && lines == 0u);
        output_failed = 0;
        assert(Ecu_ComDrainRejections(FALSE) == 1 && lines == 1u && seen[8] == 1u);
        assert(Ecu_ComDrainRejections(TRUE) == 1 && ends == 1u);
    } else if (strcmp(argv[1], "capacity") == 0) {
        for (index = 0u; index < 4096u; ++index) {
            Ecu_ComRecordRejection(9u, 3u);
        }
        assert(Ecu_ComDrainRejections(FALSE) == 1 && lines == 4096u && markers == 0u);
        Ecu_ComRecordRejection(10u, 3u);
        assert(Ecu_ComDrainRejections(FALSE) == 1 && lines == 4096u && markers == 1u);
        assert(Ecu_ComDrainRejections(TRUE) == 1 && markers == 1u && ends == 1u);
    } else {
#if defined(_WIN32)
        HANDLE threads[8];
        for (index = 0; index < 8; ++index) {
            threads[index] = CreateThread(NULL, 0, writer, (void *)(uintptr_t)index, 0, NULL);
            assert(threads[index] != NULL);
        }
        for (index = 0; index < 8; ++index) {
            assert(WaitForSingleObject(threads[index], INFINITE) == WAIT_OBJECT_0);
            assert(CloseHandle(threads[index]));
        }
#else
        pthread_t threads[8];
        for (index = 0; index < 8; ++index) {
            assert(pthread_create(&threads[index], NULL, writer, (void *)(uintptr_t)index) == 0);
        }
        for (index = 0; index < 8; ++index) {
            assert(pthread_join(threads[index], NULL) == 0);
        }
#endif
        assert(Ecu_ComDrainRejections(FALSE) == 1 && lines == 2048u);
        assert(Ecu_ComDrainRejections(FALSE) == 1 && lines == 2048u);
        for (index = 0; index < 8; ++index) {
            assert(seen[index] == 256u);
        }
        assert(Ecu_ComDrainRejections(TRUE) == 1 && markers == 0u && ends == 1u);
    }
    return 0;
}
