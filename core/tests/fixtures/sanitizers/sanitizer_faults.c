/* Deliberate faults that the Linux sanitizer lane must report (#129).
 * Sizes and shift amounts come from the command line, so the compiler
 * cannot reject them and only the instrumented run can. */
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static unsigned char frame[8];
/* Volatile preserves the deliberate non-atomic access, not synchronization. */
static volatile unsigned long shared_value;
static int published;

static void *observe(void *argument)
{
    (void)argument;
    /* Acquire orders the value only when main uses the clean release store. */
    while (__atomic_load_n(&published, __ATOMIC_ACQUIRE) == 0) {
    }
    printf("%lu\n", shared_value);
    return NULL;
}

int main(int argc, char **argv)
{
    const unsigned char payload[16] = {0u};
    if (argc != 3) {
        return 2;
    }
    const int amount = atoi(argv[2]);
    if (strcmp(argv[1], "race") == 0) {
        pthread_t thread;
        if (pthread_create(&thread, NULL, observe, NULL) != 0) {
            return 3;
        }
        shared_value = 1u;
        if (amount == 0) {
            /* Arrange the read after the write without a happens-before edge. */
            __atomic_store_n(&published, 1, __ATOMIC_RELAXED);
        } else {
            __atomic_store_n(&published, 1, __ATOMIC_RELEASE);
        }
        if (pthread_join(thread, NULL) != 0) {
            return 4;
        }
    } else if (strcmp(argv[1], "overflow") == 0) {
        /* A received length larger than the fixed static frame. */
        memcpy(frame, payload, (size_t)amount);
        printf("%u\n", frame[0]);
    } else if (strcmp(argv[1], "undefined") == 0) {
        printf("%d\n", 1 << amount);
    } else {
        return 2;
    }
    return 0;
}
