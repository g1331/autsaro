/** @file Counter, alarm and schedule-table configuration plus controlled-tick target services. */
#ifndef AUTOSAR_OS_TIME_H
#define AUTOSAR_OS_TIME_H
#include "Os_Version.h"
#include "Os.h"
#include <stddef.h>
#define OS_MAX_COUNTERS            8u  /**< Maximum configured Counter descriptors. */
#define OS_MAX_ALARMS              16u /**< Maximum configured Alarm descriptors. */
#define OS_ALARM_ACTIVATE          0u  /**< Expiry action activates the configured Task. */
#define OS_ALARM_EVENT             1u  /**< Expiry action sets configured Task event bits. */
#define OS_ALARM_CALLBACK          2u  /**< Alarm expiry invokes its static callback. */
#define OS_ALARM_INCREMENT_COUNTER 3u  /**< Increment software Counter. */
#define OS_MAX_SCHEDULE_TABLES     8u  /**< Maximum configured ScheduleTable descriptors. */
#define OS_MAX_EXPIRY_POINTS       32u /**< Maximum expiry points in one table. */
#define OS_MAX_EXPIRY_ACTIONS      16u /**< Maximum actions at one expiry point. */
#define OS_SCHEDULE_SYNC_NONE      0u  /**< Only supported table synchronization strategy: NONE. */
/** Static software/controlled Counter bounds; maximum/minimum values use Counter ticks. */
typedef struct {
    CounterType id;          /**< Unique configured Counter identifier. */
    TickType maximum;        /**< Largest Counter tick value, 1 through UINT32_MAX. */
    TickType ticks_per_base; /**< Nonzero Counter ticks per implementation time base. */
    TickType minimum_cycle;  /**< Minimum nonzero Alarm cycle in Counter ticks, no greater than
                                maximum. */
    uint8 software;          /**< 1 for software Counter; 0 for the controlled system Counter. */
} Os_CounterConfigType;
/** Static Alarm action and optional mode-specific autostart, expressed in its Counter ticks. */
typedef struct {
    AlarmType id;           /**< Unique configured Alarm identifier. */
    CounterType counter;    /**< Identifier of the Counter driving this Alarm. */
    uint8 action;           /**< One OS_ALARM_* action selector. */
    TaskType task;          /**< Target Task for ACTIVATE/EVENT; otherwise ignored. */
    EventMaskType event;    /**< Event bits for EVENT actions; otherwise ignored. */
    void (*callback)(void); /**< Static non-NULL body for CALLBACK actions; otherwise unused. */
    uint8 autostart_modes;  /**< Mode bits selecting autostart; zero disables autostart. */
    uint8 absolute;         /**< Nonzero for absolute autostart; zero for relative delay. */
    TickType start;         /**< Autostart value/delay in the selected Counter ticks. */
    TickType cycle;         /**< Repeat period in Counter ticks; zero for one-shot. */
    CounterType increment_counter; /**< Counter incremented by INCREMENT_COUNTER actions; otherwise
                                      ignored. */
} Os_AlarmConfigType;

/** One static ScheduleTable expiry action, activating a Task or setting its events. */
typedef struct {
    uint8 action;        /**< OS_ALARM_ACTIVATE or OS_ALARM_EVENT. */
    TaskType task;       /**< Configured target Task identifier. */
    EventMaskType event; /**< Bits set by EVENT action; ignored by ACTIVATE. */
} Os_ExpiryActionType;
/** Static offset in Counter ticks and ordered array of expiry actions. */
typedef struct {
    TickType offset;                    /**< Offset from table start in its Counter ticks. */
    const Os_ExpiryActionType *actions; /**< Static ordered array of action_count descriptors. */
    uint32 action_count;                /**< Number of actions, bounded by OS_MAX_EXPIRY_ACTIONS. */
} Os_ExpiryPointType;
/** Static table retained for the process lifetime; validated before threads. */
/** Process-lifetime NONE-synchronized ScheduleTable, validated before native actors exist. */
typedef struct {
    ScheduleTableType id; /**< Unique configured ScheduleTable identifier. */
    CounterType counter;  /**< Counter driving the table. */
    TickType duration;    /**< Complete table duration in Counter ticks, including final delay. */
    const Os_ExpiryPointType *points; /**< Static ordered array of point_count expiry points. */
    uint32 point_count;               /**< Number of points, 1 through OS_MAX_EXPIRY_POINTS. */
    uint8 repeating; /**< Nonzero repeats after the final delay unless a successor is selected. */
    uint8 synchronization; /**< OS_SCHEDULE_SYNC_NONE; other strategies are rejected. */
    uint8 autostart_modes; /**< Mode bits selecting autostart; zero disables autostart. */
    uint8 absolute;        /**< Nonzero for absolute autostart; zero for relative offset. */
    TickType start;        /**< Absolute start or positive relative offset in Counter ticks. */
} Os_ScheduleTableConfigType;
/** Process-lifetime Counter/Alarm/table graph and optional single-producer controlled-tick
 * handshake. */
