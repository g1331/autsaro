/** @file
 * @brief PDU handle type for the bounded Windows host configuration.
 */
#ifndef COMSTACK_TYPES_H
#define COMSTACK_TYPES_H

#include <stdint.h>

/** @brief Generated PDU handle for the host profile. */
typedef uint8_t PduIdType;

/** @brief PDU length for the generated host stack, including 256-byte diagnostics. */
typedef uint16_t PduLengthType;

/** @brief R24-11 communication-stack PDU payload and optional metadata. */
typedef struct {
    uint8_t *SduDataPtr;     /**< SDU payload bytes. */
    uint8_t *MetaDataPtr;    /**< Optional configured metadata; unused in this host profile. */
    PduLengthType SduLength; /**< Number of SDU payload bytes. */
} PduInfoType;

#endif
