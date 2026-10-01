---
title: 'Epic 4 A2 交付后编辑保存的原生回归'
type: 'chore'
created: '2026-10-01'
status: 'done'
route: 'oneshot'
review_loop_iteration: 0
baseline_commit: '099ba74799371a4c717b8fb8d5eaac1ac446af50'
context: []
---

<frozen-after-approval reason="用户明确委托完成A2，沿用持续实施授权">

## Intent

在现有隔离原生脚本中保护真实工作台的交付失效行为。Given校验、生成、构建和主机行为已通过，when应用合法的新周期并保存，then未保存时准确显示失效，保存后四阶段全部未执行，旧预览和日志清空，旧构建／验证按钮禁用；重新生成后才能对新输入构建和实际验证。原交付包与旧二进制不变，现有草稿、拒绝、恢复、重导入、再生成和旧格式断言保留。仅使用真实IPC和独立桌面，产物临时保存，不新增报告、不推送；无需改变产品接口或实现。

</frozen-after-approval>

## Implementation Notes

按安装工作流的小改动oneshot路径执行：仅扩展`scripts/epic4_desktop_cdp.mjs`。复用现有stage／click／input、保存和本次目录授权机制。应用后sourceReady为false，阶段显示“需重新校验／已失效”；保存后才能断言四阶段“未执行”，不能用后续重开或重导入替代保存路径的断言。验证使用A1已构建的真实桌面程序，运行既有`epic4_desktop.py`、Node语法和源码增量检查。

首次实际运行在应用后导航失败：按钮真实文字包含“未保存”徽标，原精确click仅匹配“标准输入”。已依据App.tsx导航实现修正这一处为真实带徽标标签，保持disabled拒绝与所有断言，不修改产品。该次运行不计A2通过；隔离Job已收尾。oneshot盲审完成，无具体发现；实际运行结果继续优先。

最终完整隔离原生流程退出0：保存后的四阶段、预览／日志清空及按钮禁用，所有旧包文件及原二进制字节保持，21ms新工程真实生成／构建／CAN-DID-N_Cr验证，随后原搬移／重导入／再生成字节相等和旧host-v1／格式拒绝全部通过。原脚本行均保留，仅增加99行；Node语法及源码质量检查通过。临时原始日志autosar-a2-native-final.log，临时目录autosar-epic4-desktop-vjqixo1l；两张新增实际画面已检查，状态与按钮可读，输入桌面未改变。普通测试没有受版本管理报告变更。oneshot盲审复查完成，无发现或递延。
