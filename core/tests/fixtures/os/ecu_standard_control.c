/* A moved STANDARD delivery must supply the selected header itself. */
#include "Os.h"
#if OS_STATUS_EXTENDED != 0
#error The delivered STANDARD configuration was replaced by Extended Status
#endif
#include "ecu_control.c"
