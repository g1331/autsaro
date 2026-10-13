/** @file EcuM wake-up indication for the configured CDD channel.
 * Unknown channels and calls before ComM_Init have no state effect.
 */
#ifndef COMM_ECUM_H
#define COMM_ECUM_H
#include "ComM.h"
void ComM_EcuM_WakeUpIndication(NetworkHandleType Channel);
#endif
