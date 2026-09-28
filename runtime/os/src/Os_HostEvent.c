#include "Os_Windows.h"
BOOL Os_HostSetEvent(HANDLE event) { return SetEvent(event); }
