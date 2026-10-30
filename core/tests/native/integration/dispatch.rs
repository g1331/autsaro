//! Observable schedule order and rejection paths of the emitted cycle body.
use super::*;
use std::process::Command;
use std::time::Duration;

#[test]
fn generated_cycle_preserves_event_groups_order_and_flush_failures() {
    let plan = build(&inputs()).unwrap();
    let files = prepared_pipeline_files_for_target(&plan, tooling::native_target());
    let source = files
        .iter()
        .find(|(name, _)| name == "generated/src/Ecu_RuntimeConfig.c")
        .unwrap();
    let source = std::str::from_utf8(&source.1).unwrap();
    let cycle = source
        .split_once("RTE_Ecu_CODE Std_ReturnType Ecu_StackRunCycle(EventMaskType events)")
        .unwrap()
        .1
        .split_once("RTE_Ecu_CODE void Ecu_StackRetire")
        .unwrap()
        .0;
    let scratch = Scratch::new();
    let callbacks = [
        "ComM_MainFunction_Host",
        "Can_MainFunction_Wakeup",
        "CanTp_MainFunction",
        "Com_MainFunctionRx_Rx",
        "Ingress_Periodic",
        "Process_Periodic",
        "Observe_Periodic",
        "Com_MainFunctionTx_Tx",
        "Dcm_MainFunction",
        "Can_MainFunction_Read",
        "Can_MainFunction_Write",
        "Can_MainFunction_Mode",
        "Can_MainFunction_BusOff",
    ];
    let mut harness = String::from(
        r#"#include <string.h>
typedef unsigned int EventMaskType;
typedef unsigned char Std_ReturnType;
#define E_OK 0u
#define E_NOT_OK 1u
#define ECU_OK 0
#define FALSE 0
static char trace[32];
static unsigned int count, flushes, fail_flush;
static int owner = 1, timing = 1;
static int Ecu_TargetIsOwner(void) { return owner; }
static int Ecu_SchMTimingActive(void) { return timing; }
static int Can_HostFlush(void) {
    trace[count++] = '|';
    return ++flushes == fail_flush ? 1 : 0;
}
"#,
    );
    for (index, callback) in callbacks.iter().enumerate() {
        harness.push_str(&format!(
            "void {callback}(void);\nvoid {callback}(void) {{ trace[count++] = '{}'; }}\n",
            char::from(b'a' + index as u8)
        ));
    }
    harness.push_str("static Std_ReturnType Ecu_StackRunCycle(EventMaskType events)\n");
    harness.push_str(cycle);
    harness.push_str(
        r#"static int check(unsigned int events, unsigned int failure, int own, int ready,
                 unsigned int expected, const char *sequence) {
    memset(trace, 0, sizeof(trace)); count = 0u; flushes = 0u;
    fail_flush = failure; owner = own; timing = ready;
    return Ecu_StackRunCycle(events) == expected && strcmp(trace, sequence) == 0;
}
int main(void) {
    if (!check(0u, 0u, 1, 1, E_OK, "||")) { return 1; }
    if (!check(1u, 0u, 1, 1, E_OK, "abcd|ijklm|")) { return 2; }
    if (!check(2u, 0u, 1, 1, E_OK, "efgh||")) { return 3; }
    if (!check(3u, 0u, 1, 1, E_OK, "abcdefgh|ijklm|")) { return 4; }
    if (!check(4u, 0u, 1, 1, E_OK, "||")) { return 5; }
    if (!check(3u, 1u, 1, 1, E_NOT_OK, "abcdefgh|")) { return 6; }
    if (!check(3u, 2u, 1, 1, E_NOT_OK, "abcdefgh|ijklm|")) { return 7; }
    if (!check(3u, 0u, 0, 1, E_NOT_OK, "")) { return 8; }
    if (!check(3u, 0u, 1, 0, E_NOT_OK, "")) { return 9; }
    return 0;
}
"#,
    );
    for cpp in [false, true] {
        let path = scratch.0.join(if cpp { "cycle.cpp" } else { "cycle.c" });
        std::fs::write(&path, &harness).unwrap();
        let binary = tooling::native_binary(&scratch.0, if cpp { "cycle-cpp" } else { "cycle-c" });
        let mut command = tooling::c_compiler();
        command.args(["-Wall", "-Wextra", "-Werror", "-pedantic"]);
        command.arg(if cpp { "-std=c++11" } else { "-std=c99" });
        command.arg(&path).arg("-o").arg(&binary);
        let result = tooling::run_public_command(
            &mut command,
            &scratch.0,
            "cycle-compile",
            Duration::from_secs(60),
        );
        assert!(result.status.success(), "{result:?}");
        let result = tooling::run_public_command(
            &mut Command::new(&binary),
            &scratch.0,
            "cycle-run",
            Duration::from_secs(30),
        );
        assert!(result.status.success(), "{result:?}");
    }
}
