#ifndef AUTOSAR_EPIC4_OS_CFG_H
#define AUTOSAR_EPIC4_OS_CFG_H
/* The fixed reference ECU selects both standard error-access switches.
 * Native configuration tests compile each independent off/on combination. */
#ifndef OS_USE_GET_SERVICE_ID
#define OS_USE_GET_SERVICE_ID 1
#endif
#ifndef OS_USE_PARAMETER_ACCESS
#define OS_USE_PARAMETER_ACCESS 1
#endif
#if ((OS_USE_GET_SERVICE_ID != 0) && (OS_USE_GET_SERVICE_ID != 1)) ||                              \
    ((OS_USE_PARAMETER_ACCESS != 0) && (OS_USE_PARAMETER_ACCESS != 1))
#error Invalid OS error-access configuration
#endif
#endif
