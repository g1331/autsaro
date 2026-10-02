#ifndef DCM_INTERNAL_H
#define DCM_INTERNAL_H

#include "Dcm.h"

/* Admission adapters own copied requests; only the core interprets services. */
EcuStatus Dcm_DispatchRequest(const uint8_t *request, size_t length, uint64_t now_ms);
void Dcm_RecordRequestTime(uint64_t now_ms);

#endif
