/** @file Immediate integer COM with real Rx group and PDU-level reception DM.
 * Com enters only its own COM_STATE area and calls PduR and its users outside it:
 * a transmission is packed in the area and sent after leaving it; notifications are
 * recorded in the area and published in order by one context at a time.
 */
#include "Com.h"
#include "Com_Cfg.h"
#include "SchM_Com.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(RTE_AR_RELEASE_MAJOR_VERSION) || !defined(RTE_AR_RELEASE_MINOR_VERSION)
#error "Com.c -> SchM_Com.h: missing AUTOSAR release"
#elif (RTE_AR_RELEASE_MAJOR_VERSION != 4U) || (RTE_AR_RELEASE_MINOR_VERSION != 10U)
#error "Com.c -> SchM_Com.h: incompatible AUTOSAR release"
#endif
#include "Com_Internal.h"
#include <stddef.h>
#include <string.h>

#define COM_BITS_PER_BYTE 8u
#define COM_UINT32_BITS   32u
#define COM_BYTE_LAST_BIT 7u
#define COM_UINT32_MAX    0xffffffffu
#define COM_START_SEC_VAR_CLEARED_UNSPECIFIED
#include "Com_MemMap.h"
/* COM_STATE protects every object below. Init and DeInit reset all flags and arrays except
 * publishing, which is cleared only by the context returning from its callback. Init then copies
 * the configured signal initial values. */
static const Com_ConfigType *com_configuration
    COM_VAR_CLEARED; /**< Published configuration; DeInit clears. */
static uint32 signal_values[COM_SIGNAL_CAPACITY] COM_VAR_CLEARED; /**< Signal codes extended to
                                                                     storage width; Init reloads. */
static uint32
    com_ticks[COM_PDU_CAPACITY] COM_VAR_CLEARED; /**< Rx-main periods left; Init resets. */
static boolean com_rx_reloaded[COM_PDU_CAPACITY] COM_VAR_CLEARED; /**< Reloaded in this Rx tick;
                                                                      Init resets. */
/* Notifications owed per Rx signal: every reception is acknowledged (SWS_Com_00883), repeated
 * timeouts coalesce, and the event recorded last is also delivered last, so a stale timeout
 * never follows a fresh reception. */
static uint32 receptions_owed[COM_SIGNAL_CAPACITY] COM_VAR_CLEARED; /**< Pending Rx notifications;
                                                                       Init resets. */
static boolean timeout_owed[COM_SIGNAL_CAPACITY] COM_VAR_CLEARED; /**< Pending timeout notification;
                                                                     Init resets. */
static boolean timeout_last[COM_SIGNAL_CAPACITY] COM_VAR_CLEARED; /**< Timeout follows receptions;
                                                                     Init resets. */
static boolean
    signal_receives[COM_SIGNAL_CAPACITY] COM_VAR_CLEARED; /**< Rx direction; Init sets. */
static boolean receive_started COM_VAR_CLEARED;           /**< Rx group is started; Init resets. */
/* Tx mode FALSE selects the absent ComTxModeFalse, i.e. NONE (ECUC_Com_00496); Init
 * evaluates the TMS of the filterless PDUs to TRUE again (SWS_Com_00677, 00877). */
static boolean tx_mode_false[COM_PDU_CAPACITY] COM_VAR_CLEARED; /**< Tx mode NONE instead of
                                                               PERIODIC; Init resets. */
/* Reception DM of the Rx group; independent of start and stop (SWS_Com_00772). */
static boolean monitoring COM_VAR_CLEARED; /**< Rx deadline monitoring enabled; Init resets. */
static boolean reception_before_main
    COM_VAR_CLEARED; /**< Receive precedes this main tick; Init resets. */
/* A context is delivering a notification; never reset, so it outlives a reinitialization. */
/* MR12 RULE 8.9 VIOLATION: Explicit VAR mapping requires file-static scope under SWS_MemMap_00023.
 */
static boolean publishing COM_VAR_CLEARED; /**< Callback in flight; Init preserves. */
#define COM_STOP_SEC_VAR_CLEARED_UNSPECIFIED
#include "Com_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Com.c -> Com_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Com.c -> Com_MemMap.h: incompatible AUTOSAR release"
#endif

