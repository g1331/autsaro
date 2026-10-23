/** @file Host driver lock and per-module strict exclusive areas. */
#ifndef CAN_HOSTLOCK_H
#define CAN_HOSTLOCK_H

void Can_Lock(void);
void Can_Unlock(void);
/* Strict SchM areas, one per module; the driver uses Can_Lock. Each
 * protects its module's data only: a thread holding one that enters any area terminates the ECU
 * with status 134. Exit must match the current thread's exact area, including the driver lock. */
typedef enum {
    CAN_HOST_AREA_DET = 0,
    CAN_HOST_AREA_CANIF,
    CAN_HOST_AREA_SCHM_MODE,
    CAN_HOST_AREA_PDUR,
    CAN_HOST_AREA_LSDUR,
    CAN_HOST_AREA_COM,
    CAN_HOST_AREA_CANTP,
    CAN_HOST_AREA_DCM,
    CAN_HOST_AREA_COMM,
    CAN_HOST_AREA_BSWM,
    CAN_HOST_AREA_BUSSM,
    CAN_HOST_AREA_COUNT
} Can_HostAreaType;
void Can_HostAreaEnter(Can_HostAreaType area);
void Can_HostAreaExit(Can_HostAreaType area);

#endif
