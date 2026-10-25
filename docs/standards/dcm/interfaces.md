# Dcm：公开接口与数据类型

本篇属于 [Dcm](README.md)。条款来自固定 R24-11 原始 SpecificationsARXML；标记仅作用于同一格中紧随其后的编号。行为测试的 `Verifies:` 与这里的处置共同核算。

`DataServices_{Data}.ReadData` 保留标准 `Std_ReturnType` 返回和 `E_NOT_OK=1` possible error。应用应返回 E_OK；错误声明保留 RTE 传输失败语义。没有 application error 的普通 C/S 操作仍为 void。数据数组名称来自实际 DcmDspData，源实现类型可以改名；标准数组别名及错误宏参与名字冲突检查。

C/S 表内的写入、条件检查、缩放及异步操作逐项受配置约束；提供同步 ReadData 不等于接纳同一标准接口中的全部可选操作。声明型 reset／DTC／security mode group 保留标准模式名，只有会话和每连接 authentication 模式由主线发布。SchM 立即模式接口在自身独占区内串行化，不以 ECU owner 身份拒绝 Dcm 合法调用上下文。

| 对象／条件 | 处置 |
| --- | --- |
| 周期诊断发送与确认 | 不适用（配置：ECUC_Dcm_00735）：SWS_Dcm_01092 |
| SecurityAccess 与持久化 attempt counter | 不适用（配置：ECUC_Dcm_00759）：SWS_Dcm_00685、01028–01029、01041、01151–01153、01159、91003–91004 |
| Authentication、角色、白名单与异步加密 | 不适用（配置：ECUC_Dcm_01151）：SWS_Dcm_91068–91069、91072–91073、91076–91077 |
| Secure coding 与 BndM | 不适用（配置：ECUC_Dcm_01245）：SWS_Dcm_91088 |
| 制造商请求通知 | 不适用（配置：ECUC_Dcm_00681）：SWS_Dcm_01039 |
| 供应商请求通知 | 不适用（配置：ECUC_Dcm_00816）：SWS_Dcm_01042 |
| 复合／嵌套 DID 数据 | 不适用（配置：ECUC_Dcm_01231）：SWS_Dcm_91118 |
| Sender/Receiver、NvData 与 ECU signal 数据访问 | 不适用（配置：ECUC_Dcm_00713）：SWS_Dcm_00687、91057–91061 |
| OpStatus／pending 异步处理 | 不适用（配置：ECUC_Dcm_00713）：SWS_Dcm_91015 |
| Bootloader 与重启恢复交互 | 不适用（配置：ECUC_Dcm_00815）：SWS_Dcm_00543–00544、00987–00988 |
| 可选 VIN 预读与 GetVin | 不适用（配置：ECUC_Dcm_00984）：SWS_Dcm_00950 |
| DID range | 不适用（配置：ECUC_Dcm_00937）：SWS_Dcm_00769、00803–00805、01012、01034、01138、01271 |
| VersionInfo API | 不适用（配置：ECUC_Dcm_00821）：SWS_Dcm_00065 |
| Dem DTC 通知与 ResponseOnEvent | 不适用（配置：ECUC_Dcm_00735）：SWS_Dcm_00614 |
| 内存访问与 upload/download | 不适用（配置：ECUC_Dcm_00735）：SWS_Dcm_00539–00540、00754–00756、91065、91070–91071、91084 |
| 文件传输 | 不适用（配置：ECUC_Dcm_01034）：SWS_Dcm_91064、91066、91078–91083、91085 |
| 非 ReadData 数据操作与 I/O control | 不适用（配置：ECUC_Dcm_00735）：SWS_Dcm_00794、00802、01285–01286、01290、01305–01309、01312–01316、01320、91007–91009、91050、91056、91087、91089、91092、91095、91098–91104 |
| 动态长度 ReadDataLength | 不适用（配置：ECUC_Dcm_00985）：SWS_Dcm_00796、91010、91096–91097 |
| ConditionCheckRead | 不适用（配置：ECUC_Dcm_00955）：SWS_Dcm_00797、91011 |
| ReadScalingInformation | 不适用（配置：ECUC_Dcm_00735）：SWS_Dcm_00798、01017、91012、91093–91094 |
| VehicleInformation | 不适用（配置：ECUC_Dcm_00630）：SWS_Dcm_00688、01013、01037、91014 |
| RoutineControl | 不适用（配置：ECUC_Dcm_00640）：SWS_Dcm_00690、01018–01027、01040、01203–01204、91013、91016–91018、91040–91049、91052–91055、91105–91107、91119–91124 |
| OBD RequestControl | 不适用（配置：ECUC_Dcm_00637）：SWS_Dcm_00691、01014–01015、01038、01338 |
| 请求 Indication／Confirmation 回调 | 不适用（配置：ECUC_Dcm_00681、ECUC_Dcm_00816）：SWS_Dcm_00694、01341–01342 |
| DTC 服务、内存选择与 ControlDTCSetting | 不适用（配置：ECUC_Dcm_00735）：SWS_Dcm_01270 |
| CommunicationControl | 不适用（配置：ECUC_Dcm_00900）：SWS_Dcm_91025–91026、91036 |
| 外置诊断服务函数 | 不适用（配置：ECUC_Dcm_00777）：SWS_Dcm_00763–00764 |
| ReadData 异步／FNC proxy | 不适用（配置：ECUC_Dcm_00713）：SWS_Dcm_91005–91006、91090–91091 |
| PID DataServices port | 不适用（配置：ECUC_Dcm_00626）：SWS_Dcm_01035 |
| S/R DataServices port | 不适用（配置：ECUC_Dcm_00713）：SWS_Dcm_01031、01310–01311 |
| Imported types | 已核对（源码：Dcm.h 引入 Std_Types.h／ComStack_Types.h，Dcm.c 不使用所列未选功能的类型）：SWS_Dcm_00333 |
| Dcm_ConfigType | 已核对（Dcm.h 声明 Dcm_ConfigType，生成的 dcm_config 经 Dcm_Init 传入）：SWS_Dcm_00982 |
| Dcm_GetSecurityLevel | 已核对（恒回 E_OK；dcm_contract.c main 断言 Init 前出参保持、Init 后为 0）：SWS_Dcm_00338 |
| Dcm_GetSesCtrlType | 已核对（恒回 E_OK；dcm_contract.c main／sessions_and_keepalive 断言会话值）：SWS_Dcm_00339 |
| Dcm_StartOfReception | 已核对（dcm_contract.c admission_and_copy：OK／E_NOT_OK／E_OVFL 与失败时出参保持）：SWS_Dcm_00094 |
| Dcm_CopyRxData | 已核对（dcm_contract.c admission_and_copy：SduLength 0 查询余量，失败时出参保持）：SWS_Dcm_00556 |
| Dcm_TpRxIndication | 已核对（dcm_contract.c admission_and_copy：E_NOT_OK 丢弃，E_OK 发布请求）：SWS_Dcm_00093 |
| Dcm_CopyTxData | 已核对（diagnostic_contract.c：CONFPENDING／DATACONF／DATARETRY 与 E_BUSY）：SWS_Dcm_00092 |
| Dcm_TpTxConfirmation | 已核对（dcm_contract.c sessions_and_keepalive：E_OK 提交会话，E_NOT_OK 丢弃）：SWS_Dcm_00351 |
| Dcm_ComM_NoComModeEntered | 已核对（dcm_contract.c gates_and_reentrancy：NoCom 拒收，未配置通道被忽略）：SWS_Dcm_00356 |
| Dcm_ComM_SilentComModeEntered | 已核对（dcm_contract.c gates_and_reentrancy：Silent 可收不发）：SWS_Dcm_00358 |
| Dcm_ComM_FullComModeEntered | 已核对（dcm_contract.c gates_and_reentrancy：Full 放行发送）：SWS_Dcm_00360 |
| Dcm_MainFunction | 已核对（SchM_Dcm.h 声明，生成的 Ecu_RuntimeConfig.c 按配置周期调用）：SWS_Dcm_00053 |
| Mandatory interfaces | 已核对（源码：Dcm.c 调用 ComM_DCM_ActiveDiagnostic／InactiveDiagnostic 与 PduR_DcmTransmit）：SWS_Dcm_91001 |
| Optional interfaces | 已核对（源码：Dcm.c 不调用任何可选接口，它们只服务于未选功能）：SWS_Dcm_91002 |
| Dcm_SecLevelType | 已核对（Rte_Dcm_Type.h：uint8，DCM_SEC_LEV_LOCKED 为 0）：SWS_Dcm_00977 |
| Dcm_SesCtrlType | 已核对（Rte_Dcm_Type.h：uint8，DCM_*_SESSION 取值 1–4）：SWS_Dcm_00978 |
| DCMServices | 差距 #187：SWS_Dcm_00698、01030 |
| Dcm_DiagnosticSessionControlModeSwitchInterface | 已核对（composition.rs 校验接口与 P-port；diagnostics.rs dcm_service_mode_source_requires_the_fixed_standard_contract 变异拒绝）：SWS_Dcm_91020、91033 |
| Dcm_EcuResetModeSwitchInterface | 已核对（源码：composition.rs 按 DCM_MODES 逐项校验接口与 P-port，计划要求六个端口齐全）：SWS_Dcm_91022、91034 |
| Dcm_ModeRapidPowerShutDownModeSwitchInterface | 已核对（源码：composition.rs 按 DCM_MODES 逐项校验接口与 P-port）：SWS_Dcm_91024、91035 |
| Dcm_ControlDTCSettingModeSwitchInterface | 已核对（源码：composition.rs 按 DCM_MODES 逐项校验接口与 P-port）：SWS_Dcm_91028、91037 |
| Dcm_SecurityAccessModeSwitchInterface | 已核对（composition.rs 校验接口与 P-port；diagnostics.rs 删除端口得 SERVICE_TYPE_CONFLICT）：SWS_Dcm_91032、91039 |
| Dcm_AuthenticationStateMSI | 已核对（diagnostics.rs：端口名随连接名生成并经 SchM 符号核对）：SWS_Dcm_91075 |
| DcmDiagnosticSessionControl | 已核对（Rte_Dcm_Type.h：模式 0–3、转换值 255、初始 DEFAULT；diagnostics.rs 变异拒绝）：SWS_Dcm_91019 |
| DcmEcuReset | 已核对（Rte_Dcm_Type.h：七个模式 0–6，转换值 255）：SWS_Dcm_91021 |
| DcmModeRapidPowerShutDown | 已核对（Rte_Dcm_Type.h：模式 0–1，转换值 255）：SWS_Dcm_91023 |
| DcmControlDTCSetting | 已核对（Rte_Dcm_Type.h：模式 0–1，转换值 255）：SWS_Dcm_91027 |
| DcmSecurityAccess | 已核对（Rte_Dcm_Type.h：仅 DCM_SEC_LEV_LOCKED，其余模式随 SecurityRow 增加）：SWS_Dcm_91031 |
| DcmAuthenticationState | 已核对（Rte_Dcm_Type.h：每连接两态与转换值 255；diagnostics.rs 变异拒绝）：SWS_Dcm_91067 |
| Dcm_AuthenticationStateModeSwitchInterface | 已核对（composition.rs 按连接名生成接口；diagnostics.rs 校验）：SWS_Dcm_91074 |
| 周期／RoE status | 不适用（配置：ECUC_Dcm_00735）：SWS_Dcm_00976 |
| CommunicationControl 类型 | 不适用（配置：ECUC_Dcm_00900）：SWS_Dcm_00981 |
| ReadMemory 类型 | 不适用（配置：ECUC_Dcm_00735）：SWS_Dcm_00985 |
| WriteMemory 类型 | 不适用（配置：ECUC_Dcm_00735）：SWS_Dcm_00986 |
| primitive DID 类型 | 不适用（配置：ECUC_Dcm_00985）：SWS_Dcm_91062 |
| 外置诊断服务请求数组 | 不适用（配置：ECUC_Dcm_00777）：SWS_Dcm_01165 |

