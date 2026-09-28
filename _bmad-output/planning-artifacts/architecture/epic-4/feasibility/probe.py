"""Isolated FreeRTOS architecture probes. Does not integrate an OS into the product.
Run: python probe.py --kernel <verified extracted kernel directory> --repeat 3
No network download, GUI, production-runtime link or mutation of upstream source.
"""
import argparse
import difflib
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import time

CONFIG = r"""#ifndef PROBE_FREERTOS_CONFIG_H
#define PROBE_FREERTOS_CONFIG_H
#include <stdlib.h>
#define configUSE_PREEMPTION 1
#define configUSE_TIME_SLICING 0
#define configUSE_PORT_OPTIMISED_TASK_SELECTION 0
#define configUSE_IDLE_HOOK 0
#define configUSE_TICK_HOOK 0
#define configTICK_RATE_HZ 100
#define configMAX_PRIORITIES 10
#define configMINIMAL_STACK_SIZE 512
#define configMAX_TASK_NAME_LEN 12
#define configTICK_TYPE_WIDTH_IN_BITS TICK_TYPE_WIDTH_32_BITS
#define configSUPPORT_STATIC_ALLOCATION 1
#define configSUPPORT_DYNAMIC_ALLOCATION 0
#define configUSE_TIMERS 0
#define configUSE_MUTEXES 0
#define configUSE_TASK_NOTIFICATIONS 1
#define configTASK_NOTIFICATION_ARRAY_ENTRIES 1
#define configUSE_TRACE_FACILITY 1
#define configCHECK_FOR_STACK_OVERFLOW 0
#define configUSE_CO_ROUTINES 0
#define INCLUDE_vTaskSuspend 1
#define INCLUDE_vTaskPrioritySet 1
#define INCLUDE_uxTaskPriorityGet 1
#define INCLUDE_vTaskDelay 1
#define INCLUDE_vTaskDelete 0
#define INCLUDE_xTaskGetCurrentTaskHandle 1
#define INCLUDE_eTaskGetState 1
void probe_assert(const char *, int);
#define configASSERT(x) do { if (!(x)) probe_assert(__FILE__, __LINE__); } while (0)
#endif
"""
BASE_PROBE = r"""/* Isolated architecture experiments, not an AUTOSAR OS implementation. */
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <setjmp.h>
#include "FreeRTOS.h"
#include "task.h"
static StaticTask_t tcbs[5];
static StackType_t stacks[5][512];
static TaskHandle_t ctrl, a, b, h;
static char trace[128];
static unsigned n, completed, mode, entries;
static uintptr_t local_addr;
static jmp_buf restart;
static void log_char(char c) { configASSERT(n+1<sizeof(trace)); trace[n++]=c; }
void probe_assert(const char *f,int line) { fprintf(stderr,"ASSERT %s:%d\n",f,line); exit(99); }
void vApplicationGetIdleTaskMemory(StaticTask_t **t,StackType_t **s,configSTACK_DEPTH_TYPE *depth) { *t=&tcbs[4];*s=stacks[4];*depth=512; }
static void done(void) { if (++completed==2) vTaskResume(ctrl); vTaskSuspend(NULL); for(;;){} }
static void task_h(void *p) { (void)p; log_char('H'); vTaskSuspend(NULL); for(;;){} }
static void task_b(void *p) { (void)p; log_char('B'); done(); }
static void body(void) { volatile int fresh=7; log_char('A'); configASSERT(fresh==7); ++entries; fresh=99; longjmp(restart,1); log_char('X'); }
static void task_a(void *p) {
 (void)p;
 int local=0; local_addr=(uintptr_t)&local;
 if(mode==1 || mode==2) {
  log_char('A'); if(mode==2) vTaskPrioritySet(NULL,3);
  vTaskResume(h); log_char('a'); if(mode==2) vTaskPrioritySet(NULL,2); done();
 } else if(mode==3) {
  log_char('A'); vTaskResume(b); log_char('a'); done();
 } else if(mode==4) {
  uint32_t value; xTaskNotify(a,1,eSetBits);
  log_char(xTaskNotifyWait(0,0,&value,0)?'1':'0');
  log_char(xTaskNotifyWait(0,0,&value,0)?'1':'0'); done();
 } else if(mode==5) {
  log_char('A'); vTaskPrioritySet(NULL,6); vTaskResume(h); log_char('a');
  vTaskPrioritySet(NULL,2); log_char('z'); done();
 } else if(mode==6) {
  while(entries<3) { if(setjmp(restart)==0) body(); if(entries<3) taskYIELD(); }
  done();
 } else if(mode==7) {
  vTaskSuspendAll(); vTaskSuspend(NULL); /* Expected: forbidden pattern asserts. */
  xTaskResumeAll(); done();
 } else if(mode==8) {
  log_char(local_addr>=(uintptr_t)&stacks[0][0] && local_addr<(uintptr_t)&stacks[0][512]?'I':'O'); done();
 } else if(mode==9) {
  log_char('A'); vTaskSuspendAll(); vTaskResume(h); log_char('a');
  xTaskResumeAll(); log_char('z'); done();
 }
 for(;;){}
}
static void control(void *p) {
 (void)p; vTaskResume(a); if(mode!=3) vTaskResume(b);
 vTaskSuspend(NULL); vTaskEndScheduler(); for(;;){}
}
int main(int argc,char **argv) {
 mode=argc==2?(unsigned)atoi(argv[1]):1;
 a=xTaskCreateStatic(task_a,"A",512,NULL,2,stacks[0],&tcbs[0]); vTaskSuspend(a);
 b=xTaskCreateStatic(task_b,"B",512,NULL,2,stacks[1],&tcbs[1]); vTaskSuspend(b);
 h=xTaskCreateStatic(task_h,"H",512,NULL,5,stacks[2],&tcbs[2]); vTaskSuspend(h);
 ctrl=xTaskCreateStatic(control,"control",512,NULL,8,stacks[3],&tcbs[3]);
 vTaskStartScheduler();
 printf("mode=%u trace=%s entries=%u stack_outside=%u\n",mode,trace,entries,
   (unsigned)(local_addr<(uintptr_t)&stacks[0][0] || local_addr>=(uintptr_t)&stacks[0][512]));
 return 0;
}
"""
POLICY_PROBE = r"""/* Isolated architecture experiments, not an AUTOSAR OS implementation. */
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <setjmp.h>
#include "FreeRTOS.h"
#include "task.h"
static StaticTask_t tcbs[5];
static StackType_t stacks[5][512];
static TaskHandle_t ctrl, a, b, h;
static char trace[128];
static unsigned n, completed, mode, entries;
static uintptr_t local_addr;
static jmp_buf restart;
void probe_requeue_current(void);
void probe_transition(TaskHandle_t, BaseType_t);
static uint32_t event_bits;
static void log_char(char c) { configASSERT(n+1<sizeof(trace)); trace[n++]=c; }
void probe_assert(const char *f,int line) { fprintf(stderr,"ASSERT %s:%d\n",f,line); exit(99); }
void vApplicationGetIdleTaskMemory(StaticTask_t **t,StackType_t **s,configSTACK_DEPTH_TYPE *depth) { *t=&tcbs[4];*s=stacks[4];*depth=512; }
static void done(void) { if (++completed==2) vTaskResume(ctrl); vTaskSuspend(NULL); for(;;){} }
static void task_h(void *p) { (void)p; log_char('H'); vTaskSuspend(NULL); for(;;){} }
static void task_b(void *p) { (void)p; log_char('B'); if(mode==10) { vTaskResume(a); log_char('b'); } if(mode==14) { event_bits=1; vTaskResume(a); log_char('b'); } done(); }
static void body(void) { volatile int fresh=7; log_char('A'); configASSERT(fresh==7); ++entries; fresh=99; longjmp(restart,1); log_char('X'); }
static void task_a(void *p) {
 (void)p;
 int local=0; local_addr=(uintptr_t)&local;
 if(mode==10) {
  if(setjmp(restart)==0) { log_char('A'); ++entries; probe_transition(b,pdFALSE); longjmp(restart,1); log_char('X'); }
  else { log_char('C'); ++entries; done(); }
 } else if(mode==11) {
  if(setjmp(restart)==0) { log_char('A'); ++entries; probe_transition(a,pdTRUE); longjmp(restart,1); log_char('X'); }
  else { log_char('A'); ++entries; done(); }
 } else if(mode==13) {
  event_bits=1; log_char(event_bits&1?'1':'0'); log_char(event_bits&1?'1':'0'); done();
 } else if(mode==14) {
  log_char('W'); if(!(event_bits&1)) probe_transition(NULL,pdFALSE);
  log_char(event_bits&1?'E':'X'); log_char(event_bits&1?'e':'X'); done();
 } else if(mode==1 || mode==2) {
  log_char('A'); if(mode==2) vTaskPrioritySet(NULL,3);
  vTaskResume(h); log_char('a'); if(mode==2) vTaskPrioritySet(NULL,2); done();
 } else if(mode==3) {
  log_char('A'); vTaskResume(b); log_char('a'); done();
 } else if(mode==4) {
  uint32_t value; xTaskNotify(a,1,eSetBits);
  log_char(xTaskNotifyWait(0,0,&value,0)?'1':'0');
  log_char(xTaskNotifyWait(0,0,&value,0)?'1':'0'); done();
 } else if(mode==5) {
  log_char('A'); vTaskPrioritySet(NULL,6); vTaskResume(h); log_char('a');
  vTaskPrioritySet(NULL,2); log_char('z'); done();
 } else if(mode==6) {
  while(entries<3) { if(setjmp(restart)==0) body(); if(entries<3) probe_requeue_current(); }
  done();
 } else if(mode==7) {
  vTaskSuspendAll(); vTaskSuspend(NULL); /* Expected: forbidden pattern asserts. */
  xTaskResumeAll(); done();
 } else if(mode==8) {
  log_char(local_addr>=(uintptr_t)&stacks[0][0] && local_addr<(uintptr_t)&stacks[0][512]?'I':'O'); done();
 } else if(mode==9) {
  log_char('A'); vTaskSuspendAll(); vTaskResume(h); log_char('a');
  xTaskResumeAll(); log_char('z'); done();
 }
 for(;;){}
}
static void control(void *p) {
 (void)p; vTaskResume(a); if(mode!=3 && mode!=10) vTaskResume(b);
 vTaskSuspend(NULL); vTaskEndScheduler(); for(;;){}
}
int main(int argc,char **argv) {
 mode=argc==2?(unsigned)atoi(argv[1]):1;
 a=xTaskCreateStatic(task_a,"A",512,NULL,mode==14?5:2,stacks[0],&tcbs[0]); vTaskSuspend(a);
 b=xTaskCreateStatic(task_b,"B",512,NULL,mode==10?5:2,stacks[1],&tcbs[1]); vTaskSuspend(b);
 h=xTaskCreateStatic(task_h,"H",512,NULL,5,stacks[2],&tcbs[2]); vTaskSuspend(h);
 ctrl=xTaskCreateStatic(control,"control",512,NULL,8,stacks[3],&tcbs[3]);
 vTaskStartScheduler();
 printf("mode=%u trace=%s entries=%u stack_outside=%u\n",mode,trace,entries,
   (unsigned)(local_addr<(uintptr_t)&stacks[0][0] || local_addr>=(uintptr_t)&stacks[0][512]));
 return 0;
}
"""

