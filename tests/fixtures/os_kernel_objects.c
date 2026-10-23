/** @file Independent C99 consumers of static kernel object types and queue storage. */
#include "Os.h"
#include "FreeRTOS.h"
#include "task.h"
#include "queue.h"
#include <stdio.h>

void StartupHook(void) {}
void ShutdownHook(StatusType reason) { (void)reason; }

int main(void)
{
    StaticTask_t first = {0};
    StaticTask_t second = {0};
    TaskHandle_t first_handle = &first;
    TaskHandle_t second_handle = &second;
    StackType_t first_stack[4] = {0};
    StackType_t second_stack[4] = {0};
    List_t list;
    ListItem_t *sentinel = &list.xListEnd;
    StaticQueue_t queue;
    QueueHandle_t typed_queue = &queue;
    uint32 storage[2] = {0};
    const uint32 first_value = 17u;
    const uint32 second_value = 29u;
    uint32 received = 0u;

    first.pxTopOfStack = &first_stack[1];
    second.pxTopOfStack = &second_stack[2];
    if ((pxTaskOsPortStack(first_handle) != &first_stack[1]) ||
        (pxTaskOsPortStack(second_handle) != &second_stack[2])) {
        return 2;
    }
    if (xQueueCreateStatic(2u, sizeof(uint32), (uint8 *)&storage, &queue) != typed_queue) {
        return 3;
    }
    if ((xQueueSend(typed_queue, &first_value, 0u) != pdPASS) ||
        (xQueueSend(typed_queue, &second_value, 0u) != pdPASS) ||
        (xQueueSend(typed_queue, &first_value, 0u) != errQUEUE_FULL)) {
        return 4;
    }
    if ((xQueuePeek(typed_queue, &received, 0u) != pdPASS) || (received != first_value) ||
        (xQueueReceive(typed_queue, &received, 0u) != pdPASS) || (received != first_value) ||
        (xQueueReceive(typed_queue, &received, 0u) != pdPASS) || (received != second_value) ||
        (xQueueReceive(typed_queue, &received, 0u) != errQUEUE_EMPTY)) {
        return 5;
    }
    if ((xQueueSend(typed_queue, &second_value, 0u) != pdPASS) ||
        (xQueueReset(typed_queue) != pdPASS) ||
        (xQueueReceive(typed_queue, &received, 0u) != errQUEUE_EMPTY)) {
        return 6;
    }
    vListInitialise(&list);
    if ((sentinel->pxNext != sentinel) || (sentinel->pxPrevious != sentinel) ||
        (list.uxNumberOfItems != 0u)) {
        return 7;
    }
    puts("static task and queue types; task stack identity; bounded FIFO/reset observed");
    return 0;
}
