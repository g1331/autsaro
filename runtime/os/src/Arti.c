#include "Arti.h"
#include "Os_Host.h"

Arti_Event Arti_Events[ARTI_EVENT_CAPACITY];
volatile ArtiAtomic32 Arti_EventCount;
volatile ArtiAtomic32 Arti_EventsDropped;
volatile ArtiAtomic32 Arti_DevelopmentError;
static __thread Arti_AddressCapture captured_address;
void Arti_CaptureAddress(uintptr_t address) {
    captured_address.address = address;
    captured_address.valid = 1u;
}
void Arti_CaptureServiceStatus(uint32_t status) {
    captured_address.service_status = status;
    captured_address.status_valid = 1u;
}
Arti_AddressCapture Arti_SaveAddressCapture(void) {
    const Arti_AddressCapture saved = captured_address;
    captured_address.valid = 0u;
    captured_address.status_valid = 0u;
    return saved;
}
void Arti_RestoreAddressCapture(Arti_AddressCapture capture) { captured_address = capture; }

void Arti_Init(void) {
    /* Called before automotive actors exist. Payload bytes outside published
     * slots are inaccessible; only publication flags need clearing. */
    Arti_EventCount = 0;
    Arti_EventsDropped = 0;
    Arti_DevelopmentError = 0;
    captured_address.valid = 0u;
    captured_address.status_valid = 0u;
    for (size_t i = 0u; i < ARTI_EVENT_CAPACITY; ++i) {
        Arti_Events[i].published = 0L;
    }
}
void Arti_GetVersionInfo(Std_VersionInfoType *versioninfo) {
    if (versioninfo != NULL) {
        versioninfo->vendorID = 0u;
        /* This original tool binding has no assigned vendor/module identity;
         * document ID 923 is not an AUTOSAR module identifier. */
        versioninfo->moduleID = 0u;
        versioninfo->sw_major_version = 1u;
        versioninfo->sw_minor_version = 0u;
        versioninfo->sw_patch_version = 0u;
    } else {
        InterlockedExchange(&Arti_DevelopmentError, (LONG)ARTI_E_PARAM_POINTER);
    }
}
void Arti_Record(const char *context, const char *class_name, const char *instance,
                 const char *event, uint32_t instance_parameter, uint32_t event_parameter) {
    /* Slot reservation never waits for an actor that may be preempted or have
     * a damaged native stack. Read the buffer only after OS shutdown/quiescence;
     * the reservation count is not a concurrent publication protocol. */
    LONG slot = InterlockedCompareExchange(&Arti_EventCount, 0L, 0L);
    /* Count is monotonic while actors exist, so contention can cause at most
     * ARTI_EVENT_CAPACITY retries; reservation does not overflow or spin on a
     * suspended owner. Reinitializing during execution is outside the API. */
    while (slot < (LONG)ARTI_EVENT_CAPACITY) {
        const LONG observed = InterlockedCompareExchange(&Arti_EventCount, slot + 1L, slot);
        if (observed == slot) {
            break;
        }
        slot = observed;
    }
    if (slot < (LONG)ARTI_EVENT_CAPACITY) {
        Arti_Events[slot].context = context;
        Arti_Events[slot].class_name = class_name;
        Arti_Events[slot].instance = instance;
        Arti_Events[slot].event = event;
        Arti_Events[slot].instance_parameter = instance_parameter;
        Arti_Events[slot].event_parameter = event_parameter;
        Arti_Events[slot].event_address = captured_address.address;
        Arti_Events[slot].address_valid = captured_address.valid;
        Arti_Events[slot].service_status = captured_address.service_status;
        Arti_Events[slot].status_valid = captured_address.status_valid;
        InterlockedExchange(&Arti_Events[slot].published, 1L);
    } else {
        LONG dropped = InterlockedCompareExchange(&Arti_EventsDropped, 0L, 0L);
        /* Saturate the four-byte native cell rather than wrap its drop count. */
        while (dropped < INT32_MAX) {
            const LONG previous =
                InterlockedCompareExchange(&Arti_EventsDropped, dropped + 1L, dropped);
            if (previous == dropped) {
                break;
            }
            dropped = previous;
        }
    }
    captured_address.valid = 0u;
    captured_address.status_valid = 0u;
}
