/* Test hooks appended to the sealed HostBatch main. After the Task consumes a
 * batch it waits, so the host thread confirms the resulting frame before that
 * epoch's cycle runs: the order a loaded host can produce at any time. */
int Ecu_TargetTestFailStage(unsigned stage) {
    if (stage == 12u) {
        Os_HostSleepMs(100u);
    }
    return 0;
}
void Ecu_TargetTestRetire(unsigned stage) { (void)stage; }
void Ecu_TargetTestShutdown(StatusType reason) { (void)reason; }
