#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include "Can.h"
#include "Ecu_Config.h"
#include "Ecu_Runtime.h"
#include "Os.h"
#include "Rte.h"

static int ParseDecimal(const char *text, uint64_t *value) {
    uint64_t parsed = 0;
    const unsigned char *cursor = (const unsigned char *)text;
    if (*cursor == '\0') {
        return 0;
    }
    while (*cursor != '\0') {
        unsigned digit;
        if (*cursor < '0' || *cursor > '9') {
            return 0;
        }
        digit = (unsigned)(*cursor - '0');
        if (parsed > (UINT64_MAX - digit) / 10u) {
            return 0;
        }
        parsed = parsed * 10u + digit;
        ++cursor;
    }
    *value = parsed;
    return 1;
}

static int ParseField(const char *text, uint64_t maximum, uint64_t *value) {
    return ParseDecimal(text, value) && *value <= maximum;
}

static int ParseHex(const char *text, uint8_t dlc, uint8_t bytes[8]) {
    size_t i;
    if (strlen(text) != (size_t)dlc * 2u) {
        return 0;
    }
    for (i = 0; i < dlc; ++i) {
        unsigned pair[2];
        unsigned k;
        for (k = 0; k < 2u; ++k) {
            unsigned char c = (unsigned char)text[i * 2u + k];
            if (c >= '0' && c <= '9') {
                pair[k] = c - '0';
            } else if (c >= 'A' && c <= 'F') {
                pair[k] = c - 'A' + 10u;
            } else {
                return 0;
            }
        }
        bytes[i] = (uint8_t)((pair[0] << 4u) | pair[1]);
    }
    return 1;
}

static EcuStatus EmitFrame(uint32_t id, uint8_t dlc, const uint8_t data[8]) {
    unsigned i;
    if (printf("X %" PRIu32 " %u ", id, (unsigned)dlc) < 0) {
        return ECU_ERR_IO;
    }
    for (i = 0; i < dlc; ++i) {
        if (printf("%02X", (unsigned)data[i]) < 0) {
            return ECU_ERR_IO;
        }
    }
    return putchar('\n') == EOF ? ECU_ERR_IO : ECU_OK;
}

static int Report(EcuStatus result) { return printf("E %s\n", Ecu_StatusName(result)) < 0 ? 3 : 0; }

int main(int argc, char **argv) {
    char line[256];
    EcuStatus result;
    const char *nvm_path = NULL;
    const char *security_key_path = NULL;
    const char *security_state_path = NULL;
    int arg;
    (void)setvbuf(stdout, NULL, _IONBF, 0);
    for (arg = 1; arg < argc; arg += 2) {
        if (arg + 1 >= argc || argv[arg + 1][0] == '\0') {
            (void)Report(ECU_ERR_CONFIG);
            return 1;
        }
        if (strcmp(argv[arg], "--nvm") == 0 && nvm_path == NULL) {
            nvm_path = argv[arg + 1];
        } else if (strcmp(argv[arg], "--security-key") == 0 && security_key_path == NULL) {
            security_key_path = argv[arg + 1];
        } else if (strcmp(argv[arg], "--security-state") == 0 && security_state_path == NULL) {
            security_state_path = argv[arg + 1];
        } else {
            (void)Report(ECU_ERR_CONFIG);
            return 1;
        }
    }
    if ((Ecu_Config.diagnostic != NULL && Ecu_Config.diagnostic->dtc != NULL) !=
            (nvm_path != NULL) ||
        (Ecu_Config.diagnostic != NULL && Ecu_Config.diagnostic->security_enabled != 0u) !=
            (security_key_path != NULL && security_state_path != NULL)) {
        (void)Report(ECU_ERR_CONFIG);
        return 1;
    }
    result = Ecu_Init(&Ecu_Config, EmitFrame, nvm_path, security_key_path, security_state_path);
    if (result != ECU_OK) {
        (void)Report(result);
        return 1;
    }
    while (fgets(line, sizeof(line), stdin) != NULL) {
        char *tokens[5];
        size_t count = 0;
        char *token;
        uint64_t number;
        if (strchr(line, '\n') == NULL && !feof(stdin)) {
            (void)printf("E PROTOCOL\n");
            return 2;
        }
        token = strtok(line, " \t\r\n");
        while (token != NULL && count < 5u) {
            tokens[count++] = token;
            token = strtok(NULL, " \t\r\n");
        }
        if (token != NULL || count == 0u) {
            goto malformed;
        }
        if (strcmp(tokens[0], "T") == 0 && count == 2u && ParseDecimal(tokens[1], &number)) {
            result = Os_Advance(number);
            if (result == ECU_ERR_TIME) {
                (void)Report(result);
                return 2;
            }
        } else if (strcmp(tokens[0], "R") == 0 && count == 4u) {
            uint64_t id;
            uint64_t dlc;
            uint8_t data[8] = {0};
            if (!ParseField(tokens[1], UINT32_MAX, &id) ||
                !ParseField(tokens[2], UINT8_MAX, &dlc)) {
                goto malformed;
            }
            if (dlc < 1u || dlc > 8u) {
                result = ECU_ERR_FRAME_DLC;
            } else if (!ParseHex(tokens[3], (uint8_t)dlc, data)) {
                goto malformed;
            } else {
                result = Can_Inject((uint32_t)id, (uint8_t)dlc, data, Os_Now());
            }
        } else if (strcmp(tokens[0], "S") == 0 && count == 3u) {
            uint64_t id;
            if (!ParseField(tokens[1], UINT16_MAX, &id) ||
                !ParseField(tokens[2], UINT32_MAX, &number)) {
                goto malformed;
            }
            result = Rte_WriteSignal((uint16_t)id, (uint32_t)number);
        } else if (strcmp(tokens[0], "G") == 0 && count == 2u) {
            uint32_t value;
            uint8_t valid;
            if (!ParseField(tokens[1], UINT16_MAX, &number)) {
                goto malformed;
            }
            result = Rte_ReadSignal((uint16_t)number, &value, &valid);
            if (result == ECU_OK &&
                printf("V %" PRIu64 " %" PRIu32 " %u\n", number, value, (unsigned)valid) < 0) {
                result = ECU_ERR_IO;
            }
        } else if (strcmp(tokens[0], "M") == 0 && count == 2u) {
            if (!ParseField(tokens[1], 2u, &number)) {
                goto malformed;
            }
            Can_SetMode((CanMode)number);
            result = ECU_OK;
        } else {
            goto malformed;
        }
        if (result != ECU_OK) {
            if (Report(result) != 0 || result == ECU_ERR_IO) {
                return 3;
            }
        }
        continue;
    malformed:
        (void)printf("E PROTOCOL\n");
        return 2;
    }
    return ferror(stdin) || ferror(stdout) ? 3 : 0;
}
