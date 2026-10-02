#include "Ecu_Execution.h"
#include "Ecu_Diagnostic.h"
#include "Ecu_Target.h"
#include "Ecu_HostBatch.h"
#include "Dcm_Internal.h"
#include "PduR_Internal.h"
#include <string.h>

typedef struct {
    uint64_t received_at;
    size_t length;
    uint8_t data[ECU_MAX_PDU_PAYLOAD];
} EcuDiagnosticRequest;

static EcuDiagnosticRequest requests[ECU_BATCH_CAPACITY];
static unsigned request_write;
static unsigned request_read;
static unsigned request_count;

EcuStatus Ecu_ExecutionTransmit(CanTxSink sink, PduIdType handle, uint32_t id, uint8_t dlc,
                                const uint8_t data[8]) {
    (void)sink;
    return Ecu_TargetEnqueueTransmit(handle, id, dlc, data);
}

uint64_t Ecu_ExecutionNow(void) { return Ecu_TargetNow(); }

void Ecu_DiagnosticReset(void) {
    request_write = 0u;
    request_read = 0u;
    request_count = 0u;
}

EcuStatus Ecu_DiagnosticAdmit(const EcuDiagnosticConfig *config, const uint8_t *request,
                              size_t length, uint64_t now_ms) {
    EcuDiagnosticRequest *pending = &requests[request_write];
    Ecu_TargetAssertOwner();
    if ((config == NULL) || (request == NULL)) {
        return ECU_ERR_CONFIG;
    }
    if ((length == 0u) || (length > sizeof(pending->data))) {
        return ECU_ERR_TP_LENGTH;
    }
    if (request_count == ECU_BATCH_CAPACITY) {
        return ECU_ERR_TP_BUSY;
    }
    pending->received_at = now_ms;
    pending->length = length;
    (void)memcpy(pending->data, request, length);
    request_write = (request_write + 1u) % ECU_BATCH_CAPACITY;
    ++request_count;
    /* Receipt precedes the S3 check; dispatch stays in the owner phase. */
    Dcm_RecordRequestTime(now_ms);
    return ECU_OK;
}

unsigned Ecu_DiagnosticPending(void) {
    Ecu_TargetAssertOwner();
    return request_count;
}

EcuStatus Ecu_DiagnosticProcess(uint64_t now_ms) {
    EcuStatus status = ECU_OK;
    Ecu_TargetAssertOwner();
    while ((request_count != 0u) && (PduR_DiagnosticReady() != 0) && (status == ECU_OK)) {
        const EcuDiagnosticRequest *pending = &requests[request_read];
        if (pending->received_at > now_ms) {
            return ECU_ERR_TIME;
        }
        status = Dcm_DispatchRequest(pending->data, pending->length, pending->received_at);
        request_read = (request_read + 1u) % ECU_BATCH_CAPACITY;
        --request_count;
    }
    return status;
}
