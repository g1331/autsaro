/** @file R24-11 Dcm active and inactive diagnostic notifications to ComM. */
#ifndef COMM_DCM_H
#define COMM_DCM_H
#include "ComM.h"
void ComM_DCM_ActiveDiagnostic(NetworkHandleType Channel);
void ComM_DCM_InactiveDiagnostic(NetworkHandleType Channel);
#endif
