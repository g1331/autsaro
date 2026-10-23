/** @file ARTI Category 2 ISR start and stop events stay balanced under nesting. */
#include "Os_Arti.h"
#define main nested_main
#include "nested_interrupts.c"
#undef main

static void arti_failure(const char *invariant,
                         unsigned line,
                         long cursor,
                         const ISRType *stack,
                         unsigned used,
                         unsigned starts,
                         unsigned returns,
                         unsigned service_queries,
                         unsigned hook_errors)
{
    fprintf(stderr,
            "ARTI consumer assertion failed after actor shutdown: %s:%u invariant=%s\n"
            "arti_snapshot scenario=%s cursor=%ld count=%ld dropped=%ld development_error=%ld "
            "used=%u starts=%u returns=%u queries=%u hook_errors=%u\n"
            "nested_snapshot children=%u grandchildren=%u helpers=%u errors=%u depth=%u "
            "maximum=%u trace=%s parent_state=%u\n",
            __FILE__,
            line,
            invariant,
            scenario,
            cursor,
            (long)Arti_EventCount,
            (long)Arti_EventsDropped,
            (long)Arti_DevelopmentError,
            used,
            starts,
            returns,
            service_queries,
            hook_errors,
            children,
            grandchildren,
            helpers,
            errors,
            depth,
            maximum,
            trace,
            (unsigned)parent_state);
    for (unsigned j = 0u; j < used && j < 32u; ++j) {
        fprintf(stderr, "arti_stack index=%u isr=%u\n", j, (unsigned)stack[j]);
    }
    for (unsigned j = 0u; j < errors && j < 8u; ++j) {
        fprintf(stderr,
                "nested_error isr=%u status=%u service=%u\n",
                error_isrs[j],
                (unsigned)error_codes[j],
                (unsigned)error_services[j]);
    }
    /* Actors are stopped; an unpublished reservation has no readable payload. */
    for (long j = 0L; j < Arti_EventCount && j < (long)ARTI_EVENT_CAPACITY; ++j) {
        const Arti_Event *event = &Arti_Events[j];
        fprintf(stderr, "arti_record index=%ld published=%ld", j, (long)event->published);
        if (event->published == 1L) {
            fprintf(stderr,
                    " context=%s class=%s instance=%s event=%s instance_parameter=%u "
                    "event_parameter=%u status_valid=%u service_status=%u",
                    event->context,
                    event->class_name,
                    event->instance,
                    event->event,
                    (unsigned)event->instance_parameter,
                    (unsigned)event->event_parameter,
                    (unsigned)event->status_valid,
                    (unsigned)event->service_status);
        }
        fputc('\n', stderr);
    }
    ExitProcess(7u);
}
#define arti_check(condition)                                                                      \
    do {                                                                                           \
        if (!(condition)) {                                                                        \
            arti_failure(#condition,                                                               \
                         __LINE__,                                                                 \
                         i,                                                                        \
                         stack,                                                                    \
                         used,                                                                     \
                         starts,                                                                   \
                         returns,                                                                  \
                         service_queries,                                                          \
                         hook_errors);                                                             \
        }                                                                                          \
    } while (0)
void Os_ArtiTestShutdownObserved(void)
{
    ISRType stack[32];
    unsigned used = 0u, starts = 0u, returns = 0u;
    unsigned service_queries = 0u, hook_errors = 0u;
    long i;
    for (i = 0L; i < Arti_EventCount; ++i) {
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
    printf("arti_nested scenario=%s balanced=%u queries=%u errors=%u dropped=0\n",
           scenario,
           starts,
           service_queries,
           hook_errors);
    arti_check(fflush(stdout) == 0);
}
int main(int argc, char **argv) { return nested_main(argc, argv); }
