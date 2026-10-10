/** @file R24-11 ComM notifications for configured Dcm channels. */
#ifndef DCM_COMM_H
#define DCM_COMM_H
#include "Std_Types.h"
void Dcm_ComM_NoComModeEntered(uint8 NetworkId);
void Dcm_ComM_SilentComModeEntered(uint8 NetworkId);
void Dcm_ComM_FullComModeEntered(uint8 NetworkId);
#endif
