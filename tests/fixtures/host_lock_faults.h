#ifndef HOST_LOCK_FAULTS_H
#define HOST_LOCK_FAULTS_H
#ifdef _WIN32
#include <windows.h>
void Test_Trap(void) __attribute__((noreturn));
#define __builtin_trap Test_Trap
BOOL WINAPI Test_Terminate(HANDLE, UINT);
#define TerminateProcess Test_Terminate
BOOL WINAPI Test_Once(PINIT_ONCE, PINIT_ONCE_FN, PVOID, LPVOID *);
BOOL WINAPI Test_Init(LPCRITICAL_SECTION, DWORD, DWORD);
void WINAPI Test_Leave(LPCRITICAL_SECTION);
#define LeaveCriticalSection        Test_Leave
#define InitOnceExecuteOnce         Test_Once
#define InitializeCriticalSectionEx Test_Init
#else
#include <pthread.h>
#include <unistd.h>
void Test_Exit(int) __attribute__((noreturn));
#define _exit                Test_Exit
int Test_Once(pthread_once_t *, void (*)(void));
int Test_Init(pthread_mutex_t *, const pthread_mutexattr_t *);
int Test_Lock(pthread_mutex_t *);
int Test_Unlock(pthread_mutex_t *);
#define pthread_once         Test_Once
#define pthread_mutex_init   Test_Init
#define pthread_mutex_lock   Test_Lock
#define pthread_mutex_unlock Test_Unlock
#endif
#endif
