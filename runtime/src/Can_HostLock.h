#ifndef CAN_HOSTLOCK_H
#define CAN_HOSTLOCK_H

void Can_Lock(void);
/* 1: acquired; 0: busy; -1: synchronization setup/operation failed. */
int Can_TryLock(void);
void Can_Unlock(void);

#endif
