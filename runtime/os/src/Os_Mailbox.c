/** @file Native input mailbox: single-producer admission and owner-Task notification. */
#include "Os_Backend.h"
#include <string.h>

#define OS_START_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
OS_MEMMAP static Os_InputRecordType
    records[OS_INPUT_CAPACITY]; /**< SPSC-owned ring slots, atomic publication; no Init reset. */

/* MR12 RULE 8.9 VIOLATION: Explicit VAR mapping requires file-static scope under SWS_MemMap_00023.
 */
OS_MEMMAP static uint64 last_ticket; /**< Single-producer accepted ticket; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_64
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_Mailbox.c -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_Mailbox.c -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
OS_MEMMAP static volatile Os_Atomic32Type
    published; /**< Atomic complete-record count; no Init reset. */
/* Acceptance bit0 and permanent-close bit1; close never waits for the producer. */
OS_MEMMAP static volatile Os_Atomic32Type
    admission; /**< Atomic producer/close bits; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"

#if defined(OS_EVENT_TESTS) && (OS_EVENT_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_input_notification(void) { Os_TestInputNotified(); }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static void observe_input_notification(void) {}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP static uint32 input_interrupt(void)
{
    if (Os_TargetReady() != 0) {
        Os_ArtiInternalEnter();
        const StatusType status = SetEvent(Os_Config->input_task, Os_Config->input_event);
        Os_ArtiInternalLeave();
        if (status != E_OK) {
            Os_BackendShutdown(E_OS_STATE);
        }
        observe_input_notification();
    }
    return 0u;
}

OS_MEMMAP void Os_MailboxInstall(void)
{
    if (Os_Config->input_event != 0u) {
        vPortSetInterruptHandler(OS_INPUT_INTERRUPT, &input_interrupt);
    }
}

OS_MEMMAP sint32 Os_MailboxHandlerAllowed(uint32 interrupt, uint32 (*handler)(void))
{
    return ((Os_BackendHandlerAllowed(interrupt, handler) != 0) &&
            ((Os_Config == NULL) || (Os_Config->input_event == 0u) ||
             (interrupt != OS_INPUT_INTERRUPT) || (handler == &input_interrupt)))
               ? 1
               : 0;
}

OS_MEMMAP void Os_MailboxClose(void) { (void)InterlockedOr(&admission, 2); }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#if defined(OS_EVENT_TESTS) && (OS_EVENT_TESTS == STD_ON)
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP void Os_TestInputLastTicket(uint64 ticket) { last_ticket = ticket; }
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
/* MR12 RULE 8.9 VIOLATION: Explicit VAR mapping requires file-static scope under SWS_MemMap_00023.
 */
OS_MEMMAP static uint32 write_index; /**< Persistent function-owned state; existing caller/port
                                        synchronization; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP StatusType Os_TargetPostInput(const uint8 *data, uint8 length, uint64 *ticket)
{
    StatusType context;
    Os_InputRecordType *record;
    if (Os_BridgeActorAllowed() == 0) {
        return E_OS_CALLEVEL;
    }
    if ((data == NULL) || (ticket == NULL)) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    if ((length == 0u) || (length > OS_INPUT_PAYLOAD)) {
        return E_OS_VALUE;
    }
    if ((Os_TargetReady() == 0) || (Os_Config->input_event == 0u)) {
        return E_OS_STATE;
    }
    context = Os_BridgeContext();
    if (context != E_OK) {
        return context;
    }
    if ((InterlockedCompareExchange(&published, 0, 0) >= (LONG)OS_INPUT_CAPACITY) ||
        (last_ticket == UINT64_MAX)) {
        return E_OS_LIMIT;
    }
    if (InterlockedCompareExchange(&admission, 1, 0) != 0) {
        return E_OS_STATE;
    }
    record = &records[write_index];
    record->ticket = last_ticket + UINT64_C(1);
    record->length = length;
    (void)memset(record->data, 0, sizeof(record->data));
    (void)memcpy(record->data, data, length);
    last_ticket = record->ticket;
    *ticket = last_ticket;
    write_index = (write_index + 1u) % OS_INPUT_CAPACITY;
    /* Interlocked publication orders the complete copy before consumer reads. */
    (void)InterlockedIncrement(&published);
    (void)InterlockedAnd(&admission, (LONG) ~(uint32)1u);
    if (Os_TargetReady() != 0) {
        Os_PortPostInterrupt(OS_INPUT_INTERRUPT);
    }
    return E_OK;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#ifdef __linux__
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP sint32 Os_BridgeActorAllowed(void)
{
    const Os_NativeStackType *actor = Os_StackCurrent();
    return ((actor == NULL) || (actor->role == 'H')) ? 1 : 0;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#else
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP sint32 Os_BridgeActorAllowed(void)
{
    const Os_NativeStackType *actor = Os_StackCurrent();
    return (actor == NULL) ? 1 : 0;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#endif
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
/* MR12 RULE 8.9 VIOLATION: Explicit VAR mapping requires file-static scope under SWS_MemMap_00023.
 */
OS_MEMMAP static volatile Os_Atomic32Type
    producer; /**< Persistent function-owned state; existing caller/port synchronization; no Init
                 reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP StatusType Os_BridgeContext(void)
{
    LONG thread;
    LONG owner;
    if (Os_BridgeActorAllowed() == 0) {
        return E_OS_CALLEVEL;
    }
    thread = (LONG)GetCurrentThreadId();
    owner = InterlockedCompareExchange(&producer, thread, 0);
    return ((owner == 0) || (owner == thread)) ? E_OK : E_OS_ACCESS;
}

OS_MEMMAP sint32 Os_MailboxQuiescent(void)
{
    sint32 quiescent = 0;
    if (((uint32)InterlockedCompareExchange(&admission, 0, 0) & 1u) == 0u) {
        quiescent = (InterlockedCompareExchange(&published, 0, 0) == 0) ? 1 : 0;
    }
    return quiescent;
}

OS_MEMMAP static StatusType input_context(void)
{
    sint32 owner;
    Os_BackendGuardService();
    if ((Os_BackendReady() == 0) || (Os_Config->input_event == 0u)) {
        return E_OS_STATE;
    }
    owner = Os_BackendInputOwner();
    return (owner == 0) ? E_OS_CALLEVEL : ((owner < 0) ? E_OS_ACCESS : E_OK);
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
#define OS_START_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
/* MR12 RULE 8.9 VIOLATION: Explicit VAR mapping requires file-static scope under SWS_MemMap_00023.
 */
OS_MEMMAP static uint32 read_index; /**< Persistent function-owned state; existing caller/port
                                       synchronization; no Init reset. */
#define OS_STOP_SEC_VAR_CLEARED_32
#include "Os_MemMap.h"
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
OS_MEMMAP StatusType Os_TargetTakeInput(Os_InputRecordType *record)
{
    StatusType status;
    if (record == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    status = input_context();
    if (status != E_OK) {
        return status;
    }
    if (InterlockedCompareExchange(&published, 0, 0) == 0) {
        return E_OS_NOFUNC;
    }
    *record = records[read_index];
    read_index = (read_index + 1u) % OS_INPUT_CAPACITY;
    (void)InterlockedDecrement(&published);
    return E_OK;
}

OS_MEMMAP StatusType Os_TargetInputCount(uint32 *count)
{
    StatusType status;
    if (count == NULL) {
        return E_OS_ILLEGAL_ADDRESS;
    }
    status = input_context();
    if (status == E_OK) {
        *count = (uint32)InterlockedCompareExchange(&published, 0, 0);
    }
    return status;
}
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
