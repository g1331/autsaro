/** @file Exercise DLL detach stack use after real controller stack overflow. */
#include <windows.h>

#if defined(STACK_EXIT_DLL)
static void write_marker(const char *text, DWORD length)
{
    DWORD written;
    (void)WriteFile(GetStdHandle(STD_ERROR_HANDLE), text, length, &written, NULL);
}

__attribute__((noinline)) static void detach_stack_work(void)
{
    /* Bounded DLL cleanup fits a healthy stack and exceeds the 16 KiB overflow guarantee. */
    volatile unsigned char storage[32768];
    for (unsigned index = 0u; index < sizeof(storage); ++index) {
        storage[index] = (unsigned char)index;
    }
}

BOOL WINAPI DllMain(HINSTANCE instance, DWORD reason, LPVOID reserved)
{
    (void)instance;
    (void)reserved;
    if (reason == DLL_PROCESS_DETACH) {
        write_marker("STACK_EXIT_DETACH_BEGIN\n", sizeof("STACK_EXIT_DETACH_BEGIN\n") - 1u);
        detach_stack_work();
        write_marker("STACK_EXIT_DETACH_END\n", sizeof("STACK_EXIT_DETACH_END\n") - 1u);
    }
    return TRUE;
}
#else
#define main stack_scenario_main
#include "../../runtime/os/tests/native_stack.c"
#undef main

int main(int argc, char **argv)
{
    if (LoadLibraryA("stack-exit.dll") == NULL) {
        return 90;
    }
    if ((argc == 2) && (strcmp(argv[1], "healthy-detach") == 0)) {
        ExitProcess(E_OS_STACKFAULT);
    }
    return stack_scenario_main(argc, argv);
}
#endif
