#ifndef CAN_HOSTLOCK_H
#define CAN_HOSTLOCK_H

void Can_Lock(void);
int Can_TryLock(void);
void Can_Unlock(void);

#endif