## C99 位域与实际类型

`SWS_Dcm_00992` 的原表（R24-11 页279，已查看渲染表）规定结构体内两个 bit：`reqType` 位于 LSB+0，`suppressPosResponse` 位于 LSB+1；表中没有强制指定 C 位域的整数基类型。当前 `Dcm.h` 用两个具名、宽度为1的 `uint8` 字段表达它们，`Platform_Types.h` 经 stdint.h 将 uint8 定义为所选 GCC x64 的无符号8位整数。

R6.1（Required，MISRA 主文页58／PDF66）在 C99 中允许实现支持的其他显式有符号／无符号整数位域类型，且允许 typedef；AMD2.41–43 仅把该措辞扩展为 C99 and later 并调整示例。所选 [GCC 13.3](https://gcc.gnu.org/onlinedocs/gcc-13.3.0/gcc/Structures-unions-enumerations-and-bit-fields-implementation.html) 与 [GCC 16.1](https://gcc.gnu.org/onlinedocs/gcc-16.1.0/gcc/Structures-unions-enumerations-and-bit-fields-implementation.html) 明确支持其他整数位域类型。因此，这两个 uint8 位域满足所选 C99 实现下的 R6.1 类型条件。R6.2（Required，主文页59／PDF67，AMD2.44–45）要求单 bit 具名位域不采用有符号类型；这两个字段均为无符号。R6.3（Required，AMD3.15 页8–9／PDF16–17）禁止直接声明为 union 成员；这里的直接所属类型为 struct，未依赖其 union 内部子对象例外。

位序另由实际 ABI 和消费者核查，不由 R6.1 的类型许可推定。`dcm_contract.c` 分别检查两种字段组合的低两位；Linux GCC13.3 的普通 native 入口通过，同一严格编译参数与实际生成类型头的对照也通过。只在测试副本交换字段声明，编译仍成功，但消费者在具名赋值后的 LSB 位置断言失败。这验证观察点能拒绝布局变化；Windows 此处依据同一声明及所选 GCC 文档作源码判断，未将 Linux 实验写成 Windows 执行。

## 逻辑右操作数的调用闭包

R13.5（Required，主文页113–114／PDF121–122；TC1 Appendix J 页14／PDF20）按函数可能的持久副作用判断，不能因调用实参已知而豁免。当前 `Dcm.c` 的逻辑右操作数没有 native／SchM 调用、volatile 读取或应用调用：`Dcm_Init` 条件内的 `Dcm_IsAvailability` 只对值参数做位运算与取模；`Dcm_Available` 只调用此前者并读取执行者持有的不可变配置。其余逻辑项读取普通标量、请求／响应边界或执行者私有 work。实际 ReadData 回调位于独立条件中；会进入 SchM 的 `Dcm_Current` 也单独求值，不能把它们当成无副作用函数移入逻辑 RHS。本结论只处置当前 Dcm 源码中的这些表达式，回调实现及其他模块继续按其实际源码核查。

负响应的六个既有值直接消费生成的标准NRC常量，不再维护同值的私有DCM_NRC_*表。已对照SWS_Dcm_00980原表PDF365：DCM_E_GENERALREJECT／SERVICENOTSUPPORTED／SUBFUNCTIONNOTSUPPORTED／INCORRECTMESSAGELENGTHORINVALIDFORMAT／RESPONSETOOLONG／REQUESTOUTOFRANGE分别为0x10／11／12／13／14／31。Dcm.h原有Rte_Dcm_Type.h包含链、Dcm_Negative的uint8参数、响应字节／长度、各拒绝优先级及ReadData调用边界均保持；这是对R2.5未使用公共宏的真实消费者整理，不扩充服务或改变ISO处置。
