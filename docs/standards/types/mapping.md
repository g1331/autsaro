# 首次映射布局

自有六头的后续整改将 Arti.h、Os_Stack.h 与 Os_HostLinux.h 的条件类型移到
Arti_Types.h、Os_StackTypes.h 与 Os_HostLinuxTypes.h。原头在任何 C 声明前
开始首次完整映射，Linux 的 inline 定义与外部声明共用 CODE 区；类型头本身没有映射请求。
三个 SchM 头把 C++ 的 C linkage 放在每个声明前，语言条件只定义所属头的 linkage 宏，
映射区内没有条件指令。对应声明消费者去除这些属性后仍取得原来的返回与参数类型。
这消除了六处首次包含的前置 C 构造；后续 STOP、不同 memory class 及所需重入包含仍按
[MemMap 类别记录](../memmap/README.md#memmap-类别偏离)处理。

验证通过正常结构检查及交付消费者进行：重复包含三个 SchM 头的 C++ 程序链接独立 C
provider，分别调用 CAN 轮询、Dcm 独占区和 BswM 主函数并观察三个结果；删除任一 C linkage
会在链接时暴露符号不匹配。原子别名 ArtiAtomic32 改为符合命名约定的 Arti_Atomic32Type，已修的 clang-tidy 基线项删除；
Arti_Event／Arti_AddressCapture 保留在 Arti.h，成员、顺序、限定符和公开服务签名不变。新增头随资产清单交付；
集成方重新生成并整体重编。固定布局检查也要求这些头的首次 MemMap 包含之前只有预处理与注释。