typedef struct {
    const Os_CounterConfigType *counters; /**< Process-lifetime counter_count descriptors. */
    uint32 counter_count;                 /**< Counter count, 1 through OS_MAX_COUNTERS. */
    const Os_AlarmConfigType
        *alarms;        /**< Static alarm_count descriptors; may be NULL for zero count. */
    uint32 alarm_count; /**< Alarm count, zero through OS_MAX_ALARMS. */
    CounterType
        system_counter; /**< Counter driven by the optional one-millisecond controlled tick. */
    TaskType owner;     /**< Controlled-tick owner Task; INVALID_TASK when wake_event is zero. */
    /* Zero selects standard software Counter/Alarm use without the private
     * host tick handshake; owner must then be INVALID_TASK. A nonzero mask
     * retains the configured AUTOSTART Extended owner and controlled ISR. */
    EventMaskType
        wake_event; /**< Controlled owner wake bits; zero disables the private tick handshake. */
    void (*error_hook)(StatusType Error); /**< Optional action-error fallback when no standard
                                            ErrorHook is configured. */
    const Os_ScheduleTableConfigType *schedule_tables; /**< Static schedule_table_count descriptors;
                                                      NULL allowed for zero count. */
    uint32 schedule_table_count; /**< Table count, zero through OS_MAX_SCHEDULE_TABLES. */
} Os_TimeConfigType;
/** Complete acknowledged logical tick snapshot, copied to the native producer only after
 * publication. */
typedef struct {
    uint64 epoch;         /**< Confirmed logical one-millisecond epoch. */
    uint64 ticket;        /**< Accepted request ticket; equals epoch in the selected protocol. */
    uint32 kernel_tick;   /**< Native FreeRTOS tick count snapshot, with its 32-bit wrap. */
    TickType counter;     /**< System Counter modular tick value at completion. */
    uint64 alarm_actions; /**< Cumulative Alarm/ScheduleTable action executions. */
    uint64 action_errors; /**< Cumulative non-E_OK action results. */
} Os_TickCompletionType;
#define OS_START_SEC_CODE
#include "Os_MemMap.h"
/** Accept the next one-millisecond request from the native control producer.
 * @param[in] epoch Exactly the next epoch; the last confirmed epoch is a no-op.
 * @param[out] ticket Accepted ticket, or the existing confirmed ticket on a no-op.
 * @return E_OK, E_OS_STATE (not ready/unconfirmed/closed), E_OS_VALUE,
 * E_OS_LIMIT (epoch exhausted), E_OS_CALLEVEL, E_OS_ACCESS or E_OS_ILLEGAL_ADDRESS.
 * Preconditions: ready controlled-tick configuration and the bound native producer; refusal
 * preserves outputs. Context: single native control producer, outside OS/BSW critical or exclusive
 * areas; WaitTick may block. Reentrancy: non-reentrant for the producer; atomic state coordinates
 * with the automotive owner. Ownership: copies arguments and results during the call; caller output
 * storage is not retained.
 */
OS_MEMMAP StatusType Os_TargetAdvanceOneTick(uint64 epoch, uint64 *ticket);

