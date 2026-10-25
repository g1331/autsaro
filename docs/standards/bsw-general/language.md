# BSW源码语言边界

本篇承接[BSW General](README.md)，记录所选C99交付源码的词法与初始化判断；逐条规范处置和原始分析状态仍分别核算。

## 注释与字面量的实际词法边界

R3.1（Required，主文印刷45／PDF53）在C99下禁止注释内的新注释起始序列；AMD4.39–40的例外1允许scheme://path形式的URI，例外2仍允许//出现在真正的//行注释中。FreeRTOS许可URI适用例外，Doxygen块内的C示例//不适用；@code／@verbatim不改变C词法。三个实际包含的采用头task.h／queue.h／timers.h分别有247／174／218处此类位置。受控0021补丁仅将对应23／15／10个Doxygen块转换为C99的///形式，保留说明、示例、标签、物理行数及全部非注释token，上游原件与许可URI不变。该正常修复不使用新偏离，原始checker未报告不作为满足依据。

R3.2为Required，本仓库C99配置适用（印刷46／PDF54；AMD2.27追加C11，AMD2.31删除说明性Note）。上述48块没有反斜杠换行，转换后的行注释也没有该构造。实际宏及Os_Hooks／commented_defines的续行仍位于预处理指令或完整块注释中，不能仅凭反斜杠换行判R3.2违反。新注释形式不改变此前的宏拼接语义。

R4.1（Required，印刷46–47／PDF54–55；AMD2.32未放宽终结条件）要求数值转义在另一个转义或当前literal结束处终结，不能仅靠后续非数字字符停止。已核冻结自有C／头、模板、四份Linux生成语料及双目标补丁内核中的实际数值转义均为字符literal末端的\0。Task名字虽经JSON序列化形成C字符串，正常入口先按grammar／rules将SHORT-NAME限定为ASCII identifier，不会把引号、反斜杠或控制字符送入该生成点。R4.2（Advisory，印刷47／PDF55）按替换发生在注释和条件处理前的顺序核对九种trigraph，已核原始文本未发现；生成注释没有通用自由文本转义层，当前实际调用方为固定说明、受约束名字或类型化数值，不扩张到任意用户应用或未来输入。

上述构造结论、源身份、生成入口与Windows语料补核集中PR #371；正常生成／编译／实际分析和独立复核按各自head记录。它们不改变原始passed/error/not_assessed及严格比较拒绝，也不构成全部221项或完整平台符合性。

## 初始化层级与实际求值

R9.1为Mandatory，R9.2–R9.5、R13.1为Required，当前C99适用（主文印刷75–81、108／PDF83–89、116）。AMD4.47–48明确禁止designator／位置项重复初始化同一子对象；AMD4.56只将C11 atomic初始化交给R9.7，AMD4.58为R9.2补充R9.6引用。R9.1仍按每个元素／成员及实际成功／失败路径判断，不由聚合布局或零诊断销项。

四份switch-emitter工程中，Can载荷、CanIf错误槽、Os_Error来源与Os_Time等待句柄均给足数组元素；running、increments、向量表及内核计数数组使用允许的{0}。xYieldPendings的pdFALSE元素数对应实际单核。生成tasks[]内部.id是结构成员designator；实际数组designator仅为.priorities内部的[OS_INPUT_INTERRUPT]，容量由Os_IsrConfigType显式声明，单次指定、其余零初始化，未混入覆盖它的位置项。函数地址／OS_TASK_ENTRY名字及sizeof(storage)不执行回调或改变存储，等待列表读取普通HANDLE。此结论仅到上述初始化及宏闭包，不外推SDK、多核或任意用户应用；完整逐处依据留PR #371。
