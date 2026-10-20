/** @file Native input mailbox: single-producer admission and owner-Task notification. */
#include "Os_Backend.h"
#include <string.h>

#define OS_START_SEC_CODE
#include "Os_MemMap.h"

static Os_InputRecordType
    records[OS_INPUT_CAPACITY]; /**< SPSC-owned ring slots, atomic publication; no Init reset. */
static uint64 last_ticket;      /**< Single-producer accepted ticket; no Init reset. */
static volatile Os_Atomic32Type published; /**< Atomic complete-record count; no Init reset. */
/* Acceptance bit0 and permanent-close bit1; close never waits for the producer. */
static volatile Os_Atomic32Type admission; /**< Atomic producer/close bits; no Init reset. */

static uint32 input_interrupt(void)
{
    if (Os_TargetReady() != 0) {
        Os_ArtiInternalEnter();
        const StatusType status = SetEvent(Os_Config->input_task, Os_Config->input_event);
        Os_ArtiInternalLeave();
        if (status != E_OK) {
            Os_BackendShutdown(E_OS_STATE);
        }
#if defined(OS_EVENT_TESTS) && (OS_EVENT_TESTS == STD_ON)
        Os_TestInputNotified();
#endif
    }
    return 0u;
}
void Os_MailboxInstall(void)
{
    if (Os_Config->input_event != 0u) {
        vPortSetInterruptHandler(OS_INPUT_INTERRUPT, input_interrupt);
    }
}
sint32 Os_MailboxHandlerAllowed(uint32 interrupt, uint32 (*handler)(void))
{
    return Os_BackendHandlerAllowed(interrupt, handler) &&
           ((Os_Config == NULL) || (Os_Config->input_event == 0u) ||
            (interrupt != OS_INPUT_INTERRUPT) || (handler == input_interrupt));
}
void Os_MailboxClose(void) { InterlockedOr(&admission, 2); }
#if defined(OS_EVENT_TESTS) && (OS_EVENT_TESTS == STD_ON)
void Os_TestInputLastTicket(uint64 ticket) { last_ticket = ticket; }
#endif
StatusType Os_TargetPostInput(const uint8 *data, uint8 length, uint64 *ticket)
{
    static uint32 write_index;
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
    InterlockedIncrement(&published);
    InterlockedAnd(&admission, ~1L);
    if (Os_TargetReady() != 0) {
        Os_PortPostInterrupt(OS_INPUT_INTERRUPT);
    }
    return E_OK;
}
sint32 Os_BridgeActorAllowed(void)
{
    const Os_NativeStackType *actor = Os_StackCurrent();
#ifdef __linux__
    return actor == NULL || actor->role == 'H';
#else
    return actor == NULL;
#endif
}
StatusType Os_BridgeContext(void)
{
    static volatile Os_Atomic32Type producer;
    LONG thread;
    LONG owner;
    if (Os_BridgeActorAllowed() == 0) {
        return E_OS_CALLEVEL;
    }
    thread = (LONG)GetCurrentThreadId();
    owner = InterlockedCompareExchange(&producer, thread, 0);
    return ((owner == 0) || (owner == thread)) ? E_OK : E_OS_ACCESS;
}
sint32 Os_MailboxQuiescent(void)
{
    return ((InterlockedCompareExchange(&admission, 0, 0) & 1) == 0) &&
           (InterlockedCompareExchange(&published, 0, 0) == 0);
}
static StatusType input_context(void)
{
    sint32 owner;
    Os_BackendGuardService();
    if ((Os_BackendReady() == 0) || (Os_Config->input_event == 0u)) {
        return E_OS_STATE;
    }
    owner = Os_BackendInputOwner();
    return (owner == 0) ? E_OS_CALLEVEL : ((owner < 0) ? E_OS_ACCESS : E_OK);
}
StatusType Os_TargetTakeInput(Os_InputRecordType *record)
{
    static uint32 read_index;
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
    InterlockedDecrement(&published);
    return E_OK;
}
StatusType Os_TargetInputCount(uint32 *count)
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