#define COM_START_SEC_CODE
#include "Com_MemMap.h"
/* The helpers below run inside COM_STATE with an initialized configuration. */
static COM_CODE uint16 Com_FindPdu(PduIdType id, boolean receive)
{
    uint16 index = 0u;
    while ((index < com_configuration->pdu_count) &&
           ((com_configuration->pdus[index].pdu != id) ||
            (com_configuration->pdus[index].receive != receive))) {
        ++index;
    }
    return index;
}
static COM_CODE uint16 Com_FindSignal(Com_SignalIdType id)
{
    uint16 index = 0u;
    while ((index < com_configuration->signal_count) &&
           (com_configuration->signals[index].signal_id != id)) {
        ++index;
    }
    return index;
}

static COM_CODE uint32 Com_Mask(uint8 width)
{
    const uint32 mask = COM_UINT32_MAX;
    const uint8 distance = COM_UINT32_BITS - width;
    const uint8 shift = distance % COM_UINT32_BITS;
    return mask >> shift;
}

static COM_CODE boolean Com_IsSigned(Com_IntegerType type)
{
    return ((type == COM_SINT8) || (type == COM_SINT16) || (type == COM_SINT32));
}

static COM_CODE uint32 Com_Extend(const Com_SignalConfigType *signal, uint32 raw)
{
    const uint32 mask = Com_Mask(signal->bit_length);
    uint32 extended = raw;
    if ((Com_IsSigned(signal->type) == TRUE) && (raw > (mask >> 1u))) {
        extended |= ~mask;
    }
    return extended;
}

static COM_CODE uint16 Com_Bit(const Com_SignalConfigType *signal, uint8 ordinal)
{
    uint16 bit = signal->start_bit;
    if (signal->big_endian == TRUE) {
        bit ^= COM_BYTE_LAST_BIT;
        bit += (uint16)ordinal;
        bit ^= COM_BYTE_LAST_BIT;
    } else {
        bit += (uint16)ordinal;
    }
    return bit;
}

static COM_CODE uint8 Com_Shift(const Com_SignalConfigType *signal, uint8 ordinal)
{
    uint8 shift = ordinal;
    if (signal->big_endian == TRUE) {
        shift = signal->bit_length;
        --shift;
        shift -= ordinal;
    }
    return shift;
}

static COM_CODE void Com_Pack(uint16 index, uint8 *bytes)
{
    const Com_PduConfigType *pdu = &com_configuration->pdus[index];
    const uint16 end = pdu->first_signal + pdu->signal_count;
    uint16 row;
    uint8 byte;
    for (byte = 0u; byte < pdu->length; ++byte) {
        bytes[byte] = pdu->unused_byte;
    }
    for (row = pdu->first_signal; row < end; ++row) {
        const Com_SignalConfigType *signal = &com_configuration->signals[row];
        uint8 ordinal;
        for (ordinal = 0u; ordinal < signal->bit_length; ++ordinal) {
            const uint16 bit = Com_Bit(signal, ordinal);
            const uint8 shift = Com_Shift(signal, ordinal);
            const uint16 offset = bit / COM_BITS_PER_BYTE;
            const uint8 mask = (uint8)(1u << (bit % COM_BITS_PER_BYTE));
            const uint8 cleared = (uint8)~mask;
            bytes[offset] &= cleared;
            if (((signal_values[row] >> shift) & 1u) != 0u) {
                bytes[offset] |= mask;
            }
        }
    }
}

static COM_CODE void Com_Unpack(uint16 row, const PduInfoType *info)
{
    const Com_SignalConfigType *signal = &com_configuration->signals[row];
    uint32 raw = 0u;
    uint8 ordinal = 0u;
    while (ordinal < signal->bit_length) {
        const uint16 bit = Com_Bit(signal, ordinal);
        const uint16 offset = bit / COM_BITS_PER_BYTE;
        if (offset >= info->SduLength) {
            break;
        }
        const uint32 byte = (uint32)info->SduDataPtr[offset];
        const uint16 bit_in_byte = bit % COM_BITS_PER_BYTE;
        const uint32 shifted = byte >> bit_in_byte;
        const uint32 value = shifted & 1u;
        raw |= value << Com_Shift(signal, ordinal);
        ++ordinal;
    }
    /* SWS_Com_00574: update and notify only when all bits were received. */
    if (ordinal == signal->bit_length) {
        signal_values[row] = Com_Extend(signal, raw);
        if (receptions_owed[row] != COM_UINT32_MAX) {
            ++receptions_owed[row];
        }
        timeout_last[row] = FALSE;
    }
}

