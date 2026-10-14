/* Temporary #61 hardware probe: thread handoff and suspend costs on this runner. */
#define _WIN32_WINNT 0x0A00
#include <windows.h>
#include <stdio.h>
#include <stdlib.h>

#define ROUNDS 4000

static HANDLE ping, pong, gate, reply;
static volatile LONG stop;
static double frequency;

static double now_us(void)
{
    LARGE_INTEGER counter;
    QueryPerformanceCounter(&counter);
    return (double)counter.QuadPart * 1e6 / frequency;
}
static int compare(const void *a, const void *b)
{
    const double x = *(const double *)a, y = *(const double *)b;
    return (x > y) - (x < y);
}
static void report(const char *label, double *samples)
{
    qsort(samples, ROUNDS, sizeof(samples[0]), compare);
    printf("%-44s p50=%9.1f p90=%9.1f p99=%9.1f max=%9.1f us\n", label,
           samples[ROUNDS / 2], samples[ROUNDS * 9 / 10], samples[ROUNDS * 99 / 100],
           samples[ROUNDS - 1]);
}
static DWORD WINAPI echo(LPVOID unused)
{
    (void)unused;
    for (;;) {
        WaitForSingleObject(ping, INFINITE);
        if (stop) {
            return 0;
        }
        SetEvent(pong);
    }
}
static DWORD WINAPI spin(LPVOID unused)
{
    volatile unsigned long long count = 0;
    (void)unused;
    while (!stop) {
        ++count;
    }
    return 0;
}
static DWORD WINAPI spin_query(LPVOID unused)
{
    MEMORY_BASIC_INFORMATION region;
    ULONG_PTR low, high;
    (void)unused;
    while (!stop) {
        GetCurrentThreadStackLimits(&low, &high);
        VirtualQuery(&region, &region, sizeof(region));
    }
    return 0;
}
static DWORD WINAPI sleeper(LPVOID unused)
{
    (void)unused;
    while (!stop) {
        Sleep(1);
    }
    return 0;
}
/* FreeRTOS-port style task: waits on its own event, is suspended there, then resumed. */
static DWORD WINAPI task(LPVOID unused)
{
    (void)unused;
    for (;;) {
        WaitForSingleObject(gate, INFINITE);
        if (stop) {
            return 0;
        }
        SetEvent(reply);
    }
}
static void pingpong(const char *label)
{
    static double samples[ROUNDS];
    for (int i = 0; i < ROUNDS; ++i) {
        const double started = now_us();
        SetEvent(ping);
        WaitForSingleObject(pong, INFINITE);
        samples[i] = now_us() - started;
    }
    report(label, samples);
}
static void suspend_cycle(const char *label, HANDLE thread)
{
    static double samples[ROUNDS];
    CONTEXT context;
    for (int i = 0; i < ROUNDS; ++i) {
        const double started = now_us();
        SuspendThread(thread);
        context.ContextFlags = CONTEXT_INTEGER;
        GetThreadContext(thread, &context);
        ResumeThread(thread);
        samples[i] = now_us() - started;
    }
    report(label, samples);
}
static void resume_handoff(const char *label, HANDLE thread)
{
    static double samples[ROUNDS];
    CONTEXT context;
    for (int i = 0; i < ROUNDS; ++i) {
        double started;
        Sleep(0);
        SuspendThread(thread);
        context.ContextFlags = CONTEXT_INTEGER;
        GetThreadContext(thread, &context);
        started = now_us();
        SetEvent(gate);
        ResumeThread(thread);
        WaitForSingleObject(reply, INFINITE);
        samples[i] = now_us() - started;
    }
    report(label, samples);
}
static HANDLE start(LPTHREAD_START_ROUTINE entry)
{
    return CreateThread(NULL, 0, entry, NULL, 0, NULL);
}
static void finish(HANDLE thread)
{
    InterlockedExchange(&stop, 1);
    WaitForSingleObject(thread, INFINITE);
    CloseHandle(thread);
    InterlockedExchange(&stop, 0);
}
int main(void)
{
    LARGE_INTEGER f;
    HANDLE echo_thread, task_thread, other;
    QueryPerformanceFrequency(&f);
    frequency = (double)f.QuadPart;
    ping = CreateEventA(NULL, FALSE, FALSE, NULL);
    pong = CreateEventA(NULL, FALSE, FALSE, NULL);
    gate = CreateEventA(NULL, FALSE, FALSE, NULL);
    reply = CreateEventA(NULL, FALSE, FALSE, NULL);
    echo_thread = start(echo);
    task_thread = start(task);
    pingpong("event round trip, quiet");
    resume_handoff("suspend+resume handoff, quiet", task_thread);
    other = start(spin);
    Sleep(50);
    pingpong("event round trip, one spinner");
    resume_handoff("suspend+resume handoff, one spinner", task_thread);
    suspend_cycle("suspend/context/resume of spinner", other);
    finish(other);
    other = start(spin_query);
    Sleep(50);
    pingpong("event round trip, VirtualQuery spinner");
    resume_handoff("suspend+resume handoff, VirtualQuery spinner", task_thread);
    suspend_cycle("suspend/context/resume of VirtualQuery spinner", other);
    finish(other);
    other = start(sleeper);
    Sleep(50);
    pingpong("event round trip, Sleep(1) loop");
    suspend_cycle("suspend/context/resume of Sleep(1) loop", other);
    finish(other);
    return 0;
}
