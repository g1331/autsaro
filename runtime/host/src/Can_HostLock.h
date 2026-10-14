/** @file Host lock, per-module strict areas and legacy shared state area (ADR 0018). */
#ifndef CAN_HOSTLOCK_H
#define CAN_HOSTLOCK_H

void Can_Lock(void);
void Can_Unlock(void);
/* Strict SchM areas (ADR 0018), one per migrated module; the driver uses Can_Lock. Each
 * protects its module's data only: a thread holding one that enters any area aborts. */
typedef enum {
    CAN_HOST_AREA_DET = 0,
    CAN_HOST_AREA_CANIF,
    CAN_HOST_AREA_SCHM_MODE,
    CAN_HOST_AREA_COUNT
} Can_HostAreaType;
void Can_HostAreaEnter(Can_HostAreaType area);
void Can_HostAreaExit(Can_HostAreaType area);
/* Legacy shared state area of the modules not yet migrated; it may enclose strict areas
 * but never the reverse. Removed when its last user is migrated (ADR 0018). */
void Can_HostStateLock(void);
void Can_HostStateUnlock(void);

#endif
