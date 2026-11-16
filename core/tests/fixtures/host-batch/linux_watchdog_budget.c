/** @file Bounded H/S CPU and handoff accounting for the timing-test ELF only. */
#include "Os_Host.h"
#include "Os_Backend.h"
#include "Os_Target.h"
#include "Os_Stack.h"
#include "Ecu_Target.h"
#include <stdint.h>
#include <time.h>

/* Each role has one writer. GDB reads only after the whole thread group stops.
 * Exclude only recorder accesses from TSan's measured work; every real callee remains
 * instrumented. ASan/UBSan still check this bounded test recorder. */
#define BUDGET_RECORD __attribute__((no_sanitize_thread))
enum { BUDGET_PHASES = 5, BUDGET_SAMPLES = 512 };
enum { BUDGET_WAIT, BUDGET_SUSPEND, BUDGET_RESUME, BUDGET_DRAIN, BUDGET_POST };
typedef struct {
    uint64_t calls;
    uint64_t completed;
    uint64_t exclusive_cpu_ns;
    uint64_t cpu_ns;
    uint64_t wall_ns;
} BudgetPhase;
typedef struct {
    uint64_t sequence;
    uint64_t wall_ns;
    uint64_t cpu_ns;
    uint64_t calls[BUDGET_PHASES];
    uint64_t outside_cpu_ns;
    uint64_t phase_exclusive_cpu_ns[BUDGET_PHASES];
    uint64_t phase_cpu_ns[BUDGET_PHASES];
    uint64_t phase_wall_ns[BUDGET_PHASES];
    unsigned phase;
    unsigned entering;
} BudgetSample;
typedef struct {
    unsigned tid;
    unsigned active;
    uint64_t revision;
    uint64_t entered_wall_ns;
    uint64_t entered_cpu_ns;
    uint64_t accounted_cpu_ns;
    uint64_t outside_cpu_ns;
    uint64_t next_sample_ns;
    uint64_t sequence;
    unsigned clock_errors;
    BudgetPhase phases[BUDGET_PHASES];
    BudgetSample samples[BUDGET_SAMPLES];
} BudgetThread;
/* Volatile retains debugger-only stores; each role still has exactly one writer. */
static volatile BudgetThread capture_budget[2];

static BUDGET_RECORD uint64_t budget_clock(clockid_t clock, volatile BudgetThread *thread)
{
    struct timespec now;
    if (clock_gettime(clock, &now) != 0) {
        ++thread->clock_errors;
        return 0u;
    }
    return ((uint64_t)now.tv_sec * UINT64_C(1000000000)) + (uint64_t)now.tv_nsec;
}

static BUDGET_RECORD volatile BudgetThread *budget_thread(void)
{
    const Os_NativeStackType *stack = Os_StackCurrent();
    volatile BudgetThread *thread = NULL;
    if ((stack != NULL) && ((stack->role == 'H') || (stack->role == 'S'))) {
        thread = &capture_budget[(stack->role == 'H') ? 0 : 1];
        if (thread->tid == 0u) {
            thread->tid = GetCurrentThreadId();
        }
    }
    return thread;
}

static BUDGET_RECORD void budget_sample(
    volatile BudgetThread *thread, uint64_t wall, uint64_t cpu, unsigned phase, unsigned entering)
{
    if (wall >= thread->next_sample_ns) {
        volatile BudgetSample *sample = &thread->samples[thread->sequence % BUDGET_SAMPLES];
        sample->sequence = 0u;
        sample->wall_ns = wall;
        sample->cpu_ns = cpu;
        sample->outside_cpu_ns = thread->outside_cpu_ns;
        sample->phase = phase;
        sample->entering = entering;
        for (unsigned index = 0u; index < BUDGET_PHASES; ++index) {
            sample->calls[index] = thread->phases[index].calls;
            sample->phase_exclusive_cpu_ns[index] = thread->phases[index].exclusive_cpu_ns;
            sample->phase_cpu_ns[index] = thread->phases[index].cpu_ns;
            sample->phase_wall_ns[index] = thread->phases[index].wall_ns;
        }
        sample->sequence = thread->sequence + 1u;
        ++thread->sequence;
        thread->next_sample_ns = wall + UINT64_C(20000000);
    }
}

static BUDGET_RECORD void budget_account(volatile BudgetThread *thread, uint64_t cpu)
{
    const uint64_t delta = cpu - thread->accounted_cpu_ns;
    if (thread->active != 0u) {
        thread->phases[thread->active - 1u].exclusive_cpu_ns += delta;
    } else {
        thread->outside_cpu_ns += delta;
    }
    thread->accounted_cpu_ns = cpu;
}

typedef struct {
    volatile BudgetThread *thread;
    uint64_t wall_ns;
    uint64_t cpu_ns;
    unsigned previous_active;
    uint64_t previous_wall_ns;
    uint64_t previous_cpu_ns;
} BudgetSpan;

