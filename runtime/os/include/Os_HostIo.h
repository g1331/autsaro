/** @file Bounded native byte I/O and text records; no C stream buffering. */
#ifndef AUTOSAR_OS_HOST_IO_H
#define AUTOSAR_OS_HOST_IO_H
#include "Os_Version.h"
#include <stddef.h>
#include <stdint.h>

/** Caller-owned record; initialize with {storage, sizeof(storage), 0u, 0}. */
typedef struct {
    char *data;      /**< Borrowed writable bytes; no terminating NUL is required. */
    size_t capacity; /**< Actual storage size in bytes. */
    size_t length;   /**< Constructed bytes, always at most capacity. */
    int failed;      /**< Latched capacity/field error; prevents publishing any prefix. */
} Os_HostTextType;

#define OS_START_SEC_CODE
#include "Os_MemMap.h"
/** Read one input byte without newline or NUL translation.
 * @param[out] byte Non-NULL output, written only when a byte is read.
 * @return 1 for a byte, 0 for EOF, -1 for a native error.
 * Preconditions: native standard input belongs to the caller.
 * Context: host control, outside exclusive areas; may block.
 * Reentrancy: one reader per input. Ownership: borrows byte until return; does not close input.
 */
OS_MEMMAP int Os_HostReadByte(char *byte);

/** Deliver every requested byte to a native standard output handle.
 * @param[in] error_stream 0 selects stdout, 1 selects stderr; other values are refused.
 * @param[in] data Non-NULL readable bytes, borrowed until return.
 * @param[in] length Number of readable bytes; zero is accepted.
 * @return 1 only after every byte is written; 0 on error or zero progress.
 * Preconditions: caller serializes records and owns the selected output for the call.
 * Context: native control/reporting outside exclusive areas; may block, acquires no application
 * lock. Reentrancy: independent outputs may be used concurrently; no shared formatting state.
 * Ownership: no pointers retained or handles closed; failure may leave a visible partial record.
 */
OS_MEMMAP int Os_HostWriteBytes(int error_stream, const char *data, size_t length);

/** Append a NUL-terminated text field, latching insufficient capacity.
 * @param[in,out] text Initialized caller-owned record; no overlapping source.
 * @param[in] value NUL-terminated readable text, borrowed until return.
 * @par Return
 * None.
 * Preconditions: valid record storage and non-NULL arguments.
 * Context: native record construction; no I/O. Reentrancy: independent records only.
 * Ownership: copied bytes belong to text; errors prevent subsequent publication.
 */
OS_MEMMAP void Os_HostTextAppend(Os_HostTextType *text, const char *value);

/** Append one byte, including NUL, or latch insufficient capacity.
 * @param[in,out] text Initialized caller-owned record.
 * @param[in] value Byte to append.
 * @par Return
 * None.
 * Preconditions: valid non-NULL record. Context: native record construction; no I/O.
 * Reentrancy: independent records only. Ownership: byte is copied; no retained pointer.
 */
OS_MEMMAP void Os_HostTextByte(Os_HostTextType *text, char value);

/** Append an unsigned decimal field name and value without locale dependence.
 * @param[in,out] text Initialized caller-owned record.
 * @param[in] name NUL-terminated prefix, including separators required by the protocol.
 * @param[in] value Full uint64 value, including zero and UINT64_MAX.
 * @par Return
 * None.
 * Preconditions: valid non-NULL record and name. Context: native formatting; no I/O.
 * Reentrancy: independent records only. Ownership: prefix/value copied; no retained pointer.
 */
OS_MEMMAP void Os_HostTextField(Os_HostTextType *text, const char *name, uint64_t value);

/** Append an exact-width hexadecimal value; refuse width or value truncation.
 * @param[in,out] text Initialized caller-owned record.
 * @param[in] value Unsigned value fitting the requested width.
 * @param[in] width Number of hex digits, 1 through 16.
 * @param[in] uppercase 0 for lower case, 1 for upper case; other values are refused.
 * @par Return
 * None.
 * Preconditions: valid non-NULL record. Context: native formatting; no I/O.
 * Reentrancy: independent records only. Ownership: value copied; errors latch in text.
 */
OS_MEMMAP void Os_HostTextHex(Os_HostTextType *text, uint64_t value, size_t width, int uppercase);

/** Publish a complete record to stdout, refusing a latched construction error.
 * @param[in] text Initialized record, borrowed until return.
 * @return 1 only after complete native output; 0 on construction or native failure.
 * Preconditions: valid record and serialized stdout ownership.
 * Context: native control/reporting outside exclusive areas; may block.
 * Reentrancy: caller serializes records. Ownership: no retained pointer; record remains
 * caller-owned.
 */
OS_MEMMAP int Os_HostTextWrite(const Os_HostTextType *text);
#define OS_STOP_SEC_CODE
#include "Os_MemMap.h"
/* SWS_BSW_00036: imported AUTOSAR major/minor release. */
#if !defined(MEMMAP_AR_RELEASE_MAJOR_VERSION) || !defined(MEMMAP_AR_RELEASE_MINOR_VERSION)
#error "Os_HostIo.h -> Os_MemMap.h: missing AUTOSAR release"
#elif (MEMMAP_AR_RELEASE_MAJOR_VERSION != 4U) || (MEMMAP_AR_RELEASE_MINOR_VERSION != 10U)
#error "Os_HostIo.h -> Os_MemMap.h: incompatible AUTOSAR release"
#endif
#endif
