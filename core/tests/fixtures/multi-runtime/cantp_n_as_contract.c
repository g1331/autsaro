/** Independent N_As consumer of the delivered CanTp (SWS_CanTp_00075, 00229):
 * a lower frame unconfirmed within N_As aborts the N-SDU with E_NOT_OK, and
 * its late confirmation neither reaches the upper layer nor completes a later
 * transmission on the same connection.
 */
#include "CanTp.h"
#include "PduR_CanTp.h"
#include "LSduR_CanTp.h"
#include "SchM_CanTp.h"
#include "Det.h"
#include <assert.h>
#include <string.h>

enum { LOWER = 3u, UPPER = 7u, N_AS = 5u };
static unsigned frames;
static unsigned completions;
static Std_ReturnType completion;
static unsigned timeouts;

Std_ReturnType LSduR_CanTpTransmit(PduIdType TxPduId, const PduInfoType *PduInfoPtr) {
    assert(TxPduId == LOWER && PduInfoPtr != NULL_PTR && PduInfoPtr->SduLength == 8u);
    ++frames;
    return E_OK;
}
BufReq_ReturnType PduR_CanTpCopyTxData(PduIdType id, const PduInfoType *info,
                                       const RetryInfoType *retry, PduLengthType *available) {
    assert(id == UPPER && retry == NULL_PTR);
    memset(info->SduDataPtr, 0x5a, info->SduLength);
    *available = 0u;
    return BUFREQ_OK;
}
void PduR_CanTpTxConfirmation(PduIdType id, Std_ReturnType result) {
    assert(id == UPPER);
    completion = result;
    ++completions;
}
BufReq_ReturnType PduR_CanTpStartOfReception(PduIdType id, const PduInfoType *info,
                                             PduLengthType length, PduLengthType *available) {
    (void)id; (void)info; (void)length; (void)available;
    assert(0);
    return BUFREQ_E_NOT_OK;
}
BufReq_ReturnType PduR_CanTpCopyRxData(PduIdType id, const PduInfoType *info,
                                       PduLengthType *available) {
    (void)id; (void)info; (void)available;
    assert(0);
    return BUFREQ_E_NOT_OK;
}
void PduR_CanTpRxIndication(PduIdType id, Std_ReturnType result) {
    (void)id; (void)result;
    assert(0);
}
void SchM_Enter_CanTp_CANTP_STATE(void) {}
void SchM_Exit_CanTp_CANTP_STATE(void) {}
Std_ReturnType Det_ReportRuntimeError(uint16_t ModuleId, uint8_t InstanceId, uint8_t ApiId,
                                      uint8_t ErrorId) {
    assert(ModuleId == 35u && InstanceId == 0u && ApiId == 0x06u);
    (void)ErrorId;
    ++timeouts;
    return E_OK;
}

static Std_ReturnType single_frame(void) {
    uint8 payload[4] = {0u};
    const PduInfoType info = {payload, NULL_PTR, 4u};
    return CanTp_Transmit(0u, &info);
}

static void main_functions(unsigned count) {
    unsigned i;
    for (i = 0u; i < count; ++i) {
        CanTp_MainFunction();
    }
}

int main(void) {
    static const CanTp_ConfigType config = {
        .receive = 0u, .transmit = 0u, .lower_receive = 2u, .lower_transmit = LOWER,
        .maximum_length = 256u, .n_as = N_AS, .n_ar = N_AS, .n_bs = 10u, .n_cr = 10u,
        .n_cs = 10u, .main_period_ms = 1u, .wait_frame_max = 0u, .padding = 0xccu,
        .upper_receive = 1u, .upper_transmit = UPPER};
    CanTp_Init(&config);

    /* A confirmation inside N_As completes normally. */
    assert(single_frame() == E_OK && frames == 1u && completions == 0u);
    main_functions(N_AS - 1u);
    assert(completions == 0u && timeouts == 0u);
    CanTp_TxConfirmation(LOWER, E_OK);
    assert(completions == 1u && completion == E_OK);

    /* Missing confirmation: the N-SDU ends with E_NOT_OK at N_As, once. */
    assert(single_frame() == E_OK && frames == 2u);
    main_functions(N_AS - 1u);
    assert(completions == 1u);
    main_functions(1u);
    assert(completions == 2u && completion == E_NOT_OK && timeouts == 1u);
    main_functions(N_AS);
    assert(completions == 2u && timeouts == 1u);

    /* The unconfirmed lower frame keeps its identity: no new transmission
     * until it is confirmed, and its late confirmation is not reported. */
    assert(single_frame() == E_NOT_OK && frames == 2u);
    CanTp_TxConfirmation(LOWER, E_OK);
    assert(completions == 2u);

    /* A failed lower confirmation aborts with E_NOT_OK. */
    assert(single_frame() == E_OK && frames == 3u);
    CanTp_TxConfirmation(LOWER, E_NOT_OK);
    assert(completions == 3u && completion == E_NOT_OK);

    /* A confirmation for another lower PDU does not complete this one. */
    assert(single_frame() == E_OK && frames == 4u);
    CanTp_TxConfirmation(LOWER + 1u, E_OK);
    assert(completions == 3u);
    CanTp_TxConfirmation(LOWER, E_OK);
    assert(completions == 4u && completion == E_OK && timeouts == 1u);
    return 0;
}
