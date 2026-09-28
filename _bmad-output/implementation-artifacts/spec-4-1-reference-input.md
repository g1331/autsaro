---
title: '4.1 原创参考输入与可定位的拒绝样例'
type: 'feature'
created: '2026-09-28'
status: 'done'
route: 'dispatch'
review_loop_iteration: 0
baseline_commit: '663aab48e4f02844f31a57468850f2ccdc16b1d7'
story_key: '4-1-建立原创参考输入与可定位的拒绝样例'
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

建立 epic4-win64-sr-cs-v1 的原创多文件输入基线，供 W0/W2 与后续集成共同验收。内容遵守最终 INTEGRATION-CONTRACT §1/2/6；官方 XSD/MOD 仅为本地外部校验资料。用户已授权逐 story 开发、验证、独立复核和本地提交，无逐条确认。

## Boundaries & Constraints

单 ECU_EXTRACT/ReferenceEcu、单不可重入 EchoApplication、显式未排队 uint32 S/R、同实例同步 ReadData，标准类型、映射、通信、ECUC 和所选真实 BSW 描述闭合。保留无关有效对象及文件原始字节。未来 OS/RTE 生产者明确列为后续要求。此次不实现新输入解析/生成，不用旧导入器替代语义预期，不复制官方原件，不推送，不降低 W3/SC1 出口。

## I/O & Edge-Case Matrix

| 输入 | 独立预期 | 定位 |
| --- | --- | --- |
| 原创正例 | 每文件 R24-11 XSD pass；完整引用闭合、固定值与契约一致 | 文件角色、完整对象路径、摘要 |
| 目标不唯一/断链/DEST 错 | 语义拒绝；结构合法性分别实测 | 文件、对象、类别 |
| 方向/长度/类型/周期不一致 | 语义拒绝，不补默认映射 | 同上 |
| 多实例/重入/未选变体 | 剖面拒绝 | 同上 |
| 缺 BSW/服务使用端 | 闭包拒绝，不声称接口存在 | 同上 |
| 排队/隐式/mode/transformer | 所需能力超出剖面，拒绝 | 同上 |
| 非法 XML 结构 | XSD fail，与语义反例分列 | XSD diagnostic |
| 缺官方 XSD/MOD | not_run/阻断，不跳过计 pass | 缺依赖名称 |

</frozen-after-approval>

## Code Map

- core/src/schema.rs：现有 libxml、本地 schema ZIP 与 no-net 校验。
- core/tests/end_to_end.rs：独立行为入口；增加 epic4_reference_input_baseline。
- core/tests/fixtures/epic4/：新增原创多文件正例、独立语义预期和负例目录。
- scripts/epic4_input.py：样例完整性/引用/参数与外部定义验证；不作为 4.10 的产品解析器。
- runtime/include/、runtime/src/：BSW 描述的真实声明/实现来源；runtime/os/ 为已通过的新 OS 基础。

## Tasks & Acceptance

- [x] 创建独立多文件类型/SWC、Extract/通信、ECUC 与原创 BSW 描述，补全标准必要关系。
- [x] 记录文件角色、原始摘要、官方外部定义摘要/来源、未来生产者及无关保留对象。
- [x] 固定独立语义预期；逐项制作结构与语义负例，并记录文件/对象/拒绝类别。
- [x] 注册并实际执行恰一个 epic4_reference_input_baseline；缺依赖明确失败。
- [x] 执行增量全门、独立三视角复核，修复可确认发现，同步 sprint 后本地提交。

Given 合法外部 XSD/MOD，When 校验原创正例，Then XSD 通过且契约值及引用一致，原始文件边界与 SHA 可复验。

Given 每个独立反例，When 核对该副本，Then 结构 pass/fail 与语义应拒绝分别记录，拒绝有文件/完整对象路径/类别。

Given 现有 BSW 与未来生产者，When 核对描述，Then 已有符号对应实际源码，尚未实现项明确属于后续 story，不冒称当前能力。

## Implementation Notes

XSD ZIP SHA-256 为 9db3ab1d2ec4db7cc8ff09f1259ff93a7a5945a9500d4cd3ea4a7090f2a25766；MOD ZIP 为 df1e3bc992e49de6e14e5c1a679d7ce7ca90d2450d66186cea0b4a0f1f6555fb。本机 lxml 5.4.0 可辅助查看 grammar；验收仍从官方 XSD 实际验证。

正式 Rust 入口选中并执行恰一个测试通过；七个正例文件、18 个反例、原始摘要及外部定义引用已校验。17 个当前 BSW 入口的正式 return/arguments 与真实头文件/实现核对；未来 RTE/组件实现单列。17 个反例 XSD pass 后语义拒绝，1 个结构错误 XSD fail 后语义 not_run。证据为 docs/assurance/evidence/epic4/reference-input.json。

最终全门通过：核心 3 单元/61 集成无跳过、UI lint/build、桌面 build/clippy、Python 16、质量及 assurance 7。BSW 精确签名、周期 Tx 映射、独立预期及两项复核修复均包含在最终测试范围中。

## Review Log

- Blind：未发现可确认可操作问题。
- Verification gap：清单未约束实际文件集合。主代理在临时副本加入 XSD 非法的未列 extra.arxml，复现旧审计接受。已对目录全文件集合与清单精确比对，并在 Rust 对实际正例 ARXML 集合单独比对，保证全部进入 XSD。新增回归测试保护该真实缺陷。
- Edge：本地对象可覆盖官方 MOD 同路径定义。主代理复现 local/external 相同路径被接受，已在合并前拒绝路径交集；新增真实审计调用验证拒绝，不放宽固定外部定义身份。
- 两项修复的定向 Python 2 测试和恰一个正式 Rust 入口均通过。两位独立复核者分别确认原发现已关闭，未发现修复范围内具体残留缺陷；最终全门通过。仅 4.1 可 done，W0 义务矩阵、产品输入计划、W3 和 SC1 门未放行。