static COM_CODE sint32 Com_Signed(uint32 raw)
{
    const uint32 mask = COM_UINT32_MAX;
    sint32 value;
    if (raw > (mask >> 1u)) {
        /* Invert within the network width before converting to a signed type. */
        const uint32 distance = mask - raw;
        value = -(sint32)distance - 1;
    } else {
        value = (sint32)raw;
    }
    return value;
}

static COM_CODE boolean Com_LoadSignal(const Com_SignalConfigType *signal,
                                       const void *data,
                                       uint32 *raw)
{
    sint32 signed_value = 0;
    uint32 unsigned_value = 0u;
    const uint32 mask = Com_Mask(signal->bit_length);
    boolean valid;
    switch (signal->type) {
        case COM_UINT8: {
            uint8 value;
            (void)memcpy((void *)&value, data, sizeof(value));
            unsigned_value = value;
            break;
        }
        case COM_UINT16: {
            uint16 value;
            (void)memcpy((void *)&value, data, sizeof(value));
            unsigned_value = value;
            break;
        }
        case COM_UINT32:
            (void)memcpy((void *)&unsigned_value, data, sizeof(unsigned_value));
            break;
        case COM_SINT8: {
            sint8 value;
            (void)memcpy((void *)&value, data, sizeof(value));
            signed_value = (sint32)value;
            break;
        }
        case COM_SINT16: {
            sint16 value;
            (void)memcpy((void *)&value, data, sizeof(value));
            signed_value = value;
            break;
        }
        default: /* COM_SINT32, validated during Init. */
            (void)memcpy((void *)&signed_value, data, sizeof(signed_value));
            break;
    }
    if (Com_IsSigned(signal->type) == TRUE) {
        const uint32 positive = mask >> 1u;
        const sint32 maximum = (sint32)positive;
        valid = ((signed_value >= (-maximum - 1)) && (signed_value <= maximum)) ? TRUE : FALSE;
        const uint32 converted = (uint32)signed_value;
        unsigned_value = converted;
    } else {
        valid = (unsigned_value <= mask) ? TRUE : FALSE;
    }
    if (valid == TRUE) {
        *raw = unsigned_value;
    }
    return valid;
}

static COM_CODE void Com_StoreSignal(const Com_SignalConfigType *signal, uint32 raw, void *data)
{
    switch (signal->type) {
        case COM_UINT8: {
            const uint8 value = (uint8)raw;
            (void)memcpy(data, (const void *)&value, sizeof(value));
            break;
        }
        case COM_UINT16: {
            const uint16 value = (uint16)raw;
            (void)memcpy(data, (const void *)&value, sizeof(value));
            break;
        }
        case COM_UINT32:
            (void)memcpy(data, (const void *)&raw, sizeof(raw));
            break;
        case COM_SINT8: {
            const sint32 extended = Com_Signed(raw);
            const sint8 value = (sint8)extended;
            (void)memcpy(data, (const void *)&value, sizeof(value));
            break;
        }
        case COM_SINT16: {
            const sint32 extended = Com_Signed(raw);
            const sint16 value = (sint16)extended;
            (void)memcpy(data, (const void *)&value, sizeof(value));
            break;
        }
        default: { /* COM_SINT32, validated during Init. */
            const sint32 value = Com_Signed(raw);
            (void)memcpy(data, (const void *)&value, sizeof(value));
            break;
        }
    }
}

