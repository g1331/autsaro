/** @file Independent CDD consumers of ComM channel requests and user mode aggregation.
 * Verifies[ComM]: SWS_ComM_00176, SWS_ComM_00663, SWS_ComM_00798
 */
#include "ComM.h"
#include "ComM_BswM.h"
#include "ComM_Dcm.h"
#include "ComM_Internal.h"
#include "BswM_ComM.h"
#include <assert.h>

static ComM_ModeType actual[256];
static unsigned calls[256];
static unsigned bswm[256];
static unsigned dcm[256];
static unsigned notifications[3];
static ComM_ModeType published[3];

static Std_ReturnType bus_request(NetworkHandleType channel, ComM_ModeType mode)
{
    ++calls[channel];
    actual[channel] = mode;
    return E_OK;
}
static Std_ReturnType bus_current(NetworkHandleType channel, ComM_ModeType *mode)
{
    *mode = actual[channel];
    return E_OK;
}
void BswM_ComM_CurrentMode(NetworkHandleType channel, ComM_ModeType mode)
{
    assert(actual[channel] == mode);
    ++bswm[channel];
}
static void diagnostic_mode(uint8 channel)
{
    assert(channel == 89u);
    ++dcm[channel];
}
static void first(ComM_ModeType mode)
{
    published[0] = mode;
    ++notifications[0];
}
static void middle(ComM_ModeType mode)
{
    published[1] = mode;
    ++notifications[1];
}
static void both(ComM_ModeType mode)
{
    published[2] = mode;
    ++notifications[2];
}
static void indicate(NetworkHandleType channel, ComM_ModeType mode)
{
    actual[channel] = mode;
    ComM_BusSM_ModeIndication(channel, mode);
}
static void check_current(ComM_UserHandleType user, ComM_ModeType expected)
{
    ComM_ModeType current = 99u;
    assert(ComM_GetCurrentComMode(user, &current) == E_OK);
    assert(current == expected);
}

int main(void)
{
    static const ComM_ChannelConfigType channels[] = {
        {252u, 2u, NULL_PTR, NULL_PTR, NULL_PTR},
        {89u, 3u, diagnostic_mode, diagnostic_mode, diagnostic_mode},
        {4u, 4u, NULL_PTR, NULL_PTR, NULL_PTR},
        {128u, 5u, NULL_PTR, NULL_PTR, NULL_PTR},
        {7u, 6u, NULL_PTR, NULL_PTR, NULL_PTR},
        {41u, 7u, NULL_PTR, NULL_PTR, NULL_PTR},
    };
    static const NetworkHandleType a[] = {252u};
    static const NetworkHandleType b[] = {89u};
    static const NetworkHandleType ab[] = {89u, 252u};
    static const ComM_UserModeConfigType users[] = {
        {2u, a, 1u, first},
        {8u, b, 1u, middle},
        {17u, ab, 2u, both},
    };
    static const ComM_ConfigType config = {channels, 6u, users, 3u, bus_request, bus_current};

    ComM_Init(&config);
    for (uint16 i = 0u; i < config.channel_count; ++i) {
        const NetworkHandleType channel = channels[i].channel;
        assert(calls[channel] == 1u);
        assert(bswm[channel] == 0u && dcm[channel] == 0u);
        ComM_CommunicationAllowed(channel, TRUE);
    }
    check_current(17u, COMM_NO_COMMUNICATION);
    assert(notifications[0] == 0u && notifications[1] == 0u && notifications[2] == 0u);

    assert(ComM_RequestComMode(2u, COMM_FULL_COMMUNICATION) == E_OK);
    assert(calls[252u] == 2u && calls[89u] == 1u);
    indicate(252u, COMM_FULL_COMMUNICATION);
    check_current(2u, COMM_FULL_COMMUNICATION);
    check_current(8u, COMM_NO_COMMUNICATION);
    check_current(17u, COMM_NO_COMMUNICATION);
    assert(notifications[0] == 1u && notifications[1] == 0u && notifications[2] == 0u);
    assert(bswm[252u] == 1u && bswm[89u] == 0u && dcm[89u] == 0u);

    assert(ComM_RequestComMode(17u, COMM_FULL_COMMUNICATION) == E_OK);
    assert(calls[252u] == 2u && calls[89u] == 2u);
    indicate(89u, COMM_FULL_COMMUNICATION);
    check_current(17u, COMM_FULL_COMMUNICATION);
    assert(notifications[2] == 1u && published[2] == COMM_FULL_COMMUNICATION);
    assert(bswm[89u] == 1u && dcm[89u] == 1u);

    indicate(252u, COMM_SILENT_COMMUNICATION);
    indicate(89u, COMM_SILENT_COMMUNICATION);
    check_current(17u, COMM_SILENT_COMMUNICATION);
    assert(notifications[2] == 2u && published[2] == COMM_SILENT_COMMUNICATION);
    indicate(89u, COMM_NO_COMMUNICATION);
    check_current(17u, COMM_NO_COMMUNICATION);
    assert(notifications[2] == 3u && published[2] == COMM_NO_COMMUNICATION);
    indicate(89u, COMM_NO_COMMUNICATION);
    assert(notifications[2] == 3u && bswm[89u] == 3u && dcm[89u] == 3u);

    assert(ComM_RequestComMode(2u, COMM_NO_COMMUNICATION) == E_OK);
    assert(ComM_RequestComMode(17u, COMM_NO_COMMUNICATION) == E_OK);
    assert(ComM_RunChannel(252u) == COMM_FULL_COM_NETWORK_REQUESTED);
    assert(ComM_RunChannel(252u) == COMM_FULL_COM_READY_SLEEP);
    assert(ComM_RunChannel(89u) == COMM_FULL_COM_NETWORK_REQUESTED);
    assert(ComM_RunChannel(89u) == COMM_FULL_COM_NETWORK_REQUESTED);
    assert(ComM_RunChannel(89u) == COMM_FULL_COM_READY_SLEEP);

    ComM_DCM_ActiveDiagnostic(4u);
    assert(calls[4u] == 2u && calls[7u] == 1u && calls[41u] == 1u && calls[128u] == 1u);
    assert(ComM_RunChannel(128u) == COMM_NO_COM_NO_PENDING_REQUEST);
    assert(ComM_RunChannel(4u) == COMM_FULL_COM_NETWORK_REQUESTED);
    return 0;
}
