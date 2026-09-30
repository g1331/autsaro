#include "Os_Arti.h"
#define main nested_main
#include "nested_interrupts.c"
#undef main

static void arti_check(int accepted) {
    if (accepted == 0) {
        fprintf(stderr, "ARTI consumer assertion failed after actor shutdown\n");
        ExitProcess(7u);
    }
}
void Os_ArtiTestShutdownObserved(void) {
    ISRType stack[32];
    unsigned used = 0u, starts = 0u, returns = 0u;
    unsigned service_queries = 0u, hook_errors = 0u;
    for (long i = 0L; i < Arti_EventCount; ++i) {
        const Arti_Event *event = &Arti_Events[i];
        arti_check(event->published == 1L && event->instance_parameter == 0u);
        arti_check(strcmp(event->instance, "Os") == 0);
        if (strcmp(event->class_name, "AR_CP_OS_CAT2ISR") == 0) {
            const ISRType id = (ISRType)event->event_parameter;
            arti_check(event->event_parameter >= 2u && event->event_parameter < 32u);
            arti_check((Os_Config->category1_isrs & (UINT32_C(1) << id)) == 0u);
            arti_check(strcmp(event->context, "NOSUSP") == 0);
            if (strcmp(event->event, "OsCat2Isr_Start") == 0) {
                arti_check(used < 32u);
                stack[used++] = id;
                ++starts;
            } else {
                arti_check(strcmp(event->event, "OsCat2Isr_Stop") == 0 && used != 0u);
                arti_check(stack[--used] == id);
                ++returns;
            }
        } else if (strcmp(event->event, "OsServiceCall_GetISRID_Return") == 0) {
            if (event->event_parameter != INVALID_ISR) {
                arti_check(event->event_parameter == 4u || event->event_parameter == 6u ||
                           event->event_parameter == 20u || is("maximum"));
                ++service_queries;
            }
            arti_check(event->status_valid == 1u && event->service_status == E_OK);
        } else if (strcmp(event->event, "OsHook_ErrorHook_Start") == 0) {
            arti_check(event->event_parameter == 3u || event->event_parameter == 5u ||
                       event->event_parameter == 6u || event->event_parameter == 9u);
            ++hook_errors;
        }
    }
    arti_check(used == 0u && starts == returns && starts > 0u && service_queries > 0u);
    arti_check(hook_errors == errors && Arti_EventsDropped == 0L);
    if (is("maximum")) {
        arti_check(starts == 30u);
    }
    printf("arti_nested scenario=%s balanced=%u queries=%u errors=%u dropped=0\n", scenario, starts,
           service_queries, hook_errors);
}
int main(int argc, char **argv) { return nested_main(argc, argv); }