BASE_EXPECTED = {1:"AHBa",2:"AHaB",3:"AaB",4:"10B",5:"AaHBz",6:"ABAA",8:"OB",9:"AaHBz"}
POLICY_EXPECTED = {1:"AHaB",2:"AHaB",3:"AaB",4:"10B",5:"AaHzB",6:"ABAA",8:"OB",9:"AaHzB",10:"ABbC",11:"ABA",13:"11B",14:"WBEeb"}

def run():
    ap = argparse.ArgumentParser()
    ap.add_argument("--kernel", type=Path, required=True)
    ap.add_argument("--repeat", type=int, default=3)
    ap.add_argument("--output", type=Path)
    args = ap.parse_args()
    if args.repeat < 1 or args.repeat > 10:
        ap.error("repeat must be 1..10")
    manifest = json.loads(Path(__file__).with_name("source-manifest.json").read_text(encoding="utf-8"))
    kernel = args.kernel.resolve()
    for name in ["LICENSE.md", "tasks.c", "queue.c", "portable/MSVC-MingW/port.c", "portable/MSVC-MingW/portmacro.h"]:
        if hashlib.sha256((kernel/name).read_bytes()).hexdigest() != manifest[name]:
            raise SystemExit("source hash mismatch: " + name)
    work = Path(tempfile.mkdtemp(prefix="autosar-freertos-probes-"))
    (work/"FreeRTOSConfig.h").write_text(CONFIG, encoding="utf-8")
    port = (kernel/"portable/MSVC-MingW/port.c").read_text(encoding="utf-8")
    assert port.count("REALTIME_PRIORITY_CLASS") == 1
    port_test = port.replace("REALTIME_PRIORITY_CLASS", "NORMAL_PRIORITY_CLASS")
    port_test = port_test.replace("THREAD_PRIORITY_TIME_CRITICAL", "THREAD_PRIORITY_ABOVE_NORMAL")
    port_test = port_test.replace("THREAD_PRIORITY_HIGHEST", "THREAD_PRIORITY_NORMAL")
    port_test = port_test.replace("portTASK_THREAD_PRIORITY                    THREAD_PRIORITY_ABOVE_NORMAL", "portTASK_THREAD_PRIORITY                    THREAD_PRIORITY_NORMAL")
    (work/"port-probe.c").write_text(port_test, encoding="utf-8")
    (work/"port-priority.patch").write_text("".join(difflib.unified_diff(port.splitlines(True),port_test.splitlines(True),fromfile="upstream/port.c",tofile="research/port.c")), encoding="utf-8")
    # Apply the exact reviewed experiment patch to a fresh copy, never upstream.
    patched = work/"policy-kernel"
    (patched/"portable").mkdir(parents=True)
    (patched/"tasks.c").write_text((kernel/"tasks.c").read_text(encoding="utf-8"), encoding="utf-8", newline="\n")
    patchfile = Path(__file__).with_name("tasks-policy-probe.patch").resolve()
    subprocess.run(["git", "apply", "--check", str(patchfile)], cwd=patched, check=True, capture_output=True)
    subprocess.run(["git", "apply", str(patchfile)], cwd=patched, check=True, capture_output=True)
    compiler = subprocess.run(["gcc", "--version"], text=True, capture_output=True, check=True).stdout.splitlines()[0]
    target = subprocess.run(["gcc", "-dumpmachine"], text=True, capture_output=True, check=True).stdout.strip()
    result = {"harness_sha256":hashlib.sha256(Path(__file__).read_bytes()).hexdigest(), "policy_patch_sha256":hashlib.sha256(patchfile.read_bytes()).hexdigest(), "source":manifest, "compiler":compiler,"compiler_target":target,"repeat":args.repeat,"port_priority_normalized":True,"builds":[],"runs":[],"scope":"research only; not SC1 or production integration"}
    for variant,code,expected in [("baseline",BASE_PROBE,BASE_EXPECTED),("policy",POLICY_PROBE,POLICY_EXPECTED)]:
        source = work/(variant+".c")
        source.write_text(code, encoding="utf-8")
        tasks = kernel/"tasks.c" if variant == "baseline" else patched/"tasks.c"
        exe = work/(variant+".exe")
        cmd = ["gcc","-std=c99","-O1","-Wall","-Wextra","-Werror","-I"+str(work),"-I"+str(kernel/"include"),"-I"+str(kernel/"portable/MSVC-MingW"),str(source),str(tasks),str(kernel/"list.c"),str(work/"port-probe.c"),"-lwinmm","-o",str(exe)]
        build = subprocess.run(cmd, capture_output=True, text=True, timeout=40)
        result["builds"].append({"variant":variant,"exit":build.returncode,"stderr":build.stderr,"flags":"-std=c99 -O1 -Wall -Wextra -Werror; static allocation; no time slicing; no timers/mutexes; NORMAL process priority"})
        if build.returncode:
            raise SystemExit(build.stderr)
        for repetition in range(1,args.repeat+1):
            for mode in sorted(set(expected)|{7}):
                started = time.monotonic()
                child = subprocess.run([str(exe),str(mode)], capture_output=True, text=True, timeout=4)
                expected_exit = 99 if mode == 7 else 0
                match = child.returncode == expected_exit
                if mode == 7:
                    match = match and "ASSERT" in child.stderr
                else:
                    match = match and ("trace="+expected[mode]+" ") in child.stdout
                result["runs"].append({"variant":variant,"repeat":repetition,"mode":mode,"exit":child.returncode,"stdout":child.stdout.strip(),"stderr":child.stderr.replace(str(kernel),"<kernel>").replace(str(patched),"<policy-kernel>").strip(),"seconds":round(time.monotonic()-started,3),"expected_observation":match})
                if not match:
                    raise SystemExit("unexpected observation: " + json.dumps(result["runs"][-1]))
            print(variant,"repeat",repetition,"expected observations reproduced",flush=True)
    result["all_expected_observations"] = all(x["expected_observation"] for x in result["runs"])
    output = args.output or work/"results.json"
    output.write_text(json.dumps(result,indent=2)+"\n", encoding="utf-8")
    print("RESULT",str(output),"runs",len(result["runs"]),flush=True)
    print("Temporary research build:",work,flush=True)

if __name__ == "__main__":
    run()