/** Read a complete tick record from the same native control producer.
 * @param[in] ticket Previously accepted ticket.
 * @param[out] record Complete output, unchanged until confirmation or on rejection.
 * @return E_OK, E_OS_NOFUNC (not complete), E_OS_ID, E_OS_STATE,
 * E_OS_ACCESS, E_OS_CALLEVEL or E_OS_ILLEGAL_ADDRESS.
 * Preconditions: ready controlled-tick configuration and the bound native producer; refusal
 * preserves outputs. Context: single native control producer, outside OS/BSW critical or exclusive
 * areas; WaitTick may block. Reentrancy: non-reentrant for the producer; atomic state coordinates
 * with the automotive owner. Ownership: copies arguments and results during the call; caller output
 * storage is not retained.
 */
OS_MEMMAP StatusType Os_TargetTickCompletion(uint64 ticket, Os_TickCompletionType *record);

/** Wait for the matching complete record without polling or advancing time.
 * @param[in] ticket Accepted ticket.
 * @param[in] timeout_ms Host watchdog duration, 1 through 5000 milliseconds.
 * @param[out] record Complete output, unchanged on timeout or rejection.
 * @return Completion statuses, E_OS_VALUE for timeout bounds, E_OS_NOFUNC on timeout or
 * after Os_TargetNotifyNative, or E_OS_STATE for a native wait failure. Watchdog time
 * never changes automotive time.
 * Preconditions: ready controlled-tick configuration and the bound native producer; refusal
 * preserves outputs. Context: single native control producer, outside OS/BSW critical or exclusive
 * areas; WaitTick may block. Reentrancy: non-reentrant for the producer; atomic state coordinates
 * with the automotive owner. Ownership: copies arguments and results during the call; caller output
 * storage is not retained.
 */
OS_MEMMAP StatusType Os_TargetWaitTick(uint64 ticket,
                                       uint32 timeout_ms,
                                       Os_TickCompletionType *record);

/** Wake the native producer's Os_TargetWaitTick for work it must do before this tick
 * can complete, such as confirming a published output. A wake without such work only
 * returns one early E_OS_NOFUNC; it never completes a tick or changes automotive time.
 * @return E_OK, E_OS_STATE (not ready, closed or signal failure), E_OS_ACCESS or E_OS_CALLEVEL.
 * Preconditions: ready controlled-tick configuration and its configured owner Task; refusal
 * preserves outputs. Context: configured automotive owner Task, outside BSW exclusive areas.
 * Reentrancy: owner-only operation; state publication synchronizes with the native producer.
 * Ownership: no pointer retained; caller outputs, if any, are written before successful return.
 */
OS_MEMMAP StatusType Os_TargetNotifyNative(void);

/** Confirm after the configured automotive owner has completed this tick's work.
 * @param[in] ticket Current delivered ticket.
 * @return E_OK, E_OS_STATE, E_OS_ID, E_OS_ACCESS or E_OS_CALLEVEL.
 * Preconditions: ready controlled-tick configuration and its configured owner Task; refusal
 * preserves outputs. Context: configured automotive owner Task, outside BSW exclusive areas.
 * Reentrancy: owner-only operation; state publication synchronizes with the native producer.
 * Ownership: no pointer retained; caller outputs, if any, are written before successful return.
 */
OS_MEMMAP StatusType Os_TargetCompleteTick(uint64 ticket);

/** Read the delivered tick on its automotive owner.
 * @param[out] ticket Delivered ticket output.
 * @param[out] epoch Delivered logical epoch output.
 * @return E_OK, E_OS_NOFUNC, E_OS_STATE, E_OS_ACCESS, E_OS_CALLEVEL or E_OS_ILLEGAL_ADDRESS.
 * Preconditions: ready controlled-tick configuration and its configured owner Task; refusal
 * preserves outputs. Context: configured automotive owner Task, outside BSW exclusive areas.
 * Reentrancy: owner-only operation; state publication synchronizes with the native producer.
 * Ownership: no pointer retained; caller outputs, if any, are written before successful return.
 */
OS_MEMMAP StatusType Os_TargetCurrentTick(uint64 *ticket, uint64 *epoch);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_Time.h -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_Time.h -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
