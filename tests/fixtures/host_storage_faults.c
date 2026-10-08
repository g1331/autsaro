#include "NvM.h"
#include "NvM_HostStorage.h"
#include <errno.h>
#include <stdio.h>
#include <string.h>

static unsigned opens;
static unsigned closes;
static int fail_close;
FILE *Test_Open(const char *path, const char *mode) {
    ++opens;
    return fopen(path, mode);
}
int Test_Close(FILE *file) {
    int result = fclose(file);
    ++closes;
    if (fail_close != 0) {
        errno = EIO;
        result = EOF;
    }
    return result;
}
int main(int argc, char **argv) {
    uint8_t status = 0xa5u;
    const EcuFrameConfig frame = {.id = 0x123u, .dlc = 8u, .timeout_ms = 10u};
    const EcuDtcConfig dtc = {.code = 0x123456u, .monitor_frame_index = 0u};
    const EcuDiagnosticConfig diagnostic = {.dtc = &dtc};
    const EcuConfig config = {.frames = &frame, .frame_count = 1u, .diagnostic = &diagnostic};
    unsigned before;
    if (argc != 3) {
        return 1;
    }
    if (strcmp(argv[1], "init") == 0) {
        if (NvM_Init(&config, argv[2], &status) != ECU_OK || status != 0x50u) {
            return 2;
        }
        before = opens;
        fail_close = 1;
        status = 0xa5u;
        if (NvM_Init(&config, argv[2], &status) != ECU_ERR_NVM || status != 0xa5u ||
            opens != before) {
            return 3;
        }
        fail_close = 0;
        if (NvM_Init(&config, argv[2], &status) != ECU_OK || status != 0x50u ||
            NvM_Write(0x51u) != ECU_OK) {
            return 4;
        }
    } else {
        if (NvM_HostOpen(argv[2]) != NVM_HOST_OPEN_CREATED) {
            return 5;
        }
        fail_close = 1;
        before = opens;
        if (strcmp(argv[1], "reopen") == 0) {
            if (NvM_HostOpen(argv[2]) != NVM_HOST_OPEN_ERROR || opens != before) {
                return 6;
            }
        } else if (NvM_HostClose() != -1) {
            return 7;
        }
        if (closes != 1u || NvM_HostClose() != 0 || NvM_HostRead(&status, 1u) != -1) {
            return 8;
        }
        fail_close = 0;
        if (NvM_HostOpen(argv[2]) != NVM_HOST_OPEN_EXISTING) {
            return 9;
        }
    }
    return NvM_HostClose() == 0 ? 0 : 10;
}
