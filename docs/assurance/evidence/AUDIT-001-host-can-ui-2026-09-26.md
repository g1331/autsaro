# AUDIT-001：隔离原生界面与来源变更拒绝切片（2026-09-26）

## 范围、输入与基线

- 源码基线 `231b7131eb31`；被测源码提交 `c747bf91f365`，分支 `audit/AUDIT-001-host-can`。目标为 Windows 主机虚拟 ECU；工具链和本地 R24-11 XSD/MOD/Showcase 包摘要见[前一切片](AUDIT-001-host-can-2026-09-26.md)。官方 ZIP 不提交。
- 本机 17 个 R24-11 官方 ZIP 中，对单项不超过 25 MiB 的 `.arxml/.xml` 搜索 `<CAN-FRAME>`、`<I-SIGNAL-I-PDU>` 和 `CanIfCfg` 配置值，未取得可直接编辑的完整外部 CAN 项目；本搜索不涵盖超大成员，也不证明外界没有此类项目。因此本次**原生界面编辑输入仍是本工具生成后拆分的 R24-11 XSD 有效主机配置**，不能填补“真实外部多文件 CAN 输入”的门。
- 可重建的三文件配置由 `core/tests/end_to_end.rs` 的 `three_file_host_can_edit_preserves_retained_and_untouched_sources` 固定：Alpha 的 `Command` 是 ID `801`、DLC `2`、10 ms Tx，`SendCount` 为 bit 3、8 bit、初值 5；`Reply` 是 ID `1110`、DLC `2`、50 ms Rx，`RecvStatus` 在 bit 0。将系统 `ISignal_SendCount` 移到同包 `Signals.arxml`，Mcu 模块移到同包 `Clock.arxml`，另加独立的 `RetainedUnknown` 信号作为保留项。Beta 使用相反 Tx/Rx 方向、相同两 ID、Rx 超时 40 ms 与 Tx 周期 20 ms。测试在临时目录重建和校验输入；原生演练也使用临时目录，不把机器路径写入可移植命令。
- 演练的三份 Alpha 输入在编辑前 SHA-256：主文件 `c4fdb51831785c98cd22874de9b43f672c8e6322df3f816f635bb496e5eadf9a`、Clock `0d5a38e6d5ebf721b43f7cca8de04c10b1596831f45a0e95131fd4c7f5a0725f`、Signals `6d910aed539f576b94c80b3421c1959adf86dc88b15b29d966bda09646ab1801`。这些摘要仅标识本次临时演练输入；可移植重建入口是上述测试的配置步骤。

## 不占用使用者前台的原生执行

