/* Independent literal oracles; these expectations are not generated. */
#include "Os.h"
#include <stdio.h>
#ifndef EXPECTED_COUNTER_MAX
#define EXPECTED_COUNTER_MAX 65535u
#endif
#ifdef COUNTER_TIME_RENAMED
#define CONVERT_NS(ticks) OS_TICKS2NS_RenamedCounter(ticks)
#define CONVERT_US(ticks) OS_TICKS2US_RenamedCounter(ticks)
#define CONVERT_MS(ticks) OS_TICKS2MS_RenamedCounter(ticks)
#define CONVERT_SEC(ticks) OS_TICKS2SEC_RenamedCounter(ticks)
#define NAMED_MAX OSMAXALLOWEDVALUE_RenamedCounter
#define NAMED_BASE OSTICKSPERBASE_RenamedCounter
#define NAMED_MIN OSMINCYCLE_RenamedCounter
#define COUNTER_ID OS_COUNTER_ID_RenamedCounter
#define ID_MAX OSMAXALLOWEDVALUE_OS_COUNTER_ID_RenamedCounter
#define ID_BASE OSTICKSPERBASE_OS_COUNTER_ID_RenamedCounter
#define ID_MIN OSMINCYCLE_OS_COUNTER_ID_RenamedCounter
#else
#define CONVERT_NS(ticks) OS_TICKS2NS_SystemCounter(ticks)
#define CONVERT_US(ticks) OS_TICKS2US_SystemCounter(ticks)
#define CONVERT_MS(ticks) OS_TICKS2MS_SystemCounter(ticks)
#define CONVERT_SEC(ticks) OS_TICKS2SEC_SystemCounter(ticks)
#define NAMED_MAX OSMAXALLOWEDVALUE_SystemCounter
#define NAMED_BASE OSTICKSPERBASE_SystemCounter
#define NAMED_MIN OSMINCYCLE_SystemCounter
#define COUNTER_ID OS_COUNTER_ID_SystemCounter
#define ID_MAX OSMAXALLOWEDVALUE_OS_COUNTER_ID_SystemCounter
#define ID_BASE OSTICKSPERBASE_OS_COUNTER_ID_SystemCounter
#define ID_MIN OSMINCYCLE_OS_COUNTER_ID_SystemCounter
#endif

typedef struct {
    TickType ticks;
    PhysicalTimeType ns;
    PhysicalTimeType us;
    PhysicalTimeType ms;
    PhysicalTimeType seconds;
} TimeVector;

static const TimeVector vectors[] = {
    {0u, UINT64_C(0), UINT64_C(0), UINT64_C(0), UINT64_C(0)},
    {1u, UINT64_C(1000000), UINT64_C(1000), UINT64_C(1), UINT64_C(0)},
    {999u, UINT64_C(999000000), UINT64_C(999000), UINT64_C(999), UINT64_C(0)},
    {1000u, UINT64_C(1000000000), UINT64_C(1000000), UINT64_C(1000), UINT64_C(1)},
    {65535u, UINT64_C(65535000000), UINT64_C(65535000), UINT64_C(65535), UINT64_C(65)},
    {UINT32_MAX, UINT64_C(4294967295000000), UINT64_C(4294967295000), UINT64_C(4294967295),
     UINT64_C(4294967)},
};

int main(void) {
    size_t index;
    TickType ticks = 1000u;
    PhysicalTimeType converted;
    if ((NAMED_MAX != EXPECTED_COUNTER_MAX) || (NAMED_BASE != 1u) || (NAMED_MIN != 1u) ||
        (OSMAXALLOWEDVALUE != EXPECTED_COUNTER_MAX) || (OSTICKSPERBASE != 1u) ||
        (OSMINCYCLE != 1u) || (OSTICKDURATION != UINT64_C(1000000)) || (COUNTER_ID != 0u) ||
        (ID_MAX != EXPECTED_COUNTER_MAX) || (ID_BASE != 1u) || (ID_MIN != 1u)) {
        return 3;
    }
    for (index = 0u; index < (sizeof(vectors) / sizeof(vectors[0])); ++index) {
        if (CONVERT_NS(vectors[index].ticks) != vectors[index].ns ||
            CONVERT_US(vectors[index].ticks) != vectors[index].us ||
            CONVERT_MS(vectors[index].ticks) != vectors[index].ms ||
            CONVERT_SEC(vectors[index].ticks) != vectors[index].seconds) {
            return 1;
        }
    }
    converted = CONVERT_NS(ticks++);
    if (converted != UINT64_C(1000000000) || ticks != 1001u) {
        return 2;
    }
    converted = CONVERT_US(ticks++);
    if (converted != UINT64_C(1001000) || ticks != 1002u) {
        return 3;
    }
    converted = CONVERT_MS(ticks++);
    if (converted != UINT64_C(1002) || ticks != 1003u) {
        return 4;
    }
    converted = CONVERT_SEC(ticks++);
    if (converted != UINT64_C(1) || ticks != 1004u) {
        return 5;
    }
    if (2u * CONVERT_NS(3u + 4u) != UINT64_C(14000000) ||
        2u * CONVERT_US(3u + 4u) != UINT64_C(14000) || 2u * CONVERT_MS(3u + 4u) != UINT64_C(14) ||
        2u * CONVERT_SEC(500u + 500u) != UINT64_C(2)) {
        return 6;
    }
    puts("counter_time PASS: 24 literal unit values; 4 single evaluations; 4 expressions");
    return 0;
}