/* SWS_Com_00291: signals without update bits share the smallest nonzero PDU deadline. */
static COM_CODE uint32 Com_PduTimeout(const Com_PduConfigType *pdu)
{
    const uint16 end = pdu->first_signal + pdu->signal_count;
    uint16 row;
    uint32 minimum = 0u;
    for (row = pdu->first_signal; row < end; ++row) {
        const uint32 timeout = com_configuration->signals[row].timeout_ticks;
        if ((timeout != 0u) && ((minimum == 0u) || (timeout < minimum))) {
            minimum = timeout;
        }
    }
    return minimum;
}

static COM_CODE void Com_RestartRx(boolean initialize)
{
    uint16 index;
    for (index = 0u; index < com_configuration->pdu_count; ++index) {
        const Com_PduConfigType *pdu = &com_configuration->pdus[index];
        if (pdu->receive == TRUE) {
            if (initialize == TRUE) {
                const uint16 end = pdu->first_signal + pdu->signal_count;
                uint16 row;
                for (row = pdu->first_signal; row < end; ++row) {
                    const Com_SignalConfigType *signal = &com_configuration->signals[row];
                    signal_values[row] = Com_Extend(signal, signal->initial_value);
                }
            }
            /* First timeout zero: nothing is monitored until a reception (SWS_Com_00716). */
            com_ticks[index] = 0u;
            com_rx_reloaded[index] = FALSE;
        }
    }
}
static COM_CODE void Com_Reset(void)
{
    uint16 index;
    com_configuration = NULL_PTR;
    receive_started = FALSE;
    monitoring = FALSE;
    reception_before_main = FALSE;
    for (index = 0u; index < COM_SIGNAL_CAPACITY; ++index) {
        signal_values[index] = 0u;
        receptions_owed[index] = 0u;
        timeout_owed[index] = FALSE;
        timeout_last[index] = FALSE;
        signal_receives[index] = FALSE;
    }
    for (index = 0u; index < COM_PDU_CAPACITY; ++index) {
        com_ticks[index] = 0u;
        com_rx_reloaded[index] = FALSE;
        tx_mode_false[index] = FALSE;
    }
}
/* Takes the next owed notification: its Rx signal index, or signal_count when nothing is owed. */
static COM_CODE uint16 Com_TakeOwed(boolean *timeout)
{
    uint16 index = 0u;
    uint16 taken = com_configuration->signal_count;
    while ((taken == com_configuration->signal_count) &&
           (index < com_configuration->signal_count)) {
        if ((timeout_owed[index] == TRUE) &&
            ((timeout_last[index] == FALSE) || (receptions_owed[index] == 0u))) {
            timeout_owed[index] = FALSE;
            *timeout = TRUE;
            taken = index;
        } else if (receptions_owed[index] != 0u) {
            --receptions_owed[index];
            *timeout = FALSE;
            taken = index;
        } else {
            ++index;
        }
    }
    return taken;
}
/* Single-flight ordered publication: a context that finds another one delivering
 * only records; the delivering context keeps going until nothing is owed. */
