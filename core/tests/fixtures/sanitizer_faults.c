/* Deliberate faults that the Linux sanitizer lane must report (#129).
 * Sizes and shift amounts come from the command line, so the compiler
 * cannot reject them and only the instrumented run can. */
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static unsigned char frame[8];
static unsigned long shared_counter;

static void *increment(void *argument)
{
    unsigned long i;
    (void)argument;
    for (i = 0u; i < 100000u; ++i) {
        ++shared_counter;
    }
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
        pthread_t threads[2];
        (void)pthread_create(&threads[0], NULL, increment, NULL);
        (void)pthread_create(&threads[1], NULL, increment, NULL);
        (void)pthread_join(threads[0], NULL);
        (void)pthread_join(threads[1], NULL);
        printf("%lu\n", shared_counter);
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
