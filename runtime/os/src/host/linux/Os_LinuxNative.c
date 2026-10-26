/** @file Typed x86-64 Linux syscalls, signal snapshots and one-time stack entry. */
#include "Os_LinuxNative.h"
#include <asm/sigcontext.h>
#include <asm/ucontext.h>
#include <asm/unistd.h>
#include <errno.h>
#include <fcntl.h>
#include <linux/futex.h>
#include <stddef.h>
#include <string.h>
#include <unistd.h>

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static long native_call(
    long number, const volatile void *first, long second, long third, const void *fourth)
{
    long result;
    __asm__ volatile("mov %5,%%r10\n\tsyscall"
                     : "=a"(result)
                     : "a"(number), "D"(first), "S"(second), "d"(third), "r"(fourth)
                     : "rcx", "r11", "r10", "memory");
    return result;
}

OS_MEMMAP int Os_LinuxFutexWait(const volatile int32_t *cell,
                                int32_t expected,
                                const struct timespec *interval)
{
    return (int)native_call(__NR_futex, cell, FUTEX_WAIT_PRIVATE, expected, interval);
}

OS_MEMMAP void Os_LinuxFutexWake(const volatile int32_t *cell)
{
    const long result = native_call(__NR_futex, cell, FUTEX_WAKE_PRIVATE, INT32_MAX, NULL);
    if (result < 0) {
        ExitProcess(1u);
    }
}

OS_MEMMAP DWORD Os_LinuxThreadId(void)
{
    const long result = native_call(__NR_gettid, NULL, 0, 0, NULL);
    return ((result > 0) && ((uint64_t)result <= UINT32_MAX)) ? (DWORD)result : 0u;
}

OS_MEMMAP int Os_LinuxAltStack(const stack_t *stack, stack_t *previous)
{
    long result;
    __asm__ volatile("syscall"
                     : "=a"(result)
                     : "a"(__NR_sigaltstack), "D"(stack), "S"(previous)
                     : "rcx", "r11", "memory");
    return (int)result;
}

OS_MEMMAP void Os_LinuxCaptureContext(CONTEXT *destination, const void *source)
{
    /* C99 compile-time ABI constraints: incompatible platform layouts fail compilation. */
    (void)sizeof(
        char[(offsetof(struct ucontext, uc_mcontext) == offsetof(ucontext_t, uc_mcontext)) ? 1
                                                                                           : -1]);
    (void)sizeof(
        char[(sizeof(struct _fpstate) == sizeof(destination->native.__fpregs_mem)) ? 1 : -1]);
    (void)sizeof(char[(sizeof(struct sigcontext) == sizeof(mcontext_t)) ? 1 : -1]);
    (void)sizeof(char[((offsetof(struct ucontext, uc_sigmask) == 296u) &&
                       (offsetof(struct ucontext, uc_mcontext.rsp) == 160u) &&
                       (offsetof(struct ucontext, uc_mcontext.fpstate) == 224u))
                          ? 1
                          : -1]);
    (void)sizeof(
        char[(offsetof(struct ucontext, uc_sigmask) == offsetof(ucontext_t, uc_sigmask)) ? 1 : -1]);
    (void)sizeof(char[(offsetof(struct ucontext, uc_mcontext.fpstate) ==
                       (offsetof(ucontext_t, uc_mcontext) + sizeof(gregset_t)))
                          ? 1
                          : -1]);
    (void)sizeof(char[((offsetof(struct ucontext, uc_sigmask) + sizeof(uint64_t)) <=
                       offsetof(ucontext_t, __fpregs_mem))
                          ? 1
                          : -1]);
    /* Linux x64 supplies a 64-bit mask, not glibc's full sigset/FP/SSP tail. */
    const size_t prefix = offsetof(struct ucontext, uc_sigmask) + sizeof(uint64_t);
    struct ucontext kernel;
    ucontext_t snapshot = {0};
    unsigned char *native = (unsigned char *)&snapshot;
    (void)memcpy((void *)&kernel, source, prefix);
    (void)memcpy((void *)&snapshot, (const void *)&kernel, prefix);
    if (kernel.uc_mcontext.fpstate != NULL) {
        void *copied = &destination->native.__fpregs_mem;
        (void)memmove((unsigned char *)&snapshot.__fpregs_mem,
                      (const unsigned char *)kernel.uc_mcontext.fpstate,
                      sizeof(snapshot.__fpregs_mem));
        (void)memcpy(&native[offsetof(struct ucontext, uc_mcontext.fpstate)],
                     (const unsigned char *)&copied,
                     sizeof(copied));
    }
    destination->native = snapshot;
    destination->Rsp = kernel.uc_mcontext.rsp;
    destination->ContextFlags = CONTEXT_CONTROL;
}