static COM_CODE void Com_Publish(void)
{
    boolean more = TRUE;
    while (more == TRUE) {
        Com_NotificationType notify = NULL_PTR;
        CbkHandleIdType handle = 0u;
        more = FALSE;
        SchM_Enter_Com_COM_STATE();
        if ((publishing == FALSE) && (com_configuration != NULL_PTR)) {
            boolean timeout = FALSE;
            const uint16 index = Com_TakeOwed(&timeout);
            if (index < com_configuration->signal_count) {
                notify = (timeout == TRUE) ? com_configuration->timeout_notification
                                           : com_configuration->receive_notification;
                handle = com_configuration->signals[index].callback_handle;
                more = TRUE;
                publishing = TRUE;
            }
        }
        SchM_Exit_Com_COM_STATE();
        if (more == TRUE) {
            notify(handle);
            SchM_Enter_Com_COM_STATE();
            publishing = FALSE;
            SchM_Exit_Com_COM_STATE();
        }
    }
}
static COM_CODE boolean Com_ValidSignals(const Com_ConfigType *config, uint16 pdu_index)
{
    boolean valid = TRUE;
    const Com_PduConfigType *pdu = &config->pdus[pdu_index];
    const uint16 end = pdu->first_signal + pdu->signal_count;
    const uint16 bits = (uint16)pdu->length * COM_BITS_PER_BYTE;
    uint16 row;
    for (row = pdu->first_signal; row < end; ++row) {
        const Com_SignalConfigType *signal = &config->signals[row];
        uint8 limit = COM_UINT32_BITS;
        uint16 previous;
        if ((signal->type == COM_UINT8) || (signal->type == COM_SINT8)) {
            limit = COM_BITS_PER_BYTE;
        } else if ((signal->type == COM_UINT16) || (signal->type == COM_SINT16)) {
            limit = 2u * COM_BITS_PER_BYTE;
        } else if ((signal->type != COM_UINT32) && (signal->type != COM_SINT32)) {
            valid = FALSE;
        } else {
            /* UINT32 and SINT32 use the default width above. */
        }
        if ((signal->bit_length == 0u) || (signal->bit_length > limit) ||
            (signal->start_bit >= bits)) {
            valid = FALSE;
        } else {
            const uint8 last = signal->bit_length - 1u;
            if ((Com_Bit(signal, last) >= bits) ||
                (signal->initial_value > Com_Mask(signal->bit_length))) {
                valid = FALSE;
            }
        }
        for (previous = 0u; previous < row; ++previous) {
            if (signal->signal_id == config->signals[previous].signal_id) {
                valid = FALSE;
            }
        }
        if (pdu->receive == TRUE) {
            uint16 other;
            for (other = 0u; other <= pdu_index; ++other) {
                const Com_PduConfigType *candidate = &config->pdus[other];
                const uint16 candidate_end =
                    (other == pdu_index) ? row
                                         : (candidate->first_signal + candidate->signal_count);
                if (candidate->receive == TRUE) {
                    for (previous = candidate->first_signal; previous < candidate_end; ++previous) {
                        if (signal->callback_handle == config->signals[previous].callback_handle) {
                            valid = FALSE;
                        }
                    }
                }
            }
        }
    }
    return valid;
}
static COM_CODE boolean Com_ValidConfig(const Com_ConfigType *config)
{
    boolean valid = FALSE;
    if ((config != NULL_PTR) && (config->pdus != NULL_PTR) && (config->signals != NULL_PTR) &&
        (config->pdu_count > 0u) && (config->pdu_count <= COM_PDU_CAPACITY) &&
        (config->signal_count > 0u) && (config->signal_count <= COM_SIGNAL_CAPACITY)) {
        uint16 next_signal = 0u;
        uint16 index;
        valid = TRUE;
        for (index = 0u; (index < config->pdu_count) && (valid == TRUE); ++index) {
            uint16 previous;
            const Com_PduConfigType *pdu = &config->pdus[index];
            if ((pdu->length == 0u) || (pdu->length > COM_PAYLOAD_CAPACITY) ||
                (pdu->first_signal != next_signal) || (pdu->signal_count == 0u) ||
                (pdu->signal_count > (config->signal_count - next_signal)) ||
                (((pdu->receive == TRUE) && ((config->receive_notification == NULL_PTR) ||
                                             (config->timeout_notification == NULL_PTR))) ||
                 ((pdu->receive == FALSE) &&
                  ((config->transmit == NULL_PTR) || (config->tx_rejection == NULL_PTR))))) {
                valid = FALSE;
            } else {
                valid = Com_ValidSignals(config, index);
                next_signal += pdu->signal_count;
            }
            for (previous = 0u; previous < index; ++previous) {
                if ((pdu->receive == config->pdus[previous].receive) &&
                    (pdu->pdu == config->pdus[previous].pdu)) {
                    valid = FALSE;
                }
            }
        }
        if (next_signal != config->signal_count) {
            valid = FALSE;
        }
    }
    return valid;
}
COM_CODE void Com_Init(const Com_ConfigType *config)
{
    const boolean valid = Com_ValidConfig(config);
    SchM_Enter_Com_COM_STATE();
    Com_Reset();
    if (valid == TRUE) {
        com_configuration = config;
        uint16 index;
        for (index = 0u; index < config->pdu_count; ++index) {
            const Com_PduConfigType *pdu = &config->pdus[index];
            const uint16 end = pdu->first_signal + pdu->signal_count;
            uint16 row;
            for (row = pdu->first_signal; row < end; ++row) {
                signal_receives[row] = pdu->receive;
                const Com_SignalConfigType *signal = &config->signals[row];
                signal_values[row] = Com_Extend(signal, signal->initial_value);
            }
        }
    }
    SchM_Exit_Com_COM_STATE();
}
COM_CODE void Com_DeInit(void)
{
    SchM_Enter_Com_COM_STATE();
    Com_Reset();
    SchM_Exit_Com_COM_STATE();
}
/* MR12 RULE 8.7 VIOLATION: the library export retains its original interface, state owner and error
 * controls. */