static BUDGET_RECORD BudgetSpan budget_enter(unsigned phase)
{
    BudgetSpan span = {0};
    span.thread = budget_thread();
    if (span.thread != NULL) {
        volatile BudgetThread *thread = span.thread;
        ++thread->revision;
        span.wall_ns = budget_clock(CLOCK_MONOTONIC, thread);
        span.cpu_ns = budget_clock(CLOCK_THREAD_CPUTIME_ID, thread);
        budget_account(thread, span.cpu_ns);
        span.previous_active = thread->active;
        span.previous_wall_ns = thread->entered_wall_ns;
        span.previous_cpu_ns = thread->entered_cpu_ns;
        thread->entered_wall_ns = span.wall_ns;
        thread->entered_cpu_ns = span.cpu_ns;
        thread->active = phase + 1u;
        ++thread->phases[phase].calls;
        budget_sample(thread, span.wall_ns, span.cpu_ns, phase, 1u);
        ++thread->revision;
    }
    return span;
}

static BUDGET_RECORD void budget_leave(BudgetSpan span, unsigned phase)
{
    if (span.thread != NULL) {
        volatile BudgetThread *thread = span.thread;
        ++thread->revision;
        const uint64_t cpu = budget_clock(CLOCK_THREAD_CPUTIME_ID, thread);
        const uint64_t wall = budget_clock(CLOCK_MONOTONIC, thread);
        budget_account(thread, cpu);
        /* Inclusive API totals may overlap when a host call nests another boundary. */
        thread->phases[phase].cpu_ns += cpu - span.cpu_ns;
        thread->phases[phase].wall_ns += wall - span.wall_ns;
        ++thread->phases[phase].completed;
        thread->active = span.previous_active;
        thread->entered_wall_ns = span.previous_wall_ns;
        thread->entered_cpu_ns = span.previous_cpu_ns;
        budget_sample(thread, wall, cpu, phase, 0u);
        ++thread->revision;
    }
}

DWORD __real_WaitForMultipleObjects(DWORD count, const HANDLE *handles, BOOL all, DWORD timeout);
BUDGET_RECORD DWORD __wrap_WaitForMultipleObjects(DWORD count,
                                                  const HANDLE *handles,
                                                  BOOL all,
                                                  DWORD timeout);
BUDGET_RECORD DWORD __wrap_WaitForMultipleObjects(DWORD count,
                                                  const HANDLE *handles,
                                                  BOOL all,
                                                  DWORD timeout)
{
    BudgetSpan span = budget_enter(BUDGET_WAIT);
    const DWORD result = __real_WaitForMultipleObjects(count, handles, all, timeout);
    budget_leave(span, BUDGET_WAIT);
    return result;
}

DWORD __real_SuspendThread(HANDLE actor);
BUDGET_RECORD DWORD __wrap_SuspendThread(HANDLE actor);
BUDGET_RECORD DWORD __wrap_SuspendThread(HANDLE actor)
{
    BudgetSpan span = budget_enter(BUDGET_SUSPEND);
    const DWORD result = __real_SuspendThread(actor);
    budget_leave(span, BUDGET_SUSPEND);
    return result;
}

DWORD __real_ResumeThread(HANDLE actor);
BUDGET_RECORD DWORD __wrap_ResumeThread(HANDLE actor);
BUDGET_RECORD DWORD __wrap_ResumeThread(HANDLE actor)
{
    BudgetSpan span = budget_enter(BUDGET_RESUME);
    const DWORD result = __real_ResumeThread(actor);
    budget_leave(span, BUDGET_RESUME);
    return result;
}

StatusType __real_Ecu_TargetTakeOutput(Ecu_OutputRecord *output);
BUDGET_RECORD StatusType __wrap_Ecu_TargetTakeOutput(Ecu_OutputRecord *output);
BUDGET_RECORD StatusType __wrap_Ecu_TargetTakeOutput(Ecu_OutputRecord *output)
{
    BudgetSpan span = budget_enter(BUDGET_DRAIN);
#if defined(AUTOSAR_CAPTURE_INJECT)
    /* Exercise an internal BackendAssert -> Shutdown -> RequestShutdown call. */
    if ((span.thread != NULL) && (span.thread->phases[BUDGET_DRAIN].calls == 100u)) {
        Os_BackendAssert(__FILE__, __LINE__);
    }
#endif
    const StatusType result = __real_Ecu_TargetTakeOutput(output);
    budget_leave(span, BUDGET_DRAIN);
    return result;
}

StatusType __real_Os_TargetAdvanceOneTick(uint64 epoch, uint64 *ticket);
BUDGET_RECORD StatusType __wrap_Os_TargetAdvanceOneTick(uint64 epoch, uint64 *ticket);
BUDGET_RECORD StatusType __wrap_Os_TargetAdvanceOneTick(uint64 epoch, uint64 *ticket)
{
    BudgetSpan span = budget_enter(BUDGET_POST);
    const StatusType result = __real_Os_TargetAdvanceOneTick(epoch, ticket);
    budget_leave(span, BUDGET_POST);
    return result;
}
