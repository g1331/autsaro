#ifndef HOST_LOCK_FAULTS_H
#define HOST_LOCK_FAULTS_H
#include <stdlib.h>
void Test_Abort(void) __attribute__((noreturn));
#define abort Test_Abort
#ifdef _WIN32
#include <windows.h>
BOOL WINAPI Test_Once(PINIT_ONCE, PINIT_ONCE_FN, PVOID, LPVOID *);
BOOL WINAPI Test_Init(LPCRITICAL_SECTION, DWORD, DWORD);
BOOL WINAPI Test_Try(LPCRITICAL_SECTION);
#define InitOnceExecuteOnce Test_Once
#define InitializeCriticalSectionEx Test_Init
#define TryEnterCriticalSection Test_Try
#else
#include <pthread.h>
int Test_Once(pthread_once_t *, void (*)(void));
int Test_AttrInit(pthread_mutexattr_t *);
int Test_AttrType(pthread_mutexattr_t *, int);
int Test_AttrClose(pthread_mutexattr_t *);
int Test_Init(pthread_mutex_t *, const pthread_mutexattr_t *);
int Test_Close(pthread_mutex_t *);
int Test_Lock(pthread_mutex_t *);
int Test_Try(pthread_mutex_t *);
int Test_Unlock(pthread_mutex_t *);
#define pthread_once Test_Once
#define pthread_mutexattr_init Test_AttrInit
#define pthread_mutexattr_settype Test_AttrType
#define pthread_mutexattr_destroy Test_AttrClose
#define pthread_mutex_init Test_Init
#define pthread_mutex_destroy Test_Close
#define pthread_mutex_lock Test_Lock
#define pthread_mutex_trylock Test_Try
#define pthread_mutex_unlock Test_Unlock
#endif
#endif
