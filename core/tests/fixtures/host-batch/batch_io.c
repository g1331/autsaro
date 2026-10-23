/** @file Direct production field refusal and native input-error consumer. */
#include "Ecu_HostBridge.h"
#include "Os_Host.h"
#include <string.h>
#ifndef _WIN32
#include <errno.h>
#include <unistd.h>
static unsigned read_calls;
static ssize_t interrupted_read(int descriptor, void *data, size_t length);
#define read interrupted_read
#endif
#define main production_main
#include "ecu_host_batch.c"
#undef main
#ifndef _WIN32
#undef read
static ssize_t interrupted_read(int descriptor, void *data, size_t length)
{
    if (read_calls++ == 0u) {
        errno = EINTR;
        return -1;
    }
    return read(descriptor, data, length);
}
#endif

int main(int argc, char **argv)
{
    if (argc != 2) {
        return 99;
    }
    if (strcmp(argv[1], "read-error") == 0) {
#ifdef _WIN32
        if (!CloseHandle(GetStdHandle(STD_INPUT_HANDLE))) {
#else
        if (close(STDIN_FILENO) != 0) {
#endif
            return 98;
        }
        return production_main();
    }
    if (strcmp(argv[1], "read-eof") == 0) {
        return production_main();
    }
    if (strcmp(argv[1], "fields") == 0) {
        Ecu_OutputRecord output = {0};
        Ecu_HostBatchInitialize(&batch);
        output.dlc = 9u;
        if (write_output(&output, &batch, E_OK, NULL) != 0) {
            return 97;
        }
        output.dlc = 8u;
        output.can_id = 0x800u;
        if (write_output(&output, &batch, E_OK, NULL) != 0) {
            return 96;
        }
        output.epoch = UINT64_MAX;
        output.ticket = UINT64_MAX;
        output.can_id = 0x7ffu;
        for (size_t i = 0u; i < sizeof(output.data); ++i) {
            output.data[i] = 0xffu;
        }
        batch.sequence = UINT64_MAX;
        batch.batch_id = UINT64_MAX;
        batch.completed_epoch = UINT64_MAX;
        batch.first_input_sequence = UINT64_MAX;
        batch.transport_epoch = UINT64_MAX;
        batch.count = ECU_BATCH_CAPACITY;
        batch.transport_count = UINT16_MAX;
        return write_output(&output, &batch, E_OK, NULL) == 1 &&
                       write_output(NULL, &batch, E_OK, NULL) == 1
                   ? 0
                   : 95;
    }
    return 94;
}
