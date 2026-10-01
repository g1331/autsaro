#include "Os_Host.h"
BOOL Os_HostSetEvent(HANDLE event) { return SetEvent(event); }
