/** @file Reference application for periodic CAN data and a host-only value readback. */
#include "Rte_Ingress.h"

#define Ingress_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Ingress_MemMap.h"
static sint16 primary Ingress_VAR_CLEARED_UNSPECIFIED;
static uint8 secondary Ingress_VAR_CLEARED_UNSPECIFIED;
static uint32 extended Ingress_VAR_CLEARED_UNSPECIFIED;
#define Ingress_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Ingress_MemMap.h"

#define Ingress_START_SEC_CODE
#include "Ingress_MemMap.h"
Ingress_CODE void Ingress_Periodic(void)
{
    if (Rte_Read_RxValue_Value(&primary) == E_OK) {
        (void)Rte_Write_TxValue_Value(primary);
    }
#if defined(REFERENCE_TWO_BUSES)
    if (Rte_Read_RxValue1_Value(&secondary) == E_OK) {
        (void)Rte_Write_TxValue1_Value(secondary);
    }
#else
    (void)Rte_Read_RxValue1_Value(&secondary);
#endif
    (void)Rte_Read_RxExtended_Value(&extended);
}

Ingress_CODE Std_ReturnType Ingress_ReadData(Dcm_DataElement_ApplicationValueType data)
{
    uint16 bits = (uint16)primary;
    data[0] = (uint8)(bits >> 8u);
    data[1] = (uint8)bits;
    data[2] = secondary;
    data[3] = (uint8)extended;
    return E_OK;
}
#define Ingress_STOP_SEC_CODE
#include "Ingress_MemMap.h"