COM_CODE Com_StatusType Com_GetStatus(void)
{
    Com_StatusType status;
    SchM_Enter_Com_COM_STATE();
    status = (com_configuration == NULL_PTR) ? COM_UNINIT : COM_INIT;
    SchM_Exit_Com_COM_STATE();
    return status;
}
COM_CODE uint8 Com_SendSignal(Com_SignalIdType SignalId, const void *SignalDataPtr)
{
    uint8 result = COM_SERVICE_NOT_AVAILABLE;
    SchM_Enter_Com_COM_STATE();
    if (com_configuration != NULL_PTR) {
        const uint16 index = Com_FindSignal(SignalId);
        result = E_NOT_OK;
        if ((index < COM_SIGNAL_CAPACITY) && (index < com_configuration->signal_count) &&
            (SignalDataPtr != NULL_PTR) && (signal_receives[index] == FALSE)) {
            if (Com_LoadSignal(&com_configuration->signals[index],
                               SignalDataPtr,
                               &signal_values[index]) == TRUE) {
                result = E_OK;
            }
        }
    }
    SchM_Exit_Com_COM_STATE();
    return result;
}
COM_CODE uint8 Com_ReceiveSignal(Com_SignalIdType SignalId, void *SignalDataPtr)
{
    uint8 result = COM_SERVICE_NOT_AVAILABLE;
    SchM_Enter_Com_COM_STATE();
    if (com_configuration != NULL_PTR) {
        const uint16 index = Com_FindSignal(SignalId);
        result = E_NOT_OK;
        if ((index < COM_SIGNAL_CAPACITY) && (index < com_configuration->signal_count) &&
            (SignalDataPtr != NULL_PTR) && (signal_receives[index] == TRUE)) {
            Com_StoreSignal(
                &com_configuration->signals[index], signal_values[index], SignalDataPtr);
            result = (receive_started == TRUE) ? E_OK : COM_SERVICE_NOT_AVAILABLE;
        }
    }
    SchM_Exit_Com_COM_STATE();
    return result;
}
/* No ComSignalDataInvalidValue is configurable: no signal can be invalidated (SWS_Com_00643). */
/* MR12 RULE 8.7 VIOLATION: standard API with external linkage (SWS_Com_00203) that no
 * module of the selected program calls; see the Com deviation record. */
COM_CODE uint8 Com_InvalidateSignal(Com_SignalIdType SignalId)
{
    (void)SignalId;
    return COM_SERVICE_NOT_AVAILABLE;
}
COM_CODE void Com_IpduGroupStart(Com_IpduGroupIdType IpduGroupId, boolean Initialize)
{
    SchM_Enter_Com_COM_STATE();
    if ((com_configuration != NULL_PTR) && (IpduGroupId == com_configuration->receive_group) &&
        (receive_started == FALSE)) {
        receive_started = TRUE;
        Com_RestartRx(Initialize);
    }
    SchM_Exit_Com_COM_STATE();
}
/* MR12 RULE 8.7 VIOLATION: the library export retains its original interface, state owner and error
 * controls. */
COM_CODE void Com_IpduGroupStop(Com_IpduGroupIdType IpduGroupId)
{
    SchM_Enter_Com_COM_STATE();
    if ((com_configuration != NULL_PTR) && (IpduGroupId == com_configuration->receive_group)) {
        receive_started = FALSE;
        Com_RestartRx(FALSE);
    }
    SchM_Exit_Com_COM_STATE();
}
COM_CODE void Com_EnableReceptionDM(Com_IpduGroupIdType IpduGroupId)
{
    SchM_Enter_Com_COM_STATE();
    if ((com_configuration != NULL_PTR) && (IpduGroupId == com_configuration->receive_group) &&
        (monitoring == FALSE)) {
        monitoring = TRUE;
        Com_RestartRx(FALSE);
    }
    SchM_Exit_Com_COM_STATE();
}
/* MR12 RULE 8.7 VIOLATION: the library export retains its original interface, state owner and error
 * controls. */
