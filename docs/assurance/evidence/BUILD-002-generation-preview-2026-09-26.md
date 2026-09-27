# BUILD-002 生成工程预览：实施与复核证据

**状态：**独立审查通过（Agent build002_review，2026-09-26）。**基线：**`c89868e9a5ab`。**分支：**`feature/BUILD-002-generation-preview`。**目标：**CP/FO R24-11、Windows 主机虚拟 CAN、GCC 16.1.0；不升级内部支持声明。

## 结果与范围

桌面在选择输出目录后先只读计算全部生成文件与旧输出的差异，列出新增、内容变化及内容不变；变化文件可看前后文本。确认时重新生成相同候选内容、复核 ARXML 与输出完整性及预览修订，再沿原有暂存/备份路径安装。取消不写入输出目录。`files.list` 与 `files.sha256` 也在预览清单内。此处是工具工作流；不新增 BSW/ECUC 语义或标准支持声明。

## 可复现输入与预期

- 输入：在后台隔离桌面中的工作台新建 `Hidden_ECU`，Tx 帧 `HiddenFrame`，11-bit CAN ID `801`（十进制）、DLC `8`、周期 `100 ms`；8-bit LSB0 信号 `HiddenSignal` 起始位 `0`、初始值 `5`。保存并校验无阻断错误。源目录 `<isolated-test-root>/build002-hidden-src`，独立空输出目录 `<isolated-test-root>/build002-hidden-out`。本地最终 ARXML SHA-256：`3ABCF11141601DBC805D2802B954C420428DA2E49D34B7D2F2FF3DC8FD2094AC`（最终版本周期 `120 ms`）。官方 XSD/MOD 仍来自 README 中的本机忽略路径。
- 独立预期：首次预览列出全部新文件而不写盘；确认后生成与预览逐字节相同。把 Tx 周期改为 `120 ms` 并保存后，`Ecu_Config.c` 中帧周期由 `100u` 改为 `120u`，运行源码保持相同，完整性记录变化；确认后旧输出存在备份目录。旧输出已有二进制时预览即拒绝。

## 执行记录

- Windows PowerShell 当前会话先设 `VCPKG_ROOT=<local-vcpkg-root>`、`VCPKGRS_TRIPLET=x64-windows-static`、`LIBCLANG_PATH=<local-libclang-directory>`。工具链：Rust 1.98.1 MSVC、MSYS2 GCC 16.1.0、Node 24.19.0、npm 11.17.0、Python 3.12.9。
- `python scripts/workflow.py verify --scope all`：`git diff --check`、脚本测试 8/8、UI TypeScript/Vite、核心单元 2/2、核心端到端 42/42 通过。最后的 debug Tauri 构建因此前已运行的 `src-tauri/target/debug/autosar-config-desktop.exe` 被进程占用，报 `Access denied (os error 5)` 并退出 101；未停止该其他进程。
- `cargo build --release --manifest-path src-tauri/Cargo.toml`：通过，仍有既有 `LNK4098` 默认库冲突警告。最后微调预览确认期间的二次修订检查后，release Tauri 重建及 `npm run build --prefix ui` 再次通过。初审后补充预览阶段三类拒绝测试，`cargo test --manifest-path core/Cargo.toml generation_preview` 聚焦测试 3/3、`cargo test --manifest-path core/Cargo.toml` 核心单元 2/2、端到端 43/43 通过。占用 debug exe 的其他进程自行退出后，未操作该进程，`cargo build --manifest-path src-tauri/Cargo.toml` 通过。最终重新执行 `python scripts/workflow.py verify --scope all`，工作流脚本 8/8、UI 构建、核心单元 2/2、端到端 43/43、Tauri debug 构建全部通过。
- 初审指出共享交互桌面的窗口演练不符合隔离要求，维护者明确要求 Agent 在后台验收。该次窗口演练只留作历史观察，不作为本任务的 UI 验收证据；随后的验收使用 `CreateDesktopW` 建立未切换到用户屏幕的 `AutosarBuild002Hidden` 桌面，以 `CreateProcessW` 在该桌面启动本轮 release 程序。Vite 只在 `127.0.0.1:1420` 后台提供页面；WebView2 调试端口仅监听本机 `127.0.0.1:9234`。通过 Chrome DevTools Protocol 在后台读取 DOM、触发 UI 按钮并访问 Tauri IPC，没有对用户桌面发键盘或鼠标输入。此法与 [Microsoft WebView2 的调试参数说明](https://learn.microsoft.com/en-us/microsoft-edge/webview2/how-to/debug-visual-studio-code)一致，测试结束后关闭本轮进程。
- 后台原生流程：创建 `Hidden_ECU`、添加帧/信号、在 UI 确认 ARXML 保存、运行 UI 校验；目录选择器未被自动操作，测试适配器将**已有隔离目录路径**直接设到 React 状态，生成预览数据由真实 `preview_generate_project` IPC 返回后注入对话框状态。此限制意味着系统目录选择器本身未在后台验收；本任务新增的预览内容、确认按钮与 Tauri IPC 均运行在真实 WebView2 中。首次预览返回 36 项（34 个工程文件及两份清单），预览时输出目录条目数为 0；UI 确认后显示 34 个工程文件，并由 UI 构建出 `ecu_host.exe`。
- 对含刚构建二进制的同一输出目录调用真实预览 IPC，返回“输出目录含已构建的二进制文件 ecu_host.exe，拒绝替换并保留原目录”。移走本轮生成的二进制至同级保留路径后，将周期改为 `120 ms`，在 UI 保存和校验；预览显示 0 新增、3 修改、33 内容不变，`Ecu_Config.c` 对照第 9 行为 `100u → 120u`。后台 WebView2 的 CDP 截图见 [隔离桌面变化对照](BUILD-002-generation-preview-hidden-2026-09-26.png)。点击 UI 取消后，磁盘上的旧工程仍是 `100u`；再次预览并点击 UI 确认后，旧配置位于 `<isolated-test-root>/.autosar-config-backup-*/build002-hidden-out`，其中帧周期仍是 `100u`，新目录是 `120u`。新 `Ecu_Config.c` SHA-256：`5C4C863449E7FD981A000C6EC151CA4B053D2A6719F6440713E34155A6E1D5B4`。
- 核心测试 `generation_preview_is_read_only_and_confirmed_files_match` 逐项比较预览与安装文件，验证配置变化后旧修订被拒绝；`generation_preview_rejects_changed_existing_output` 验证已改动的旧输出不被覆盖；新增 `generation_preview_rejects_user_files_binaries_and_bad_proofs_before_writing` 在**预览阶段**分别注入用户文件、二进制与损坏的生成文件，确认全部拒绝并保留原字节。

## 尚未验证

- 初审提出的隔离 UI 与直接拒绝测试缺口已补齐。独立审查者重新核对截图、旧/新 `Ecu_Config.c` 字节及 SHA-256，独立运行预览聚焦测试 3/3，未发现新的代码缺陷，同意本任务按主机 CAN 预览范围收口；系统目录选择器及“选择目录→预览”入口仍未完整后台演练，见下项。`HOST-CAN-01` 六道证据门不因此升级。
- 外部跨引用多文件 ARXML 输入、其他配置变体、静态 MISRA/BSWMD/MemMap 义务、第三方协议栈和实机目标均不由本任务的主机结果证明。
- 后台验收没有覆盖 Windows 系统目录选择器的操作。debug exe 的构建问题已在原占用进程自行退出后解除，不属于本次生成预览的产品缺陷。
