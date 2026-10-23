/** @file Native port re-entry, hardware shadow-stack and detector cleanup consumer. */
#include "Os_PortRestart.h"
#include "Os_Backend.h"
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#ifndef _WIN32
#include <unistd.h>
#include <pthread.h>
#include <unwind.h>
#ifdef __SANITIZE_ADDRESS__
#include <sanitizer/asan_interface.h>
#endif
extern void corrupt_return(void);
__asm__(
    ".text\n.global corrupt_return\n.type corrupt_return,@function\ncorrupt_return:\nendbr64\nlea "
    "1f(%rip),%rax\nmov %rax,(%rsp)\nret\n1:\nmov $99,%edi\ncall exit\nud2\n");
#endif
void Os_BackendShutdown(StatusType error)
{
    (void)error;
#ifdef _WIN32
    TerminateProcess(GetCurrentProcess(), 92);
    __builtin_trap();
#else
    _exit(92);
#endif
}
static unsigned count;        /**< Probe entry count; single actor. */
static int shadow_required;   /**< Main requests hardware enforcement in CET vectors. */
static int foreign;           /**< Main selects an unrelated native exception before entry. */
static uintptr_t initial_ssp; /**< First hardware shadow-stack pointer, including disabled zero. */
static uintptr_t initial;     /**< First physical stack observation; single actor. */
#ifdef __SANITIZE_ADDRESS__
static char *abandoned; /**< Deep abandoned redzone; inspected after native unwinding. */
#endif
#ifdef __SANITIZE_THREAD__
static pthread_t observer; /**< Joined after the final observed access. */
static int shared;         /**< Deliberately raced only in the explicit fault vector. */
static int gate, done;     /**< Atomic observer handshake. */
static int race;           /**< Immutable test selection before thread creation. */
static void *observe(void *argument)
{
    (void)argument;
    while (__atomic_load_n(&gate, __ATOMIC_ACQUIRE) == 0) {
    }
    volatile int value = shared;
    (void)value;
    __atomic_store_n(&done, 1, __ATOMIC_RELEASE);
    return NULL;
}
#endif
__attribute__((noinline)) void nested(unsigned depth)
{
    volatile unsigned scratch[32];
    scratch[0] = depth;
#ifdef __SANITIZE_ADDRESS__
    if (depth == 0) {
        abandoned = (char *)&scratch[32];
        if (!__asan_address_is_poisoned(abandoned)) {
            exit(87);
        }
    }
#endif
    if (depth == 100) {
        return;
    }
    if (depth) {
        nested(depth - 1);
    } else {
        Os_PortRestart();
    }
    scratch[0]++;
}
__attribute__((noinline)) static void entry(void *argument)
{
    if (argument != &initial) {
        exit(88);
    }
#ifndef _WIN32
    if (foreign && count == 0u) {
        struct _Unwind_Exception exception = {.exception_class = UINT64_C(0x4155544f53525431)};
        ++count;
        (void)_Unwind_RaiseException(&exception);
        exit(94);
    }
#else
    if (foreign) {
        RaiseException(0xE0426002u, EXCEPTION_NONCONTINUABLE, 0, NULL);
        exit(94);
    }
#endif
    uintptr_t sp;
    uintptr_t ssp = 0u;
    __asm__ volatile("mov %%rsp,%0" : "=r"(sp));
    __asm__ volatile("rdsspq %0" : "+r"(ssp));
    if (!count) {
        initial = sp;
        initial_ssp = ssp;
    }
    if (((shadow_required != 0) && (ssp == 0u)) || (initial != sp) || (initial_ssp != ssp)) {
        fprintf(stderr,
                "stack initial=%lx actual=%lx count=%u\n",
                (unsigned long)initial,
                (unsigned long)sp,
                count);
        abort();
    }
    ++count;
#ifdef __SANITIZE_THREAD__
    const unsigned limit = race ? 20u : 10000u;
#else
    const unsigned limit = 10000u;
#endif
    if (count == limit) {
#ifdef __SANITIZE_ADDRESS__
        if (__asan_address_is_poisoned(abandoned)) {
            exit(86);
        }
#endif
#ifdef __SANITIZE_THREAD__
        shared = 42;
        if (race) {
            __atomic_store_n(&gate, 1, __ATOMIC_RELAXED);
        } else {
            __atomic_store_n(&gate, 1, __ATOMIC_RELEASE);
        }
        while (__atomic_load_n(&done, __ATOMIC_ACQUIRE) == 0) {
        }
        if (pthread_join(observer, NULL) != 0) {
            exit(85);
        }
#endif
        printf("restart iterations=%u stack=stable shadow_stack=stable ssp=%llu\n",
               count,
               (unsigned long long)initial_ssp);
        exit(0);
    }
    nested(8u + count % 31u);
}
int main(int argc, char **argv)
{
#ifdef _WIN32
    SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
#endif
    if (argc > 1 && argv[1][0] == 'u') {
        Os_PortRestart();
    }
    foreign = argc > 1 && argv[1][0] == 'f';
#ifndef _WIN32
    if (argc > 1 && (argv[1][0] == 'c' || argv[1][0] == 'b')) {
        long result;
        unsigned long status = 0;
        __asm__ volatile("syscall"
                         : "=a"(result)
                         : "a"(158L), "D"(0x5001L), "S"(1L)
                         : "rcx", "r11", "memory");
        printf("CET enable=%ld\n", result);
        fflush(stdout);
        if (result) {
            return 77;
        }
        __asm__ volatile("syscall"
                         : "=a"(result)
                         : "a"(158L), "D"(0x5005L), "S"(&status)
                         : "rcx", "r11", "memory");
        printf("CET status=%lu result=%ld\n", status, result);
        fflush(stdout);
        if (result || !(status & 1)) {
            return 78;
        }
        shadow_required = 1;
        if (argv[1][0] == 'b') {
            corrupt_return();
            return 79;
        }
    }
#else
    (void)argc;
    (void)argv;

#endif
#ifdef __SANITIZE_THREAD__
    race = argc > 1 && argv[1][0] == 'r';
    if (pthread_create(&observer, NULL, observe, NULL) != 0) {
        return 84;
    }
#endif
    Os_PortTaskLoop(entry, &initial);
}
