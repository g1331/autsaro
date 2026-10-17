# 桌面场景

`tests/desktop/*.test.mjs` 通过 Tauri 官方 `tauri-driver` 与平台 WebDriver 驱动真实应用和 IPC，由 Node 内置的 `node:test` 运行。每个文件启动独立的应用会话，使用私有的配置、主目录和临时目录，文件之间没有顺序依赖；场景只使用内置规则，共用辅助位于 `tests/desktop/support/`。

先构建验证版应用（前端打包进程序，不需要开发服务器）并安装 `tauri-driver`：

```powershell
npm run tauri --prefix ui -- build --debug --no-bundle --features native-webdriver -- --profile desktop-test
cargo install tauri-driver --version 2.1.0 --locked
# Linux（私有 display）：
node --test --test-concurrency=2 "tests/desktop/*.test.mjs"
# Windows（专用会话）：
node --test --test-concurrency=1 "tests/desktop/*.test.mjs"
```

运行前把 `AUTOSAR_DESKTOP_APP` 设为构建出的 `target/desktop-test/autosar-config-desktop`（Windows 带 `.exe`）的绝对路径。Node 24 的 `--test` 不接受目录，glob 加引号后由 Node 展开；也可只传单个文件，或用 `--test-name-pattern` 选择检查。`desktop-test` 继承 dev，保留 debug assertions 和 overflow checks，并优化 core；依赖继续采用 dev 配置，构建产物和缓存与普通开发构建分开。`native-webdriver` 只用于验证构建：提供计数器、受管失败和延迟注入，并在 Windows 上通过 WebView2 API 转交 msedgedriver 的启动参数（WebView2 在提权进程中忽略 `WEBVIEW2_*` 环境变量，GitHub 托管 runner 以管理员身份运行）。发行构建不包含这些能力。

人工解析延迟只由 responsiveness 的取消场景显式启用，其他场景不注入延迟。`session.json` 记录每个会话的 display、两个 driver 端口及配置／数据／缓存目录，用于核对并行隔离。

`AUTOSAR_DESKTOP_RESULTS` 指定结果目录，默认使用系统临时目录。每个文件在其中新建私有目录，保存各项检查的截图、`ipc.jsonl`、逐请求耗时 `webdriver.jsonl` 和会话关闭后的 `process-tree.json`；失败时另存 `failure.png` 与页面文本 `failure.txt`。`owned-execution.test.mjs` 还需要用 `AUTOSAR_CC`、`AUTOSAR_OBJDUMP`、`AUTOSAR_GIT`、`AUTOSAR_PYTHON` 声明受控 GCC、binutils、Git 与受支持 Python 的绝对路径；应用环境仍剥除这些变量，由场景通过真实的“执行工具”设置填写，未声明时该文件失败。

页面条件采样写入 `dom-queries.jsonl`，包含完整查询、发出／页面采样／收到时间、返回值及同次 DOM 的活动操作和编辑器状态。失败时，在截图前保留最后采样 `failure-query.json`、随后读取的 `failure-capabilities.json` 与现场进程树 `failure-processes.json`；Linux 另存每个所属进程的线程状态及应用、WebKitWebDriver、WebKit WebProcess 的全线程 GDB backtrace（需安装 `gdb`）。采集错误原样保留，`failure-capture.json` 标明各项采集结果；后续截图不能替代最后查询的现场。每个会话的 `host-start.json`／`host-end.json` 记录宿主 uptime、可用 CPU 数、空闲内存及并行桌面／编译进程，失败另存 `host-failure.json`；时序结论须结合这些现场，不把争用下的失败归因为产品缺陷或计作通过。

- Linux：需要 `WebKitWebDriver`（Debian／Ubuntu 软件包 `webkit2gtk-driver`）、`Xvfb`、`dbus-daemon`、`xsettingsd`、`xdotool` 与 `xclip`。每个会话使用私有 Xvfb display 和不激活服务的私有 D-Bus，原生文件选择框也在该 display 中操作，不触及正在使用的桌面，可在开发机直接运行。D-Bus socket 放在独立的短私有临时目录中，避免长结果目录或场景名超过 Unix 地址长度；正常关闭及启动失败都清理已登记的私有进程和目录。
- Windows：需要与 Edge／WebView2 版本一致的 `msedgedriver` 位于 PATH 中。会话显示真实窗口、读取会话剪贴板并操作原生文件选择框，因此只在 GitHub Actions、Windows Sandbox 或设置了 `AUTOSAR_DESKTOP_DEDICATED_SESSION=1` 的专用会话中启动，否则直接拒绝；不得在日常桌面运行。各会话共用桌面与剪贴板，必须串行运行。
- macOS：`tauri-driver` 不支持 WKWebView，不运行桌面场景。

关闭会话后，测试断言应用进程树已全部退出，残留时列出 pid、名称和命令行；按 Ctrl-C 或取消 CI 也会结束私有 display 与 driver 进程组。

同一台 Linux 机器可以在不同工作树中并行运行多个场景。手动中止或清理时只结束自己启动并记下的进程或进程组（如用 `setsid` 启动后 `kill -- -<pgid>`），不要按名称或命令行匹配（`pkill -f`、`killall`），否则会连带结束其他工作树正在运行的场景。

`desktop.yml` 在 GitHub 托管的 `ubuntu-24.04` 与 `windows-latest` 上各构建一次验证版，将应用、资源与 tauri-driver 上传为工件，再启动 Linux 两个、Windows 三个场景分片。文件名单按 run 38993367434 的基线与优化后 run 39005768979、39008074660、39013110980 的耗时均衡；Windows 的第三个分片为已观察到的执行波动留出余量，明确写在 workflow 的 `matrix.include` 中；每个文件只属于一个分片，Python 开发测试检查全量覆盖。Linux 分片内文件并发数为 2，Windows 为 1；文件内有状态的检查保持顺序。每个分片都保留自己的结果工件 14 天，任一分片失败都会使调用方的 `Extended checks result` 失败。它由 [`Extended checks`](README.md#ci) 调用：改动 `ui/`、`src-tauri/` 或 `tests/desktop/` 的 PR，以及改动这些目录、`core/` 或 Cargo 依赖的 master 推送；PR 加 `full-ci` 标签时也运行。手动运行 `desktop.yml` 可在 `scenarios` 中填写以空格分隔的场景文件或 glob，只运行受影响的场景，默认全部。它目前不在必需检查中。这些场景验证开发构建的界面与 IPC，不提供安装包、无源码 checkout 或断网环境的验收证据。

## 编写场景

- 原生交互按真实异步操作边界同步：对话框出现后等待其可操作且获得焦点，再测试键盘；保存配置后同时核对界面控件解除忙态，不能把 capability 值已更新当作界面动作已完成；对话框关闭后等待后台事务完成，再发起后续 IPC。不能用弱化并发拒绝或固定延时掩盖操作竞态。
- 场景按 IPC 错误的消息 key 判断后端错误，不匹配界面翻译文本。
- Windows 上 msedgedriver 的参数只能经上述 WebView2 API 转交，不要另设 `WEBVIEW2_USER_DATA_FOLDER`。改写应用的 `USERPROFILE` 时须提供 `Desktop`，否则 Shell 会在原生选择框之外再弹出“位置不可用”；场景遇到选择框以外的原生对话框应失败并报告其文字，不能静默关闭。
