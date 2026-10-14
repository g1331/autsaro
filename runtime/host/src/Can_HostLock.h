#ifndef CAN_HOSTLOCK_H
#define CAN_HOSTLOCK_H

void Can_Lock(void);
/* 1: acquired; 0: busy; -1: synchronization setup/operation failed. */
int Can_TryLock(void);
void Can_Unlock(void);
/* Det exclusive area: protects the configuration snapshot only (ADR 0018). */
void Can_HostDetLock(void);
void Can_HostDetUnlock(void);
/* Legacy shared state area behind SchM_CanIf; it may enclose the strict driver and Det
 * areas but never the reverse. Removed when its last user is migrated (ADR 0018). */
void Can_HostStateLock(void);
void Can_HostStateUnlock(void);
/* Strict areas (driver, Det) must not nest or call out; a violation aborts the process. */

#endif