COM_CODE void Com_DisableReceptionDM(Com_IpduGroupIdType IpduGroupId)
{
    SchM_Enter_Com_COM_STATE();
    if ((com_configuration != NULL_PTR) && (IpduGroupId == com_configuration->receive_group)) {
        monitoring = FALSE;
    }
    SchM_Exit_Com_COM_STATE();
}
COM_CODE void Ecu_ComReceptionBeforeMain(boolean pending)
{
    SchM_Enter_Com_COM_STATE();
    reception_before_main = pending;
    SchM_Exit_Com_COM_STATE();
}
COM_CODE void Com_RxIndication(PduIdType RxPduId, const PduInfoType *PduInfoPtr)
{
    SchM_Enter_Com_COM_STATE();
    if ((com_configuration != NULL_PTR) && (receive_started == TRUE) && (PduInfoPtr != NULL_PTR) &&
        ((PduInfoPtr->SduDataPtr != NULL_PTR) || (PduInfoPtr->SduLength == 0u))) {
        const uint16 index = Com_FindPdu(RxPduId, TRUE);
        if ((index < com_configuration->pdu_count) &&
            (com_configuration->pdus[index].receive == TRUE)) {
            const Com_PduConfigType *pdu = &com_configuration->pdus[index];
            const uint16 end = pdu->first_signal + pdu->signal_count;
            uint16 row;
            /* I-PDU based DM is reset first, also for a short PDU (SWS_Com_00396). */
            if (monitoring == TRUE) {
                com_ticks[index] = Com_PduTimeout(pdu);
                com_rx_reloaded[index] = reception_before_main;
            }
            for (row = pdu->first_signal; row < end; ++row) {
                Com_Unpack(row, PduInfoPtr);
            }
        }
    }
    SchM_Exit_Com_COM_STATE();
    Com_Publish();
}
/* MR12 RULE 8.7 VIOLATION: standard API with external linkage (SWS_Com_00784) that no
 * module of the selected program calls; see the Com deviation record. */
COM_CODE void Com_SwitchIpduTxMode(PduIdType PduId, boolean Mode)
{
    SchM_Enter_Com_COM_STATE();
    if (com_configuration != NULL_PTR) {
        const uint16 index = Com_FindPdu(PduId, FALSE);
        if ((index < com_configuration->pdu_count) &&
            (com_configuration->pdus[index].receive == FALSE)) {
            /* PERIODIC at the Tx main period: the next call after a switch to TRUE sends. */
            tx_mode_false[index] = (Mode == TRUE) ? FALSE : TRUE;
        }
    }
    SchM_Exit_Com_COM_STATE();
}
COM_CODE Std_ReturnType Com_TriggerTransmit(PduIdType TxPduId, PduInfoType *PduInfoPtr)
{
    Std_ReturnType result = E_NOT_OK;
    SchM_Enter_Com_COM_STATE();
    if ((com_configuration != NULL_PTR) && (PduInfoPtr != NULL_PTR) &&
        (PduInfoPtr->SduDataPtr != NULL_PTR)) {
        const uint16 index = Com_FindPdu(TxPduId, FALSE);
        if ((index < com_configuration->pdu_count) &&
            (com_configuration->pdus[index].receive == FALSE) &&
            (PduInfoPtr->SduLength >= com_configuration->pdus[index].length)) {
            Com_Pack(index, PduInfoPtr->SduDataPtr);
            PduInfoPtr->SduLength = com_configuration->pdus[index].length;
            result = E_OK;
        }
    }
    SchM_Exit_Com_COM_STATE();
    return result;
}
/* MR12 RULE 8.7 VIOLATION: the library export retains its original interface, state owner and error
 * controls. */
