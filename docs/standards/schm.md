# SchM（BSW Scheduler）

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。它逐条处置 `AUTOSAR_CP_SWS_RTE.pdf` 中 BSW Scheduler 的生命周期与独占区条目（§6.5、§8.5、§8.6）；RTE 其余条目仍在 [RTE 与 SchM](rte.md) 中按义务区域索引，由 RTE 整改处置。

SchM 由生成器输出：生命周期在 `SchM_Main.c`，声明在 `Rte_Main.h`，各模块的独占区声明在 `SchM_<Mip>.h`，区映射由主机适配 `Can_HostLock.c` 提供（[ADR 0018](../adr/0018-per-module-exclusive-areas-without-cross-module-locking.md)）。生成的启动器 `Ecu_StackBootstrap`／`Ecu_StackRetire` 在 StartupHook／ShutdownHook 中承担 EcuM 的调用方角色。

| 义务 | 处置 |
| --- | --- |
| 生命周期 API 的签名与总是生成 | 已核对（生成样本：`Rte_Main.h` 声明、`SchM_Main.c` 定义四个服务，任何配置都输出；`schm_contract.c` 按声明编译链接）：SWS_Rte_91170、91171、91172、91173、07271、04547、04550、07275 |
| 有限执行时间 | 已核对（源码：`SchM_Main.c` 的四个服务只做条件判断与赋值，没有循环或等待）：SWS_Rte_07273、04548、04551、07277 |
| RTE 初始化前不激活 runnable | 已核对（源码与测试：`Ecu_StackRunCycle` 在 `Ecu_SchMTimingActive` 为假时拒绝；`SchM_StartTiming` 只在 `Rte_Start` 成功之后调用；`bootstrap_stage_failure_closes_without_output_and_retires_started_stages` 中第 8 阶段之前失败时没有任何输出）：SWS_Rte_07580 |
| 后台事件与经 SchM 的 BSW 客户端／服务端 | 已核对（配置：BSW-BACKGROUND-EVENT 与 BSW-OPERATION-INVOKED-EVENT 由 `bsw_background_and_operation_invoked_events_are_refused_at_their_source` 在源事件处拒绝，生成工程只有 BswTimingEvent 激活的实体）：SWS_Rte_07584、04552 |
| Enter／Exit 的命名与生成 | 已核对（生成样本：每个模块的 `SchM_<Mip>.h` 为 BSWMD 中唯一的 ExclusiveArea 声明 `SchM_Enter_<Mip>_<区名>`／`SchM_Exit_<Mip>_<区名>`，前缀取 BswModuleDescription 的短名，不带 vendor 中缀）：SWS_Rte_07250、07251、07253、07254 |
| 只由声明 canEnter 的实体使用 | 已核对（BSWMD：`non_entering_functions` 只对模块源码中不进入本模块区的函数省略 canEnter，生成的包装函数保留；`prepared_bsw_entries_describe_actual_call_mechanisms_and_implementation_policy` 的 canEnter 断言）：SWS_Rte_CONSTR_09046、SWS_Rte_CONSTR_09048 |
| 调用方不嵌套同一区 | 已核对（ADR 0018：模块不同时持有两个区；主机映射检测嵌套并终止，`test_host_boundaries.py` 的 `nest-strict`／`nest-same`）：SWS_Rte_CONSTR_09047 |
| SchM_Init 之前可用 Enter／Exit | 已核对（源码：`Can_HostAreaEnter` 经 `InitOnceExecuteOnce`／`pthread_once` 自行初始化区锁，不依赖 `SchM_Init`）：SWS_Rte_07578 |
| 不同区逆序嵌套；Task、Cat1、Cat2 上下文 | 差距 #202：SWS_Rte_07252、07579 |
| EcuM 调用次序 | 已核对（源码：`Ecu_StackBootstrap` 在 StartupHook 中先调用 `SchM_Init`／`SchM_Start`，最后调用 `SchM_StartTiming`；`Ecu_StackRetire` 在 ShutdownHook 中先 `Rte_Stop`，最后 `SchM_Deinit`；`SchM_Init` 每进程只接受一次）：SWS_Rte_CONSTR_09055、SWS_Rte_CONSTR_09056、SWS_Rte_CONSTR_09057 |
| SchM_GetVersionInfo | 不适用（配置：ECUC_Rte_09062=true）：SWS_Rte_91174、07278、07279、07280、07281 |

生命周期的拒绝与状态变化由 `generated_schm_lifecycle_refuses_misordered_repeated_and_foreign_calls`（`core/tests/fixtures/bsw-contracts/schm_contract.c`）验证：生命周期阶段之外、NULL、他分区配置、重复 Init、乱序 Start／StartTiming 与非关闭阶段的 Deinit 都不改变状态。

## 差距

- **SchM_GetVersionInfo**：RteBswGeneral 与其必填 RteSchMVersionInfoApi 已进入内置定义，所选配置为 false；参数处拒绝 true 的证据在 `integration/rte.rs`。不生成无条件版本 API。R24-11 RTE p954 要求返回的 build version 同时依赖生成器版本与输入配置，不能单用 Cargo、头中的实现版本或 BSWMD 的 DOC-REVISION；启用 API 前须与 #59 的真实身份及 build-version 方案一起闭合。
- **嵌套与中断上下文**：见 #202。
