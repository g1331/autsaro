---
title: '4.18 生成真实 dispatcher 使用的可重定位中断向量段'
type: 'feature'
created: '2026-09-30'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '47524ac51473328fdb82367239cefa8afdfdf8c7'
context: []
---

<frozen-after-approval reason="Epic4产品范围及按独立增量持续实施已获用户授权">

## Intent

固定 Win64 OS 的实际 dispatcher 仍使用端口内部默认存储的中断表，生成工程没有明确的可重定位向量段。完成 SWS_Os_00336 的产品生成行为，使链接后的专用段包含真实 dispatcher 读取和安装的完整32槽向量表。

## Boundaries & Constraints

保持固定 FreeRTOS 原件和 MIT 许可、唯一 backend、原子安装与访问、0/1内核保留槽、配置 Cat2 身体和 mailbox 所有权。新增受控生产补丁只改变表的存储绑定，保留全部调度、嵌套、屏蔽和资源语义。生成源码、构建清单及独立搬移构建一致；C99与MISRA编码指导适用。主机功能不推定 MCU 向量或硬实时能力。不更改 OS 路线，不增加 assurance 工件，不推送。此增量不关闭4.18的Memory Mapping、Counter服务端口和后续Epic故事。

## I/O & Edge-Case Matrix

| Given／When | Then |
| --- | --- |
| 合法生成工程；构建和搬移后构建 | 专用可重定位段包含完整32槽、Win64原生函数指针宽度的实际向量表；原始源码不变，交付构建不依赖仓库隐含文件 |
| 配置真实Cat2身体；启动、递送和嵌套 | 内核与配置回调安装在该表，实际dispatcher通过此表执行；11个已有入口／拒绝向量及26个嵌套向量保持独立预期 |
| 屏蔽、源控制、资源退出和mailbox入口 | 原有pending、层级、恢复和唯一所有权保持，非法替换仍拒绝 |
| 移除实际表绑定或专用段的编译副本 | 实际原生断言或链接段检查失败，不能以无消费者的附属表通过 |

</frozen-after-approval>

## Code Map

- runtime/os/src/Os_Vector.h、Os_Vector.c：新增私有Win64函数指针表，明确长度和可重定位段；复用既有GCC固定目标，不引入通用平台层。
- runtime/os/patches/0014-relocatable-interrupt-vectors.patch：将真实port.c的ulIsrHandler绑定至上述表；既有安装保护和互斥边界不变，原件不修改。
- runtime/os/src/Os_Backend.h：供受控port声明表；不改变公开Os.h ABI。
- scripts/epic4_os.py、runtime/os/tests/entry_bodies.c：构建新源、读取真实表和检查链接段；复用入口／嵌套／资源清理向量。
- core/src/integration/offline.rs、ecu.rs：交付新源和补丁、更新生成使用说明；runtime/ecu/build.ps1自动收集os/src/*.c并核对实际链接段。
- core/tests/support/epic4_ecu.rs：真实生成、搬移构建、外部消费者和源身份已有测试，追加实际段位置与大小断言。

## Tasks & Acceptance

- [x] 新增真实表及受控端口绑定，保持原件与现有访问边界。
- [x] 同步原生构建、生成源和交付构建，加入实际链接段检查。
- [x] 完成正向、旧绑定／无段负向验证及受影响回归。
- [x] BMad三路复核、记录实际结果，本地提交并继续父story。

## Implementation Notes

主代理负责实现；仓库允许的子代理只执行独立只读复核。没有用户可见选择或不可逆操作，按已授权Epic范围继续。

## Spec Change Log

## Review Triage Log

blind、edge-case、verification-gap三路只读复核均完成，无可定位发现或验证缺口，没有递延本增量问题。

## Verification

运行11入口、26嵌套、10Cat2清理及46生命周期原生向量；采用既有独立字面预期。实际编译副本分别恢复原内部表和去掉段属性，确认相关检查拒绝。核心真实生成／搬移构建与外部消费者测试检查链接段；运行Python单测、增量格式/C99及核心Clippy。结果写入本规格，测试中间文件仅用临时目录。

实际向量表的原生段为.os_vec、256字节、可写，唯一外部表定义位于该段；port读取与安装绑定真实表。6个正式exact入口各选1项通过：vector21.80s、entry7.40s、nested7.56s、cleanup7.44s、lifecycle7.12s、生成与搬移构建234.36s；内部11/26/10/46原生向量及两个真实编译副本按独立预期通过／被拒绝。旧绑定副本须确切以E_OS_STATE=7拒绝，不接受任意非零；最终正式exact强化复验23.02s通过。15 Python、核心Clippy、增量格式/C99通过。
