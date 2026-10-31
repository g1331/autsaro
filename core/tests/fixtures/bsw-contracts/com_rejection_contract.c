/** Independent two-Tx refusal oracle: exact result, return semantics, continuation and no retry. */
#include "Com.h"
#include "Com_Internal.h"
#include "Ecu_Config.h"
#include <assert.h>
const EcuPolicyConfig Ecu_Policy = {.tx_confirmation = ECU_TX_SYNCHRONOUS};
static unsigned calls[2];
static unsigned rejections;
static unsigned returned;
static Std_ReturnType transmit(PduIdType id, const PduInfoType *info)
{
    assert(info->SduLength == 4u && (id == 5u || id == 9u));
    ++calls[id == 5u ? 0u : 1u];
    returned = 1u;
    return id == 5u ? 3u : E_OK;
}
static void rejection(PduIdType id, Std_ReturnType result)
{
    assert(returned == 1u && id == 5u && result == 3u);
    assert(Com_GetStatus() == COM_INIT);
    ++rejections;
    returned = 0u;
}
static const Com_PduConfigType pdus[] = {{5u, 0u, FALSE, 0u, 0u, 0u}, {9u, 1u, FALSE, 0u, 0u, 0u}};
int main(void)
{
    Com_ConfigType config = {pdus, 2u, 0u, NULL_PTR, NULL_PTR, transmit, NULL_PTR};
    Com_Init(&config);
    assert(Com_GetStatus() == COM_UNINIT);
    assert(Com_TriggerIPDUSend(5u) == E_NOT_OK && calls[0] == 0u);
    config.tx_rejection = rejection;
    Com_Init(&config);
    assert(Com_GetStatus() == COM_INIT);
    assert(Com_TriggerIPDUSend(77u) == E_NOT_OK && rejections == 0u);
    assert(Com_TriggerIPDUSend(5u) == E_OK && rejections == 1u && calls[0] == 1u);
    Ecu_ComMainFunctionTx();
    assert(rejections == 2u && calls[0] == 2u && calls[1] == 1u);
    Ecu_ComMainFunctionTx();
    assert(rejections == 3u && calls[0] == 3u && calls[1] == 2u);
    Com_SwitchIpduTxMode(5u, FALSE);
    Ecu_ComMainFunctionTx();
    assert(rejections == 3u && calls[0] == 3u && calls[1] == 3u);
    assert(Com_TriggerIPDUSend(5u) == E_OK && rejections == 4u && calls[0] == 4u);
    assert(Com_TriggerIPDUSend(9u) == E_OK && rejections == 4u && calls[1] == 4u);
    Com_DeInit();
    Ecu_ComMainFunctionTx();
    assert(rejections == 4u && calls[0] == 4u && calls[1] == 4u);
    return 0;
}