后台启动 Vite 后，用 Windows `CreateDesktopW` 建立独立桌面，并通过 `CreateProcess` 的 `STARTUPINFO.lpDesktop` 在其中启动实际 `autosar-config-desktop.exe`。给该进程的 WebView2 设置临时用户数据目录与临时 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<空闲端口>`；经 CDP 操作真实 WebView DOM，经 Windows UI Automation 在该独立桌面选择原生文件/目录。桌面没有切换到用户正在使用的前台。这是**有真实窗口但隔离的原生执行**，不把单纯浏览器无头预览算作 Tauri/IPC 证据；调试端口与临时用户数据目录只用于本次测试。测试结束后停止应用和 Vite。

| 用户操作 / 独立预期 | 实际观察 |
| --- | --- |
| 原生文件选择器一次导入 Alpha 三份同包 ARXML，系统信号与 Mcu 引用跨文件闭合；独立保留项可见 | 原生对话框选入 3 文件；工作台显示 3 文件、2 帧、2 信号、1 保留项；校验 0 个阻断错误 |
| 在信号检查器把 `SendCount` 长度 8 改 7 并应用、保存；主文件 Com 与 Signals 系统信号更新，Clock 不重写 | 保存后主文件 SHA-256 `08db20bc710796efa44d09fbbea742cd980b451da133f1e128de675451cf9d46`，Signals `a6663b9e994bc78b78f9ba3b99b1823c978d2da05ad6f9aef36e7d6de2ebd7af`，Clock 摘要与上行相同；`RetainedUnknown` 仍在；重新导入显示 7 bit、1 保留项且不脏 |
| UI 校验、生成、构建主机 C99 工程，并用不同配置 Beta 作对端；`54=0x36` 从 bit 3 打包应为 `[B0,01]` | UI 阶段各自显示完成；生成清单 34 个工程文件，`ecu_host.exe` 实际构建；虚拟运行显示 ID `801` 载荷 `[B0,01]`、接收信号 `54 valid=1`，ID `1110` 载荷 `[14,00]`、接收信号 `20 valid=1`，并记录丢帧后 71 ms 超时、BUS_OFF 禁发、STARTED 恢复及错误 DLC 拒绝 |
| 已保存且校验的项目，若磁盘 CAN ID 由外部从 `801` 改为 `802`，不得生成旧 ID 的工程 | **修复前失败复现：**UI 再次生成成功，但新 `profile.txt` 仍为 ID `801`；磁盘文件是 `802`。新增断言在修复前同样以 `unwrap_err()` 收到 `Ok(GenerationReport)` 而失败 |
| 修复后再现外部改动，且重新导入应恢复 | 以 ID `802` 重新导入并校验后，外部改磁盘为 `803`；UI “生成”显示外部修改错误，生成阶段为 failed，新目录 0 文件；UI “校验”也显示拒绝及重新导入提示。再次通过原生选择器导入三份文件，检查器读到 `803`，重新校验、生成、构建成功，新 `profile.txt` 为 ID `803` |

上述界面与文件结果由独立的磁盘内容/摘要、生成 `profile.txt`、进程构建结果和 WebView 阶段状态交叉核对；测试目录与调试截图留在临时位置，不纳入仓库或作为可发布工件。

## 代码与命令门

`Workspace::validate` 在返回成功前逐字节复核所选来源文件，`Workspace::save` 在暂存前再次复核；`generator::generate` 经 `checked_profile` 使用同一校验入口，来源变更时在创建输出目录前拒绝。新增回归断言覆盖干净工作区的外部改动、校验拒绝、生成拒绝且没有 C99 输出；三文件测试覆盖两份被编辑文件与一份未编辑文件的保存保真。已执行：

```powershell
# 按 README 为当前机器设置 VCPKG_ROOT、VCPKGRS_TRIPLET 和 LIBCLANG_PATH。
cargo test --manifest-path core/Cargo.toml split_package_save_preserves_sources_and_rejects_stale_reference_file
cargo test --manifest-path core/Cargo.toml three_file_host_can_edit_preserves_retained_and_untouched_sources
python scripts/workflow.py verify --scope all
python scripts/workflow.py check
```

修复前，新增生成拒绝断言退出码 `101`，实际返回 `Ok(GenerationReport)`；修复后两项定向测试通过。`verify --scope all` 首次被工作流测试与实时状态耦合挡住：任务已有 evidence、队列已有 `WF-002`，旧合成测试仍假设为空；已改为“当前状态只读校验 + 独立合成夹具”，Python 8/8 通过。第二次在 Vite 正运行时 `npm ci` 遇到 Windows `EPERM unlink esbuild.exe`；停止本次隔离应用与开发服务后重跑，工作流 8/8、UI 构建、Rust 单元 2/2、端到端 **39/39**、Tauri 构建均通过。Rust/MSVC 构建仍有 `LNK4098` 默认库警告；它不等于 MISRA 或静态质量报告。新增“校验也拒绝”断言后，再次运行定向测试及 `verify --scope all`，结果仍为工作流 8/8、UI 构建、Rust 单元 2/2、端到端 **39/39**、Tauri 构建通过。

## 证据门与交接

- **已关闭的风险：**顺序发生的外部来源文件变更，不再被“校验通过”或“生成成功”掩盖；拒绝前不创建新生成工程，用户重新导入后可恢复。这个结论以本次 Windows 主机配置和测试输入为限。
- **尚不能收口的门：**三文件可编辑输入是工具生成的主机剖面，而非独立外部 CAN 项目；没有完成生成差异预览、非实现者离线交接，以及逐模块 BSWMD/MemMap/MISRA 义务。预检之后并发修改文件的时间窗口仍未做故障注入。主机虚拟运行不证明实机、第三方互操作或标准符合性。`HOST-CAN-01` 仍为 `documented_behavior`；`AUDIT-001` 保持 `active`。
- **下一步：**取得可追溯的外部 R24-11 多文件 CAN ECU 输入，并先判断其配置是否落在固定主机剖面；可安全编辑者走界面、差异预览和离线交接，不属于剖面者保存原始文件、记录只读及生成拒绝。随后按产物/模块补规范义务与静态分析证据。

## 独立复核

另一 Agent 只读复核了 `c747bf91f365` 的来源字节检查、保存复核、生成前拒绝和两项端到端断言；独立复跑两项定向测试、工作流 Python **8/8** 与 `workflow.py check`，均通过。其 Rust 首次执行缺少本机 vcpkg 环境变量，按 README 设定后复跑通过。复核还对照了临时输入摘要、7 bit 信号及保留项、未改写的 Clock、34 项生成清单、空拒绝目录和恢复后的 ID `803`。未发现阻断本次本地集成的缺陷，同意 `AUDIT-001` 维持 `active`、`HOST-CAN-01` 维持 `documented_behavior`。

复核明确指出，磁盘产物和截图不能独立证明 `CreateDesktopW`、原生选择器、IPC 的完整逐步操作，因此 `user_workflow` 仅为 `pending_review`。来源预检与生成之间的并发改动仍可能产生旧快照工程，不能把顺序拒绝扩大解释为并发安全。工作流测试的合成夹具还依赖实时列表前两项的顺序；已记入 `WF-003`，不阻断本切片。运行中的 Vite 占用 `esbuild.exe` 已记入 `WF-004`。
