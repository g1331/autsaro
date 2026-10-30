# SDK名字的隔离与恢复

授权来自[2026-10-30维护者决定](https://github.com/g1331/autsaro/pull/389#issuecomment-6513137017)，仅覆盖提案6481577012在879f3962的精确实体及原展开／声明；不是ADR0023的规范不可替代性结论。保留真实违反、raw、not_assessed及refusal，不改变规则类别、不增加baseline或suppression。

表中 `All` 为 standard-ecu／user-application／component-pipeline／no-diagnostic，`ND` 仅 no-diagnostic；目标 `Both` 为 linux-x64-controlled-v1／windows-x64-controlled-v1，Linux／Windows 只选对应受控目标；程序 `Both` 仅 host-batch／probe。范围顺序为 profile／目标／程序，各表行分别限定。两份879f3962平台工件的实际程序集合已核对，不按程序数量推定集合相同；该范围是人工源与raw核对结果，c_standard 不替代程序使用分析或严格比较。

`shape` 是去注释及续行后的实际声明／宏定义／undef；表内 `&#124;` 表示C的竖线。现有c_standard双向关联并对本批shape作token核对，保留函数式宏的名字与左括号相邻关系及##操作符。缺注释、错规则／owner、改body及类别外MR12均拒绝；未带shape的原ADR0023按名记录保持原语义。签名、实体、宏body、程序／目标／profile或包含闭包改变时，须重评，不按规则／文件／前缀授权新位置。源码关联不授予新的项目批准。

R20.5为Advisory/Single Translation Unit。SDK导入前隔离SetEvent、boolean、ApplicationType并按原进入状态恢复；四处undef是自有选择，不套MemMap或SDK来源豁免。既有多入口、函数类型、六包含顺序、SC1拒绝和实际Windows编译合同保持。

| 规则 | 精确范围 | 形态、范围及技术理由 | 原授权 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 20.5 | `runtime/os/src/Os_Windows.h` 中 `#undef SetEvent` | shape=`#undef SetEvent`; 范围 `All/Windows/Both`；临时隔离SDK名字后按进入状态恢复OS名字；既有六包含顺序和Win32实际编译约束保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 20.5 | `runtime/os/src/Os_Windows.h` 中 `#undef boolean` | shape=`#undef boolean`; 范围 `All/Windows/Both`；临时隔离SDK名字后按进入状态恢复OS名字；既有六包含顺序和Win32实际编译约束保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 20.5 | `runtime/os/src/Os_Windows.h` 中 `#undef ApplicationType` | shape=`#undef ApplicationType`; 范围 `All/Windows/Both`；临时隔离SDK名字后按进入状态恢复OS名字；既有六包含顺序和Win32实际编译约束保持。 | 维护者 2026-10-30精确124键决定 |
| MISRA C:2012 Rule 20.5 | `runtime/os/src/Os_Windows.h` 中 `#undef OS_RESTORE_EVENT_SERVICE` | shape=`#undef OS_RESTORE_EVENT_SERVICE`; 范围 `All/Windows/Both`；临时隔离SDK名字后按进入状态恢复OS名字；既有六包含顺序和Win32实际编译约束保持。 | 维护者 2026-10-30精确124键决定 |
