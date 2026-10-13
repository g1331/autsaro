/** @file Private body for the generated per-channel SchM main wrapper.
 * One call consumes one configured period. Returned state is the resulting
 * state, not a second scheduler or host clock. Wrapper identity is generated.
 */
#ifndef COMM_INTERNAL_H
#define COMM_INTERNAL_H
#include "ComM.h"
ComM_StateType ComM_RunChannel(void);
#endif
