/** Real concurrent reporting with caller-owned, reentrant runtime callouts. */
#include "Det.h"
#include <assert.h>
#include <stddef.h>
#if defined(_WIN32)
#include <windows.h>
#else
#include <pthread.h>
#endif

static __thread unsigned notifications;
static Std_ReturnType observe(uint16 module, uint8 instance, uint8 api, uint8 error) {
    assert(module == 60u && instance == 2u && api == 5u && error == 7u);
    ++notifications;
    return E_NOT_OK;
}
static const Det_ErrorHookType hooks[] = {observe};
static const Det_ConfigType configuration = {NULL_PTR, 0u, hooks, 1u};

#if defined(_WIN32)
static DWORD WINAPI report(LPVOID result) {
#else
static void *report(void *result) {
#endif
    unsigned index;
    unsigned *count = result;
    for (index = 0u; index < 1000u; ++index) {
        assert(Det_ReportRuntimeError(60u, 2u, 5u, 7u) == E_OK);
    }
    *count = notifications;
#if defined(_WIN32)
    return 0u;
#else
    return NULL_PTR;
#endif
}

int main(void) {
    unsigned first = 0u;
    unsigned second = 0u;
#if defined(_WIN32)
    HANDLE threads[2];
#else
    pthread_t threads[2];
#endif
    Det_Init(&configuration);
#if defined(_WIN32)
    threads[0] = CreateThread(NULL, 0u, report, &first, 0u, NULL);
    threads[1] = CreateThread(NULL, 0u, report, &second, 0u, NULL);
    assert(threads[0] != NULL && threads[1] != NULL);
    assert(WaitForMultipleObjects(2u, threads, TRUE, INFINITE) == WAIT_OBJECT_0);
    assert(CloseHandle(threads[0]) != 0 && CloseHandle(threads[1]) != 0);
#else
    assert(pthread_create(&threads[0], NULL, report, &first) == 0);
    assert(pthread_create(&threads[1], NULL, report, &second) == 0);
    assert(pthread_join(threads[0], NULL) == 0);
    assert(pthread_join(threads[1], NULL) == 0);
#endif
    assert(first == 1000u && second == 1000u);
    assert(notifications == 0u);
    return 0;
}
