# Win64 Automotive OS runtime

固定FreeRTOS V11.3.1 commit `054e14f3397023aa83813a65aa065fc4597d481b`与GCC16.1.0 x64。内核原件、文件身份和MIT许可在third_party/freertos/，产品受控补丁在patches/。构建使用固定源码，不下载浮动版本；编译器身份由toolchain.json核对。

Os_TargetConfig静态配置汽车Task、资源、内部资源、Counter、Alarm、ScheduleTable、Hook和逻辑中断。唯一backend适配真实内核；FreeRTOS决定实际Running、抢占和上下文切换，汽车层维护完整激活请求FIFO、事件和资源契约，不建立另一套Task选择器。

StartOS成功不返回。Task正常终止／ChainTask通过实际原生入口移交；Task意外返回会报告MISSINGEND、恢复遗留资源和屏蔽，再完成激活。PreTask／PostTask对应真实Running转换；Shutdown不伪造PostTask。ErrorHook提供标准服务身份和准确参数快照，Hook内错误不递归、不覆盖外层快照。参考EXTENDED不变，显式STANDARD配置也受支持；两模式保留防御检查和非零结果报告，标准激活及Alarm告警保留。

Disable／Enable、嵌套All／OS配对按调用者保存和恢复状态。Cat1与Cat2有不同屏蔽语义；源操作改变实际pending／disabled位。嵌套ISR使用同一真实S线程的原生栈，返回恢复外层身份和资源；泄漏屏蔽／资源在Cat2退出清理。Os_IsrConfig.entries可选32槽void身体表，用OS_ISR_ENTRY(name)绑定；私有0／1、Cat1、无优先级及已配置input mailbox槽不得指定身体，配置身体不能被raw handler替换。

TASK、ISR和ALARMCALLBACK用于标准入口声明／定义；TASK与ISR分别用OS_TASK_ENTRY／OS_ISR_ENTRY绑定实际配置。CounterType为uint32，TickType及Rte_Os_Type.h的TimeInMicrosecondsType为uint64。所选生成Counter提供对应常量及单位转换宏；转换参数只求值一次，合法范围0..UINT32_MAX，整数秒向下截断。

时间由controlled_logical_ms逐毫秒请求／确认推进；软件Counter与内核tick分别保持模数，宿主看门狗不推进汽车时间。实际Waiting及输入／tick／输出确认决定提交完成。GetISRID读取真实ISR帧；单核ControlIdle支持IDLE_NO_HALT且省略CoreID检查。isOsStarted按R24-11的DRAFT定义表示进入过StartOS，而非Ready。

Windows物理栈由实际线程、保护页和保证区验证；S/C/D/T/B/I各角色的故障在健康控制路径不可逆关闭，故障线程不继续运行。上下文／资源建立失败有明确拒绝或关闭，不能伪装成功。此目标是Windows主机逻辑行为，不推定MCU电气层或硬实时性。

## 构建与测试

从仓库根目录运行原生行为测试，例如：

```powershell
python scripts/epic4_os.py --suite lifecycle
python scripts/epic4_os.py --suite stack
python scripts/epic4_os.py --suite interrupt-pairing
python scripts/epic4_os.py --suite nested-interrupts
python scripts/epic4_os.py --suite status-modes
cargo test --manifest-path core/Cargo.toml
```

脚本在临时目录构建并检查真实行为，失败返回非零，不写入共享报告。各suite由核心集成测试注册；更多用法见--help。开发、复核、验证结论及任务状态按安装的BMad，记录在对应story/spec。

当前后续产品工作包括实际生成中断向量段、Memory Mapping、Counter OsService、ARTI、适用模块描述及新目标交接；对应Epic4故事仍须实现和验证。
