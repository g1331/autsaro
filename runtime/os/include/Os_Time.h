/** @file Counter, alarm and schedule-table configuration plus controlled-tick target services. */
#ifndef AUTOSAR_OS_TIME_H
#define AUTOSAR_OS_TIME_H
#include "Os.h"
#include <stddef.h>
#define OS_MAX_COUNTERS            8u
#define OS_MAX_ALARMS              16u
#define OS_ALARM_ACTIVATE          0u
#define OS_ALARM_EVENT             1u
#define OS_ALARM_CALLBACK          2u
#define OS_ALARM_INCREMENT_COUNTER 3u
#define OS_MAX_SCHEDULE_TABLES     8u
#define OS_MAX_EXPIRY_POINTS       32u
#define OS_MAX_EXPIRY_ACTIONS      16u
#define OS_SCHEDULE_SYNC_NONE      0u
typedef struct {
    CounterType id;
    TickType maximum;
    TickType ticks_per_base;
    TickType minimum_cycle;
    uint8 software;
} Os_CounterConfig;
typedef struct {
    AlarmType id;
    CounterType counter;
    uint8 action;
    TaskType task;
    EventMaskType event;
    void (*callback)(void);
    uint8 autostart_modes;
    uint8 absolute;
    TickType start;
    TickType cycle;
    CounterType increment_counter;
} Os_AlarmConfig;

typedef struct {
    uint8 action;
    TaskType task;
    EventMaskType event;
} Os_ExpiryAction;
typedef struct {
    TickType offset;
    const Os_ExpiryAction *actions;
    size_t action_count;
} Os_ExpiryPoint;
/** Static table retained for the process lifetime; validated before threads. */
typedef struct {
    ScheduleTableType id;
    CounterType counter;
    TickType duration;
    const Os_ExpiryPoint *points;
    size_t point_count;
    uint8 repeating;
    uint8 synchronization;
    uint8 autostart_modes;
    uint8 absolute;
    TickType start;
} Os_ScheduleTableConfig;
typedef struct {
    const Os_CounterConfig *counters;
    size_t counter_count;
    const Os_AlarmConfig *alarms;
    size_t alarm_count;
    CounterType system_counter;
    TaskType owner;
    /* Zero selects standard software Counter/Alarm use without the private
     * host tick handshake; owner must then be INVALID_TASK. A nonzero mask
     * retains the configured AUTOSTART Extended owner and controlled ISR. */
    EventMaskType wake_event;
    void (*error_hook)(StatusType);
    const Os_ScheduleTableConfig *schedule_tables;
    size_t schedule_table_count;
} Os_TimeConfig;
typedef struct {
    uint64 epoch;
    uint64 ticket;
    uint32 kernel_tick;
    TickType counter;
    uint64 alarm_actions;
    uint64 action_errors;
} Os_TickCompletion;
/** Accept the next one-millisecond request from the native control producer.
 * @param epoch Exactly the next epoch; the last confirmed epoch is a no-op.
 * @param ticket Accepted ticket, or the existing confirmed ticket on a no-op.
 * @return E_OK, E_OS_STATE (not ready/unconfirmed/closed), E_OS_VALUE,
 * E_OS_LIMIT (epoch exhausted), E_OS_CALLEVEL, E_OS_ACCESS or E_OS_ILLEGAL_ADDRESS.
 */
StatusType Os_TargetAdvanceOneTick(uint64 epoch, uint64 *ticket);
/** Read a complete tick record from the same native control producer.
 * @param ticket Previously accepted ticket.
 * @param record Complete output, unchanged until confirmation or on rejection.
 * @return E_OK, E_OS_NOFUNC (not complete), E_OS_ID, E_OS_STATE,
 * E_OS_ACCESS, E_OS_CALLEVEL or E_OS_ILLEGAL_ADDRESS.
 */
StatusType Os_TargetTickCompletion(uint64 ticket, Os_TickCompletion *record);
/** Wait for the matching complete record without polling or advancing time.
 * @param ticket Accepted ticket.
 * @param timeout_ms Host watchdog duration, 1 through 5000 milliseconds.
 * @param record Complete output, unchanged on timeout or rejection.
 * @return Completion statuses, E_OS_VALUE for timeout bounds, E_OS_NOFUNC on timeout or
 * after Os_TargetNotifyNative, or E_OS_STATE for a native wait failure. Watchdog time
 * never changes automotive time.
 */
StatusType Os_TargetWaitTick(uint64 ticket, uint32 timeout_ms, Os_TickCompletion *record);
/** Wake the native producer's Os_TargetWaitTick for work it must do before this tick
 * can complete, such as confirming a published output. A wake without such work only
 * returns one early E_OS_NOFUNC; it never completes a tick or changes automotive time.
 * @return E_OK, E_OS_STATE (not ready, closed or signal failure), E_OS_ACCESS or E_OS_CALLEVEL.
 */
StatusType Os_TargetNotifyNative(void);
/** Confirm after the configured automotive owner has completed this tick's work.
 * @param ticket Current delivered ticket.
 * @return E_OK, E_OS_STATE, E_OS_ID, E_OS_ACCESS or E_OS_CALLEVEL.
 */
StatusType Os_TargetCompleteTick(uint64 ticket);
/** Read the delivered tick on its automotive owner.
 * @param ticket Delivered ticket output.
 * @param epoch Delivered logical epoch output.
 * @return E_OK, E_OS_NOFUNC, E_OS_STATE, E_OS_ACCESS, E_OS_CALLEVEL or E_OS_ILLEGAL_ADDRESS.
 */
StatusType Os_TargetCurrentTick(uint64 *ticket, uint64 *epoch);
#endif
