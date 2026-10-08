#ifndef NVM_HOST_STORAGE_H
#define NVM_HOST_STORAGE_H

#include <stddef.h>
#include <stdint.h>

typedef enum {
    NVM_HOST_OPEN_ERROR = 0,
    NVM_HOST_OPEN_EXISTING = 1,
    NVM_HOST_OPEN_CREATED = 2
} NvMHostOpenResult;

NvMHostOpenResult NvM_HostOpen(const char *path);
/* Returns 0 after closing (or if already closed), -1 on close failure. */
int NvM_HostClose(void);
long NvM_HostLength(void);
int NvM_HostRead(uint8_t *data, size_t length);
int NvM_HostWrite(size_t offset, const uint8_t *data, size_t length);

#endif
