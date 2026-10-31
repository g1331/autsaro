/** @file Append-only COM refusals with independent publication and native stderr delivery. */
#include "Ecu_ComRejection.h"
#include "Os_Host.h"
#include "Os_HostIo.h"

typedef struct {
    volatile Os_Atomic32Type published;
    PduIdType id;
    Std_ReturnType result;
} Ecu_ComRejectionSlot;

static Ecu_ComRejectionSlot
    rejection_records[ECU_COM_REJECTION_CAPACITY]; /**< Per-slot atomic publication. */
static volatile Os_Atomic32Type reserved;   /**< Atomic bounded reservation count; never reset. */
static volatile Os_Atomic32Type incomplete; /**< Atomic sticky capacity loss; never reset. */
static uint32 delivered_count;              /**< Sole native reader's successful record count. */

void Ecu_ComRecordRejection(PduIdType id, Std_ReturnType result)
{
    Os_Atomic32Type index = Os_HostAtomicLoad(&reserved);
    while (index < (Os_Atomic32Type)ECU_COM_REJECTION_CAPACITY) {
        const Os_Atomic32Type prior = Os_HostAtomicCompareExchange(&reserved, index + 1, index);
        if (prior == index) {
            rejection_records[index].id = id;
            rejection_records[index].result = result;
            (void)Os_HostAtomicExchange(&rejection_records[index].published, 1);
            return;
        }
        index = prior;
    }
    (void)Os_HostAtomicExchange(&incomplete, 1);
}

static int write_record(const Ecu_ComRejectionSlot *record)
{
    char storage[96];
    Os_HostTextType text = {storage, sizeof(storage), 0u, 0};
    Os_HostTextField(&text, "COM_TX_REJECT pdu=", record->id);
    Os_HostTextField(&text, " result=", record->result);
    Os_HostTextByte(&text, '\n');
    return (text.failed == 0) ? Os_HostWriteBytes(1, text.data, text.length) : 0;
}

static int write_incomplete(void)
{
    static uint8 incomplete_delivered; /**< Sole reader's successful marker flag; never reset. */
    const Os_Atomic32Type lost = Os_HostAtomicLoad(&incomplete);
    if ((incomplete_delivered == 0u) && (lost != 0)) {
        if (Os_HostWriteBytes(
                1, "COM_TX_REJECT_INCOMPLETE\n", sizeof("COM_TX_REJECT_INCOMPLETE\n") - 1u) == 0) {
            return 0;
        }
        incomplete_delivered = 1u;
    }
    return 1;
}

static int write_end(void)
{
    char storage[64];
    Os_HostTextType text = {storage, sizeof(storage), 0u, 0};
    Os_HostTextField(&text, "COM_TX_REJECT_END count=", delivered_count);
    Os_HostTextByte(&text, '\n');
    return (text.failed == 0) ? Os_HostWriteBytes(1, text.data, text.length) : 0;
}

int Ecu_ComDrainRejections(boolean final)
{
    static uint8 delivered[ECU_COM_REJECTION_CAPACITY]; /**< Sole reader; never reset. */
    Os_Atomic32Type reserved_now;
    const Os_Atomic32Type count = Os_HostAtomicLoad(&reserved);
    Os_Atomic32Type index;
    for (index = 0; index < count; ++index) {
        const Os_Atomic32Type published = Os_HostAtomicLoad(&rejection_records[index].published);
        if ((delivered[index] == 0u) && (published != 0)) {
            if (write_record(&rejection_records[index]) == 0) {
                return 0;
            }
            delivered[index] = 1u;
            ++delivered_count;
        }
    }
    reserved_now = Os_HostAtomicLoad(&reserved);
    if (final && (delivered_count != (uint32)reserved_now)) {
        (void)Os_HostAtomicExchange(&incomplete, 1);
    }
    if (write_incomplete() == 0) {
        return 0;
    }
    return final ? write_end() : 1;
}
