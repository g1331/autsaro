/** Runs a libFuzzer driver once per input file named on the command line, so that the
 * committed corpus and reproducers replay in the ordinary native tests. FUZZ_ENTRY names the
 * driver's entry point. Each input is an exactly sized allocation, as under libFuzzer.
 */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

int FUZZ_ENTRY(const uint8_t *data, size_t size);

int main(int argc, char **argv)
{
    int i;
    for (i = 1; i < argc; ++i) {
        FILE *file = fopen(argv[i], "rb");
        long size;
        uint8_t *data;
        if ((file == NULL) || (fseek(file, 0, SEEK_END) != 0) || ((size = ftell(file)) < 0) ||
            (fseek(file, 0, SEEK_SET) != 0)) {
            (void)fprintf(stderr, "%s: cannot read\n", argv[i]);
            return 2;
        }
        data = malloc((size == 0) ? 1u : (size_t)size);
        if ((data == NULL) || (fread(data, 1u, (size_t)size, file) != (size_t)size)) {
            (void)fprintf(stderr, "%s: cannot read\n", argv[i]);
            return 2;
        }
        (void)fclose(file);
        (void)FUZZ_ENTRY(data, (size_t)size);
        free(data);
    }
    return 0;
}