OS_MEMMAP uintptr_t Os_LinuxSignalSp(const void *context)
{
    struct ucontext kernel;
    (void)memcpy((void *)&kernel,
                 context,
                 offsetof(struct ucontext, uc_mcontext.rsp) + sizeof(kernel.uc_mcontext.rsp));
    return kernel.uc_mcontext.rsp;
}

OS_MEMMAP static int map_address(const char **cursor, uintptr_t *value)
{
    uintptr_t result = 0u;
    size_t count = 0u;
    int valid = 1;
    const char *text = *cursor;
    while (((*text >= '0') && (*text <= '9')) || ((*text >= 'a') && (*text <= 'f'))) {
        const unsigned digit =
            (*text <= '9') ? (unsigned)(*text - '0') : (unsigned)(*text - 'a') + 10u;
        if (count >= sizeof(result) * 2u) {
            valid = 0;
            break;
        }
        result = (result * 16u) + digit;
        ++count;
        ++text;
    }
    *cursor = text;
    *value = result;
    return ((valid != 0) && (count != 0u)) ? 1 : 0;
}

OS_MEMMAP static int mapped_line(const char *line,
                                 uintptr_t sp,
                                 uintptr_t *previous_low,
                                 uintptr_t *previous_high,
                                 uintptr_t *low,
                                 uintptr_t *high,
                                 size_t *guard)
{
    uintptr_t first = 0u;
    uintptr_t last = 0u;
    const char *cursor = line;
    int result = 0;
    int valid = map_address(&cursor, &first);
    if ((valid != 0) && (*cursor == '-')) {
        ++cursor;
        valid = map_address(&cursor, &last);
    } else {
        valid = 0;
    }
    if ((valid != 0) && (*cursor == ' ') && (first < last)) {
        ++cursor;
        valid = (strlen(cursor) >= 4u) ? 1 : 0;
    } else {
        valid = 0;
    }
    if ((valid != 0) && (cursor[0] == 'r') && (cursor[1] == 'w') && (first <= sp) && (sp < last)) {
        *low = (*previous_high == first) ? *previous_low : first;
        *high = last;
        *guard = first - *low;
        result = 1;
    } else if ((valid != 0) && (cursor[0] == '-') && (cursor[1] == '-') && (cursor[2] == '-')) {
        *previous_low = first;
        *previous_high = last;
    } else {
        *previous_low = 0u;
        *previous_high = 0u;
    }
    return result;
}

OS_MEMMAP int Os_LinuxMappedStack(uintptr_t *low, uintptr_t *high, size_t *guard)
{
    int result = 0;
    int descriptor;
    errno = 0;
    descriptor = open("/proc/self/maps", O_RDONLY);
    if (descriptor >= 0) {
        uintptr_t sp;
        OS_HOST_STACK_POINTER(sp);
        char data[1024];
        char line[64];
        size_t used = 0u;
        uintptr_t previous_low = 0u;
        uintptr_t previous_high = 0u;
        ssize_t count;
        do {
            errno = 0;
            count = read(descriptor, data, sizeof(data));
            if (count > 0) {
                size_t index = 0u;
                while ((index < (size_t)count) && (result == 0)) {
                    if (data[index] == '\n') {
                        line[used] = '\0';
                        result =
                            mapped_line(line, sp, &previous_low, &previous_high, low, high, guard);
                        used = 0u;
                    } else if (used < sizeof(line) - 1u) {
                        line[used] = data[index];
                        ++used;
                    } else {
                        /* Only the bounded address/permission prefix is needed. */
                    }
                    ++index;
                }
            }
        } while ((result == 0) && ((count > 0) || ((count < 0) && (errno == EINTR))));
        errno = 0;
        if (close(descriptor) != 0) {
            result = 0;
        }
    }
    return result;
}

/* One call/return pair crosses onto the owned mmap stack; its CFA remains on
 * the pthread bootstrap until return. Activations unwind only above this frame. */
__asm__(".pushsection .os_code,\"ax\",@progbits\n.global Os_LinuxCallOnStack\n"
        ".type Os_LinuxCallOnStack,@function\nOs_LinuxCallOnStack:\n.cfi_startproc\n"
        "endbr64\npush %r12\n.cfi_def_cfa_offset 16\n.cfi_offset 12,-16\n"
        "mov %rsp,%r12\n.cfi_def_cfa r12,16\nmov %rdi,%rsp\nand $-16,%rsp\n"
        "mov %rdx,%rdi\ncall *%rsi\nmov %r12,%rsp\n.cfi_def_cfa rsp,16\n"
        "pop %r12\n.cfi_def_cfa_offset 8\nret\n.cfi_endproc\n"
        ".size Os_LinuxCallOnStack,.-Os_LinuxCallOnStack\n.popsection\n");
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
