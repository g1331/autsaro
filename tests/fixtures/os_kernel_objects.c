/** @file Independent C99 consumers of static kernel object types and queue storage. */
#include "Os.h"
#include "FreeRTOS.h"
#include "task.h"
#include "queue.h"
#include <stdio.h>
#if defined(OS_QUEUE_POINTER_TESTS) && (OS_QUEUE_POINTER_TESTS == 1)
#include <stdint.h>
#include <stdlib.h>

int8_t *Os_QueueTestPrevious(int8_t *head, int8_t *cursor, size_t size)
{
    const uintptr_t begin = (uintptr_t)head;
    const uintptr_t current = (uintptr_t)cursor;
    if ((current < begin) || ((current - begin) < size)) {
        (void)fprintf(
            stderr, "QUEUE_BEFORE_HEAD size=%zu offset=%zu\n", size, (size_t)(current - begin));
        exit(78);
    }
    return cursor - size;
}
#endif

void StartupHook(void) {}
void ShutdownHook(StatusType reason) { (void)reason; }

static int queue_state(QueueHandle_t queue, UBaseType_t used, UBaseType_t capacity)
{
    const UBaseType_t actual_used = uxQueueMessagesWaitingFromISR(queue);
    const UBaseType_t actual_space = uxQueueSpacesAvailable(queue);
    const BaseType_t actual_empty = xQueueIsQueueEmptyFromISR(queue);
    const BaseType_t actual_full = xQueueIsQueueFullFromISR(queue);
    return ((actual_used == used) && (actual_space == (capacity - used)) &&
            (actual_empty == ((used == 0u) ? pdTRUE : pdFALSE)) &&
            (actual_full == ((used == capacity) ? pdTRUE : pdFALSE)))
               ? 1
               : 0;
}

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
    if (queue_state(typed_queue, 0u, 2u) == 0) {
        return 21;
    }
    if ((xQueueSend(typed_queue, &first_value, 0u) != pdPASS) ||
        (xQueueSend(typed_queue, &second_value, 0u) != pdPASS) ||
        (xQueueSend(typed_queue, &first_value, 0u) != errQUEUE_FULL)) {
        return 4;
    }
    if (queue_state(typed_queue, 2u, 2u) == 0) {
        return 22;
    }
    if ((xQueuePeek(typed_queue, &received, 0u) != pdPASS) || (received != first_value) ||
        (xQueueReceive(typed_queue, &received, 0u) != pdPASS) || (received != first_value) ||
        (xQueueReceive(typed_queue, &received, 0u) != pdPASS) || (received != second_value) ||
        (xQueueReceive(typed_queue, &received, 0u) != errQUEUE_EMPTY)) {
        return 5;
    }
    if (queue_state(typed_queue, 0u, 2u) == 0) {
        return 23;
    }
    if ((xQueueSend(typed_queue, &second_value, 0u) != pdPASS) ||
        (xQueueReset(typed_queue) != pdPASS) ||
        (xQueueReceive(typed_queue, &received, 0u) != errQUEUE_EMPTY)) {
        return 6;
    }
    for (unsigned round = 0u; round < 3u; ++round) {
        uint32 front = 0u;
        uint32 back = 0u;
        if ((xQueueSendToFront(typed_queue, &first_value, 0u) != pdPASS) ||
            (xQueueSendToFront(typed_queue, &second_value, 0u) != pdPASS) ||
            (xQueueReceive(typed_queue, &front, 0u) != pdPASS) ||
            (xQueueReceive(typed_queue, &back, 0u) != pdPASS)) {
            return 8;
        }
        if ((front != second_value) || (back != first_value)) {
            (void)fprintf(stderr,
                          "QUEUE_FRONT_ORDER round=%u front=%u back=%u\n",
                          round,
                          (unsigned)front,
                          (unsigned)back);
            return 9;
        }
    }
    {
        StaticQueue_t single;
        uint32 cell = 0u;
        QueueHandle_t one = xQueueCreateStatic(1u, sizeof(cell), (uint8 *)&cell, &single);
        if (one == NULL) {
            return 10;
        }
        for (unsigned round = 0u; round < 3u; ++round) {
            if ((xQueueSendToFront(one, &first_value, 0u) != pdPASS) ||
                (xQueueOverwrite(one, &second_value) != pdPASS) ||
                (xQueuePeek(one, &received, 0u) != pdPASS) || (received != second_value) ||
                (xQueueReceive(one, &received, 0u) != pdPASS) || (received != second_value) ||
                (xQueueReceive(one, &received, 0u) != errQUEUE_EMPTY)) {
                return 11;
            }
        }
    }
    {
        StaticQueue_t empty_items;
        QueueHandle_t zero = xQueueCreateStatic(2u, 0u, NULL, &empty_items);
        if ((zero == NULL) || (xQueueSendToFront(zero, NULL, 0u) != pdPASS) ||
            (xQueueSend(zero, NULL, 0u) != pdPASS) ||
            (xQueueSendToFront(zero, NULL, 0u) != errQUEUE_FULL) ||
            (xQueueReceive(zero, NULL, 0u) != pdPASS) ||
            (xQueueReceive(zero, NULL, 0u) != pdPASS) ||
            (xQueueReceive(zero, NULL, 0u) != errQUEUE_EMPTY)) {
            return 12;
        }
    }
    vListInitialise(&list);
    if ((sentinel->pxNext != sentinel) || (sentinel->pxPrevious != sentinel) ||
        (list.uxNumberOfItems != 0u)) {
        return 7;
    }
    puts("static task and queue types; task stack identity; bounded FIFO/front/overwrite/reset "
         "observed");
    return 0;
}
