# MemMap

本篇属于[标准与规范基线](README.md)：条款处置的标记与核算方法见 [README §5](README.md#5-条款处置与核算)。列出的条款表示已定位并用于设计或验收，不表示已逐条核完。

文档：`AUTOSAR_CP_SWS_MemoryMapping.pdf`（`9dc38d360e4a154f627ede71cc5207f7be193c55cbb9f87875e2731af14c891c`）。

| 条款 | 义务要点 |
| --- | --- |
| `00020／00028／00029／00032`（`[SWS_MemMap_00032]`／`[SWS_MemMap_00029]`） | 每个实际模块与每个 SWC type 的 MemMap 由 Memory Mapping 按输入配置提供；主机段映射不代表 MCU 段放置 |

## 已知差距

- **MemMap 与段放置**：主机 PE／ELF 段映射已有实际证据，MCU 段与权限只由目标工具链与布局关闭。
