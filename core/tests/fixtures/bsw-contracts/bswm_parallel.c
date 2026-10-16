/** Deterministic window oracle for BswM (ADR 0018); actual BswM, host areas and threads.
 * A dispatcher stops right after BswM left BSWM_STATE, before it runs the action. The
 * contender (main) then completes BswM calls meanwhile: none waits for the dispatcher or
 * runs an action itself; inputs it makes are arbitrated after the action in flight
 * (SWS_BswM_00069), and an input made after Deinit has no effect (SWS_BswM_00120). The
 * action enters another strict area, which aborts on the host if BswM still held its own.
 * Verifies[BswM]: SWS_BSW_00134
 */
#include "BswM.h"
#include "BswM_ComM.h"
#include "SchM_BswM.h"
#include "SchM_ComM.h"
#include <assert.h>
#if defined(_WIN32)
/* Threads only; skip COM/RPC headers whose boolean clashes with Platform_Types.h. */
#define WIN32_LEAN_AND_MEAN
#include <windows.h>
static CRITICAL_SECTION handshake;
static CONDITION_VARIABLE changed;
static DWORD dispatcher;
static void lock(void) { EnterCriticalSection(&handshake); }
static void unlock(void) { LeaveCriticalSection(&handshake); }
static void wake(void) { WakeAllConditionVariable(&changed); }
static void wait(void) { assert(SleepConditionVariableCS(&changed, &handshake, INFINITE)); }
static int dispatch_thread(void) { return GetCurrentThreadId() == dispatcher; }
#else
#include <pthread.h>
static pthread_mutex_t handshake = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t changed = PTHREAD_COND_INITIALIZER;
static pthread_t dispatcher;
static void lock(void) { assert(pthread_mutex_lock(&handshake) == 0); }
static void unlock(void) { assert(pthread_mutex_unlock(&handshake) == 0); }
static void wake(void) { assert(pthread_cond_broadcast(&changed) == 0); }
static void wait(void) { assert(pthread_cond_wait(&changed, &handshake) == 0); }
static int dispatch_thread(void) { return pthread_equal(pthread_self(), dispatcher); }
#endif

/* Number of the dispatcher's BSWM_STATE exit to stop after; zero when disarmed. */
static unsigned armed;
static unsigned paused;
static unsigned released;
static ComM_ModeType actions[8];
static unsigned action_count;
static unsigned action_depth;

static Std_ReturnType action(NetworkHandleType Network, ComM_ModeType Mode)
{
    assert(Network == 7u && action_count < 8u && action_depth == 0u);
    ++action_depth;
    SchM_Enter_ComM_COMM_STATE();
    SchM_Exit_ComM_COMM_STATE();
    actions[action_count++] = Mode;
    --action_depth;
    return E_OK;
}
static const BswM_ConfigType configuration = {7u, action};

void __real_SchM_Exit_BswM_BSWM_STATE(void);
void __wrap_SchM_Exit_BswM_BSWM_STATE(void)
{
    __real_SchM_Exit_BswM_BSWM_STATE();
    lock();
    if ((armed != 0u) && dispatch_thread()) {
        --armed;
        if (armed == 0u) {
            paused = 1u;
            wake();
            while (released == 0u) {
                wait();
            }
        }
    }
    unlock();
}

#if defined(_WIN32)
static DWORD WINAPI run(LPVOID argument)
{
    (void)argument;
    BswM_ComM_CurrentMode(7u, COMM_FULL_COMMUNICATION);
    return 0u;
}
#else
static void *run(void *argument)
{
    (void)argument;
    BswM_ComM_CurrentMode(7u, COMM_FULL_COMMUNICATION);
    return NULL_PTR;
}
#endif

/* Stops the dispatcher's FULL indication before its action; `contend` completes meanwhile.
 * Exit 1 records the input and claims processing; exit 2 takes it for the action. */
static void window(void (*contend)(void))
{
#if defined(_WIN32)
    HANDLE thread;
#else
    pthread_t thread;
#endif
    lock();
    armed = 2u;
    paused = 0u;
    released = 0u;
    action_count = 0u;
#if defined(_WIN32)
    thread = CreateThread(NULL, 0u, run, NULL, CREATE_SUSPENDED, &dispatcher);
    assert(thread != NULL);
    assert(ResumeThread(thread) == 1u);
#else
    assert(pthread_create(&thread, NULL_PTR, run, NULL_PTR) == 0);
    dispatcher = thread;
#endif
    while (paused == 0u) {
        wait();
    }
    unlock();
    contend();
    lock();
    released = 1u;
    wake();
    unlock();
#if defined(_WIN32)
    assert(WaitForSingleObject(thread, INFINITE) == WAIT_OBJECT_0);
    assert(CloseHandle(thread) != 0);
#else
    assert(pthread_join(thread, NULL_PTR) == 0);
#endif
}

/* Two inputs during the action: each arbitrates the port value taken just before it. */
static void indicate_while_acting(void)
{
    BswM_ComM_CurrentMode(7u, COMM_NO_COMMUNICATION);
    BswM_ComM_CurrentMode(7u, COMM_SILENT_COMMUNICATION);
    assert(action_count == 0u);
}
/* Deinit drops the postponed input; the action already taken still completes. */
static void deinitialize_while_acting(void)
{
    BswM_ComM_CurrentMode(7u, COMM_NO_COMMUNICATION);
    BswM_Deinit();
    BswM_ComM_CurrentMode(7u, COMM_SILENT_COMMUNICATION);
    assert(action_count == 0u);
}
/* A restart keeps the processor: the new lifetime's input waits for it. */
static void restart_while_acting(void)
{
    BswM_ComM_CurrentMode(7u, COMM_NO_COMMUNICATION);
    BswM_Init(&configuration);
    BswM_ComM_CurrentMode(7u, COMM_SILENT_COMMUNICATION);
    assert(action_count == 0u);
}

int main(void)
{
#if defined(_WIN32)
    InitializeCriticalSection(&handshake);
    InitializeConditionVariable(&changed);
#endif
    BswM_Init(&configuration);
    window(indicate_while_acting);
    assert(action_count == 3u && actions[0] == COMM_FULL_COMMUNICATION &&
           actions[1] == COMM_SILENT_COMMUNICATION && actions[2] == COMM_SILENT_COMMUNICATION);
    window(deinitialize_while_acting);
    assert(action_count == 1u && actions[0] == COMM_FULL_COMMUNICATION);
    BswM_ComM_CurrentMode(7u, COMM_NO_COMMUNICATION);
    assert(action_count == 1u);
    BswM_Init(&configuration);
    BswM_ComM_CurrentMode(7u, COMM_NO_COMMUNICATION); /* The processor has ended. */
    assert(action_count == 2u && actions[1] == COMM_NO_COMMUNICATION);
    window(restart_while_acting);
    assert(action_count == 2u && actions[0] == COMM_FULL_COMMUNICATION &&
           actions[1] == COMM_SILENT_COMMUNICATION);
#if defined(_WIN32)
    DeleteCriticalSection(&handshake);
#endif
    return 0;
}
