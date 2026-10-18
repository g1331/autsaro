/** @file Private receive ordering adapter for the controlled ECU owner. */
#ifndef CANTP_INTERNAL_H
#define CANTP_INTERNAL_H
#include "CanTp.h"
/** The serialized ECU owner brackets Can_MainFunction_Read with this ordering hint.
 * TRUE means that epoch's main function is still pending; FALSE ends the receive call.
 * It supplies ordering only, without a clock or a change to standard CanTp callbacks.
 */
void Ecu_CanTpReceptionBeforeMain(boolean pending);
#endif
