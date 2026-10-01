/** @file R24-11 ARTI tool binding for the controlled x64 host targets. */
#ifndef AUTOSAR_HOST_ARTI_H
#define AUTOSAR_HOST_ARTI_H
#include "Std_Types.h"
#include <stddef.h>
#ifdef _WIN32
typedef long ArtiAtomic32;
#else
typedef int32_t ArtiAtomic32;
#endif
typedef char ArtiAtomic32IsFourBytes[(sizeof(ArtiAtomic32) == 4u) ? 1 : -1];

/** @brief Maximum number of retained events; overflow is explicitly counted. */
#define ARTI_EVENT_CAPACITY 4096u
typedef struct {
    const char *context;
    const char *class_name;
    const char *instance;
    const char *event;
    uint32_t instance_parameter;
    uint32_t event_parameter;
    uintptr_t event_address;
    uint8_t address_valid;
    uint32_t service_status;
    uint8_t status_valid;
    volatile ArtiAtomic32 published;
} Arti_Event;
typedef struct {
    uintptr_t address;
    uint8_t valid;
    uint32_t service_status;
    uint8_t status_valid;
} Arti_AddressCapture;
/** @brief Lossless x64 side data for standard uint32 pointer payloads. */
void Arti_CaptureAddress(uintptr_t address);
/** @brief Distinguish a failed getter from a genuine UINT32_MAX result. */
void Arti_CaptureServiceStatus(uint32_t status);
Arti_AddressCapture Arti_SaveAddressCapture(void);
void Arti_RestoreAddressCapture(Arti_AddressCapture capture);
extern Arti_Event Arti_Events[ARTI_EVENT_CAPACITY];
/* Native atomics require the host ABI's four-byte cell type. */
extern volatile ArtiAtomic32 Arti_EventCount;
extern volatile ArtiAtomic32 Arti_EventsDropped;
extern volatile ArtiAtomic32 Arti_DevelopmentError;
#define ARTI_E_INIT_FAILED 0x01u
#define ARTI_E_PARAM_POINTER 0x02u
#define ARTI_STOPWATCH_FLAT 0x00u
#define ARTI_STOPWATCH_NESTED 0x01u

/** @brief Initialize before OS actors start; resets only the tool buffer. */
void Arti_Init(void);
/** @brief Read the host ARTI binding software version.
 * @param versioninfo Writable version result; NULL raises ARTI_E_PARAM_POINTER
 * in the host binding's debugger-visible development-error cell.
 */
void Arti_GetVersionInfo(Std_VersionInfoType *versioninfo);
/** @brief Tool callback; arguments contain only captured values and literals.
 * @param context Literal ARTI execution context.
 * @param class_name Literal standard ARTI class.
 * @param instance Literal configured module SHORT-NAME.
 * @param event Literal standard event name.
 * @param instance_parameter Hardware core index.
 * @param event_parameter Captured event payload.
 */
void Arti_Record(const char *context, const char *class_name, const char *instance,
                 const char *event, uint32_t instance_parameter, uint32_t event_parameter);

/* The standard contract requires literal tokens here. Stringification retains
 * those identities without expanding a caller's unrelated macro definitions.
 * Each numeric argument is evaluated exactly once. */
#define ARTI_TRACE(context, class_name, instance, instance_parameter, event, event_parameter)      \
    Arti_Record(#context, #class_name, #instance, #event, (instance_parameter), (event_parameter))
#endif
