#ifndef _WIN32
#define _POSIX_C_SOURCE 200809L
#endif
#include "NvM_HostStorage.h"
#include <errno.h>
#include <stdio.h>
#ifdef _WIN32
#include <io.h>
#else
#include <unistd.h>
#endif

static FILE *host_storage;

NvMHostOpenResult NvM_HostOpen(const char *path) {
    NvMHostOpenResult result = NVM_HOST_OPEN_ERROR;
    NvM_HostClose();
    errno = 0;
    host_storage = fopen(path, "r+b");
    if (host_storage != NULL) {
        result = NVM_HOST_OPEN_EXISTING;
    } else if (errno == ENOENT) {
        host_storage = fopen(path, "w+b");
        if (host_storage != NULL) {
            result = NVM_HOST_OPEN_CREATED;
        }
    } else {
        /* Existing storage could not be opened. */
    }
    return result;
}

void NvM_HostClose(void) {
    if (host_storage != NULL) {
        (void)fclose(host_storage);
        host_storage = NULL;
    }
}

long NvM_HostLength(void) {
    long length = -1L;
    if ((host_storage != NULL) && (fseek(host_storage, 0L, SEEK_END) == 0)) {
        errno = 0;
        length = ftell(host_storage);
        if (errno != 0) {
            length = -1L;
        }
    }
    return length;
}

int NvM_HostRead(uint8_t *data, size_t length) {
    int result = -1;
    if ((host_storage != NULL) && (fseek(host_storage, 0L, SEEK_SET) == 0) &&
        (fread(data, 1u, length, host_storage) == length)) {
        result = 0;
    }
    return result;
}

int NvM_HostWrite(size_t offset, const uint8_t *data, size_t length) {
    int result = -1;
    if ((host_storage != NULL) && (fseek(host_storage, (long)offset, SEEK_SET) == 0) &&
        (fwrite(data, 1u, length, host_storage) == length) && (fflush(host_storage) == 0)) {
#ifdef _WIN32
        result = _commit(_fileno(host_storage));
#else
        result = fsync(fileno(host_storage));
#endif
    }
    return result;
}
