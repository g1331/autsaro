#include "Ecu_Execution.h"
#include "Dcm_Internal.h"
EcuStatus Ecu_DiagnosticAdmit(const EcuDiagnosticConfig *config, const uint8_t *request,
                              size_t length, uint64_t now_ms) {
    (void)config;
    return Dcm_DispatchRequest(request, length, now_ms);
}

unsigned Ecu_DiagnosticPending(void) {
    /* Dispatch completes synchronously; this profile retains no request ring. */
    return 0u;
}
