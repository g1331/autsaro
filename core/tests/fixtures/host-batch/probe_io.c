/** @file Observe the real probe's output-to-confirmation boundary. */
#include "Ecu_Target.h"
#include "Os_Host.h"
#include "Os_HostIo.h"
#include <string.h>
#ifndef _WIN32
#include <unistd.h>
#endif

static unsigned pending_output;
static int observed_write(const Os_HostTextType *text);
static StatusType observed_confirm(uint64_t ticket, PduIdType pdu);
#define Os_HostTextWrite        observed_write
#define Ecu_TargetConfirmOutput observed_confirm
#define main                    production_main
#include "ecu_probe.c"
#undef main
#undef Ecu_TargetConfirmOutput
#undef Os_HostTextWrite

static int observed_write(const Os_HostTextType *text)
{
    const int result = Os_HostTextWrite(text);
    if (text->length >= 11u && memcmp(text->data, "ecu_output ", 11u) == 0) {
        if (pending_output != 0u) {
            Os_HostExit(90u);
        }
        pending_output = result == 1 ? 1u : 0u;
    }
    return result;
}

static StatusType observed_confirm(uint64_t ticket, PduIdType pdu)
{
    if (pending_output != 1u) {
        Os_HostExit(91u);
    }
    pending_output = 0u;
    if (Os_HostWriteBytes(1, "confirmed\n", sizeof("confirmed\n") - 1u) != 1) {
        Os_HostExit(92u);
    }
    return Ecu_TargetConfirmOutput(ticket, pdu);
}

int main(int argc, char **argv)
{
    if (argc == 2 && strcmp(argv[1], "closed") == 0) {
#ifdef _WIN32
        if (!CloseHandle(GetStdHandle(STD_OUTPUT_HANDLE))) {
#else
        if (close(STDOUT_FILENO) != 0) {
#endif
            return 99;
        }
    } else if (argc != 1) {
        return 98;
    }
    return production_main(1, argv);
}
