/** Public Dcm length boundaries, compiled with each real generated communication type.
 * Verifies: ECUC_Dcm_00738, SWS_Dcm_00094, SWS_Dcm_00444, SWS_Dcm_00642
 * Verifies: SWS_Dcm_00443, SWS_Dcm_00996, SWS_Dcm_00342, SWS_Dcm_00344
 * Verifies: SWS_Dcm_00346, SWS_Dcm_00350, SWS_Dcm_00352
 */
#include "Rte_Dcm_Type.h"
#include "Dcm.h"
#include "Dcm_ComM.h"
#include "ComM_Dcm.h"
#include "SchM_Dcm.h"
#include "PduR_Dcm.h"
#include <assert.h>
#include <string.h>

static unsigned transmissions;
static PduLengthType response_length;
static void authentication(uint8 mode) { assert(mode == 0u); }
void ComM_DCM_ActiveDiagnostic(NetworkHandleType channel) { assert(channel == 3u); }
void ComM_DCM_InactiveDiagnostic(NetworkHandleType channel) { assert(channel == 3u); }
Std_ReturnType SchM_Switch_Dcm_DcmDiagnosticSessionControl(
    Rte_ModeType_DcmDiagnosticSessionControl mode)
{
    assert(mode == 0u);
    return E_OK;
}
Std_ReturnType PduR_DcmTransmit(PduIdType id, const PduInfoType *info)
{
    assert(id == 4u && info->SduDataPtr == NULL_PTR && info->MetaDataPtr == NULL_PTR);
    response_length = info->SduLength;
    ++transmissions;
    return E_OK;
}
static void receive_and_respond(uint16 capacity)
{
    Dcm_ConfigType config = {
        1u, 2u, 3u, 0x1234u, NULL_PTR, 4u, 50u, 5000u, 5u, 64u, 4u, authentication, 0u};
    uint8 bytes[256];
    uint8 response[4] = {0u};
    PduLengthType available = 123u;
    PduInfoType info = {bytes, NULL_PTR, 0u};
    PduInfoType copy = {response, NULL_PTR, 2u};
    RetryInfoType retry = {TP_DATARETRY, 1u};
    const RetryInfoType pending = {TP_CONFPENDING, 0u};
    unsigned before = transmissions;
    memset(bytes, 0x99, sizeof(bytes));
    config.buffer_length = capacity;
    Dcm_Init(&config);
    Dcm_ComM_FullComModeEntered(3u);
    assert(Dcm_StartOfReception(1u, NULL_PTR, 0u, &available) == BUFREQ_E_NOT_OK);
    assert(available == 123u);
    if (capacity < 255u) {
        assert(Dcm_StartOfReception(1u, NULL_PTR, (PduLengthType)(capacity + 1u), &available) ==
               BUFREQ_E_OVFL);
        assert(available == 123u);
    }
    assert(Dcm_StartOfReception(1u, NULL_PTR, (PduLengthType)capacity, &available) == BUFREQ_OK);
    assert(available == capacity);
    /* Invalid Init must preserve the accepted reception, including its capacity. */
    {
        Dcm_ConfigType invalid = config;
        invalid.buffer_length = 7u;
        Dcm_Init(&invalid);
        assert(Dcm_CopyRxData(1u, &info, &available) == BUFREQ_OK && available == capacity);
        invalid.buffer_length = 257u;
        Dcm_Init(&invalid);
        assert(Dcm_CopyRxData(1u, &info, &available) == BUFREQ_OK && available == capacity);
        if (sizeof(PduLengthType) == 1u) {
            invalid.buffer_length = 256u;
            Dcm_Init(&invalid);
            assert(Dcm_CopyRxData(1u, &info, &available) == BUFREQ_OK && available == capacity);
        }
    }
    assert(Dcm_CopyRxData(1u, &info, &available) == BUFREQ_OK && available == capacity);
    info.SduLength = (PduLengthType)(capacity - 1u);
    assert(Dcm_CopyRxData(1u, &info, &available) == BUFREQ_OK && available == 1u);
    info.SduLength = 2u;
    available = 123u;
    assert(Dcm_CopyRxData(1u, &info, &available) == BUFREQ_E_NOT_OK && available == 123u);
    info.SduLength = 0u;
    assert(Dcm_CopyRxData(1u, &info, &available) == BUFREQ_OK && available == 1u);
    Dcm_MainFunction();
    assert(transmissions == before);
    info.SduLength = 1u;
    assert(Dcm_CopyRxData(1u, &info, &available) == BUFREQ_OK && available == 0u);
    Dcm_TpRxIndication(1u, E_OK);
    Dcm_MainFunction();
    assert(transmissions == before + 1u && response_length == 3u);
    assert(Dcm_CopyTxData(2u, &copy, &pending, &available) == BUFREQ_OK && available == 1u);
    assert(response[0] == 0x7fu && response[1] == 0x99u);
    /* Rewind one byte, then consume exactly the remaining two. */
    assert(Dcm_CopyTxData(2u, &copy, &retry, &available) == BUFREQ_OK && available == 0u);
    assert(response[0] == 0x99u && response[1] == 0x11u);
    copy.SduLength = 1u;
    response[0] = 0xa5u;
    available = 123u;
    assert(Dcm_CopyTxData(2u, &copy, NULL_PTR, &available) == BUFREQ_E_BUSY);
    assert(response[0] == 0xa5u && available == 123u);
    assert(Dcm_StartOfReception(1u, NULL_PTR, 1u, &available) == BUFREQ_E_NOT_OK);
    Dcm_TpTxConfirmation(2u, E_OK);
    assert(Dcm_StartOfReception(1u, NULL_PTR, 1u, &available) == BUFREQ_OK);
    assert(Dcm_CopyRxData(1u, &info, &available) == BUFREQ_OK);
    Dcm_TpRxIndication(1u, E_NOT_OK);
    Dcm_MainFunction();
    assert(transmissions == before + 1u);
    assert(Dcm_StartOfReception(1u, NULL_PTR, 1u, &available) == BUFREQ_OK);
    Dcm_TpRxIndication(1u, E_NOT_OK);
}
int main(void)
{
    receive_and_respond(8u);
    receive_and_respond(64u);
    receive_and_respond(255u);
    if (sizeof(PduLengthType) > 1u) {
        receive_and_respond(256u);
    }
    return 0;
}