COM_CODE Std_ReturnType Com_TriggerIPDUSend(PduIdType PduId)
{
    uint8 bytes[COM_PAYLOAD_CAPACITY];
    PduInfoType info = {bytes, NULL_PTR, 0u};
    Com_TransmitType transmit = NULL_PTR;
    Com_TxRejectionType observer = NULL_PTR;
    SchM_Enter_Com_COM_STATE();
    if (com_configuration != NULL_PTR) {
        const uint16 index = Com_FindPdu(PduId, FALSE);
        if ((index < com_configuration->pdu_count) &&
            (com_configuration->pdus[index].receive == FALSE)) {
            transmit = com_configuration->transmit;
            observer = com_configuration->tx_rejection;
            Com_Pack(index, bytes);
            info.SduLength = com_configuration->pdus[index].length;
        }
    }
    SchM_Exit_Com_COM_STATE();
    /* Started I-PDUs are triggered; without retry a lower rejection is not a stop (00348). */
    if (transmit != NULL_PTR) {
        const Std_ReturnType result = transmit(PduId, &info);
        if (result != E_OK) {
            observer(PduId, result);
        }
    }
    return (transmit != NULL_PTR) ? E_OK : E_NOT_OK;
}
COM_CODE void Com_TxConfirmation(PduIdType TxPduId, Std_ReturnType result)
{
    /* The selected periodic PDUs have no Tx ACK/NACK callback or Tx DM.
     * Lower acceptance/confirmation does not suppress the next periodic request.
     */
    (void)TxPduId;
    (void)result;
}
COM_CODE void Ecu_ComMainFunctionRx(void)
{
    SchM_Enter_Com_COM_STATE();
    if ((com_configuration != NULL_PTR) && (receive_started == TRUE) && (monitoring == TRUE)) {
        uint16 index;
        for (index = 0u; index < com_configuration->pdu_count; ++index) {
            const Com_PduConfigType *pdu = &com_configuration->pdus[index];
            if ((pdu->receive == TRUE) && (com_ticks[index] != 0u)) {
                if (com_rx_reloaded[index] == FALSE) {
                    --com_ticks[index];
                    if (com_ticks[index] == 0u) {
                        const uint16 end = pdu->first_signal + pdu->signal_count;
                        uint16 row;
                        /* Restarted on expiry (SWS_Com_00889). */
                        com_ticks[index] = Com_PduTimeout(pdu);
                        for (row = pdu->first_signal; row < end; ++row) {
                            if (com_configuration->signals[row].timeout_ticks != 0u) {
                                timeout_owed[row] = TRUE;
                                timeout_last[row] = TRUE;
                            }
                        }
                    }
                }
                com_rx_reloaded[index] = FALSE;
            }
        }
    }
    reception_before_main = FALSE;
    SchM_Exit_Com_COM_STATE();
    Com_Publish();
}
COM_CODE void Ecu_ComMainFunctionTx(void)
{
    uint16 index = 0u;
    boolean more = TRUE;
    while (more == TRUE) {
        uint8 bytes[COM_PAYLOAD_CAPACITY];
        PduInfoType info = {bytes, NULL_PTR, 0u};
        PduIdType id = 0u;
        Com_TransmitType transmit = NULL_PTR;
        Com_TxRejectionType observer = NULL_PTR;
        SchM_Enter_Com_COM_STATE();
        more = ((index < COM_PDU_CAPACITY) && (com_configuration != NULL_PTR) &&
                (index < com_configuration->pdu_count));
        if ((more == TRUE) && (com_configuration->pdus[index].receive == FALSE) &&
            (tx_mode_false[index] == FALSE)) {
            id = com_configuration->pdus[index].pdu;
            transmit = com_configuration->transmit;
            observer = com_configuration->tx_rejection;
            Com_Pack(index, bytes);
            info.SduLength = com_configuration->pdus[index].length;
        }
        SchM_Exit_Com_COM_STATE();
        /* Without ComRetryFailedTransmitRequests a rejected PERIODIC request is not retried. */
        if (transmit != NULL_PTR) {
            const Std_ReturnType result = transmit(id, &info);
            if (result != E_OK) {
                observer(id, result);
            }
        }
        ++index;
    }
}
#define COM_STOP_SEC_CODE
#include "Com_MemMap.h"
