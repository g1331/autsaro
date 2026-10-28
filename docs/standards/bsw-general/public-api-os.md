# OS 标准服务的链接诊断

这些记录只处置实际公开服务的 R8.7，不宣称完成模块符合性。原始源位置按各 profile 的 source seal 保留。
表中3项仍未闭合，不援引 ADR 0023 添加源码 MR12。既有已批准项保持原记录，不重复登记。

## 接口与替代边界

下表列明固定 R24-11 SpecificationsARXML 中的服务定义、Available via，或被继承的 OSEK 2.2.3 具体服务章节。保留当前真实签名和公开头绑定；所选程序没有额外 TU 调用时仍承认 R8.7 Advisory 违反，不把公开名称本身当成使用证据。
直接删除或仅把原定义改为文件内 static 会撤销该对外服务。宏／header inline 替代须同时满足原文、标准头、既有 C 链接与 ARTI／模块代码放置合同，不能仅以同名包装宣称等价；本批独立复核须检查各条的实际可用条件，条件外项不进入已完成处置。
风险为公共可见面；原有参数、状态、上下文、独占区与资源错误行为保持，以既有模块 C99 合同、OS suite 和外部 C 消费者验证。规范、配置可用性、签名、调用方或 ABI 变化时重评。

| 规则 | 精确范围 | 固定原文及公开合同 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `isOsStarted` | SWS_Os_91034；Os.h。保留所选实现的公开 C 服务；具体条件见本节。 | 未闭合：外部链接的不可替代性未证明 |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `TerminateTask` | OSEK OS 2.2.3 §13.2.3.2；Syntax:            StatusType TerminateTask ( void )；Conformance:       BCC1, BCC2, ECC1, ECC2。保留所选实现的公开 C 服务；具体条件见本节。 | 未闭合：外部链接的不可替代性未证明 |
| MISRA C:2012 Rule 8.7 | `runtime/os/src/Os_ArtiServices.c` 中 `NextScheduleTable` | SWS_Os_00191；Os.h。保留所选实现的公开 C 服务；具体条件见本节。 | 未闭合：外部链接的不可替代性未证明 |

独立复核指出，BSWGeneral 允许适当的宏／inline 实现；OS Programming Language、GetCoreID 与 SWS_Os_91034 也允许或建议宏形式。现有 ABI 清单和函数式测试不能替代规范必要性。后续须证明具体替代不成立，或完成保留全部可观察行为的实现调整，或取得明确的新项目处置；不能仅凭标准服务存在将本表转为批准记录。

`Arti_GetVersionInfo` 已按 BSWGeneral §7.1.10 的类型检查 inline 形式整改，保留具名参数的原型形式和函数指针调用。函数体移入 Arti.h，继续使用相同 Interlocked 原子错误写入；版本字段独立变异消费者同步实际实现 owner，不再保留原外部定义的 R8.7 待决项。

## 受控 C99 的头内服务入口

38个既有纯包装已由 Os_ArtiServices.c 迁入 Os.h 的原 CODE 映射区间，保留具名参数和全部函数体操作；其中29个原表位置以实际源码整改承接，待本批统一验证。不再提供这些入口的原外部定义；函数指针调用仍可用，地址属于各调用TU，不承诺跨TU地址相等。Os_Service.h只归并38个头内入口需要的既有实现声明和6个trace声明，不新增转发函数、状态或使用点。标准允许形式与固定配置依据为OS §4.6.2、BSWGeneral §7.1.10。

TerminateTask还读取原生栈角色和当前Task trace状态；引入完整栈布局会把FreeRTOS配置类型带入公开头，因此本批保留原外部实现。isOsStarted、NextScheduleTable也保留原名字、参数和命名基线；三项仍须分别闭合R8.7，不以本次迁移取得保留批准。错误、资源／上下文、嵌套、退出和trace控制沿现有实现，ErrorHook R18.6状态不变。

正常MemoryMap消费者通过真正头内SetEvent及函数指针观察 enter、entry、implementation、capture、return、leave 顺序；六种删除反例须先编译成功，再由专用退出4拒绝。段属性删除反例及全START／STOP结构继续保留；局部函数使用既有local_names机制核对真实ELF／PE段绑定，不再用同名公开桩定义代替服务。
