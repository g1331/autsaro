# OS

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

文档：`AUTOSAR_CP_SWS_OS.pdf`（`6ec1915808e8819c6552bcc69fe935d8664f7bf0b8f9eb55810377e79ff8ae44`）、`AUTOSAR_CP_RS_OS.pdf`；OSEK/VDX OS 2.2.3。目标等级为单核 SC1，须覆盖等级能力与最小容量，不缩减为参考配置实际使用的对象。

| 义务族 | 条款 | 要点 |
| --- | --- | --- |
| 一致性类与容量 | OSEK §3.2（p13–14）；`SWS_Os_00241`、Table 7.4（p129–131） | BCC1／BCC2／ECC1／ECC2；SC1 至少两个调度表、八个软件 Counter |
| 激活顺序与抢占恢复 | OSEK §4.3／4.5（p19–20） | 全部激活请求按优先级内请求顺序记录，不只记数量 |
| 终止／链式／错误返回 | OSEK §4.7、§13.2.3.2–3（p23／51–52）；R24-11 任务缺失结束条款（p74–75） | 合法调用不返回原应用帧；错误调用返回并保留源实例与目标状态 |
| 非抢占与内部资源 | OSEK §4.6、§8.7 | 逐任务属性、内部 ceiling 的释放／重新取得 |
| 事件 | OSEK §7、§13.5（p26–28／60–62） | 自有 mask；检查与进入 WAITING、Set 与 ready 转换同一原子域 |
| 资源 | OSEK §8；R24-11 §7.9.21（p105）；`SWS_Os_00850`（RES_SCHEDULER） | 固定 ceiling／LIFO；不能以 mutex 继承代替 |
| Counter／Alarm／ScheduleTable | R24-11 §7.3、§7.5–7.7、§8／10；`SWS_Os_00560` | 回绕、零偏移、同 tick 动作顺序、相对／绝对起点 |
| 栈 | §7.5；`SWS_Os_00067／00068／00396` | Task／Cat2 ISR 栈故障检测；无 ProtectionHook 时调用 ShutdownOS |
| 生命周期、ISR、错误、Hook | §7.1／8／10；OSEK §6 | StartOS／ShutdownOS、Extended Status、中断禁用配对、调用层级 |
| ARTI | §7.16／7.17；`SWS_Os_00858／00829／00836／00837` | 生成描述与实际对象一致；旧日志宏不替代 ARTI |
| 扩展功能 | `SWS_Os_00240`、§7.11 | 首版不承诺 SC2–4、时间／内存保护、多核或 IOC |
| RTE 调用的 OS 服务 | `OS00383／00392` | GetCounterValue／GetElapsedValue 的同步／重入属性 |

## 已知差距

- **OS**：完整 SC1 条款逐项证据、ARTI 适用性与 OSEK 引用版本（ISO 17356-3）全文未核对；Windows 实际交接运行未在每次变更后执行。
