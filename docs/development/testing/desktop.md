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

`AUTOSAR_DESKTOP_RESULTS` 指定结果目录，默认使用系统临时目录。每个文件在其中新建私有目录，保存各项检查的截图、`ipc.jsonl`、逐请求证据 `webdriver.jsonl`（按会话内 `id` 配对的 `started`／`completed`／`failed`，包含完整求值表达式、时间和失败原文；请求发出前即落盘，响应体解析失败也保留终态）和会话关闭后的 `process-tree.json`；失败时另存 `failure.png` 与页面文本 `failure.txt`。启动失败也在清理前保存 `startup-failure.json`（原始错误、同步页面上下文与进程树）及可取得的 `startup-failure.png`，Windows 另存 `windows-versions.json`；driver stdout／stderr 分别保存为 `driver.stdout.log`／`driver.stderr.log`，包括代理连接错误；每个流按字节顺序写入，正常关闭等待输出完成，写入或流错误在自有进程清理后报告，附日志路径并保留底层错误。故障断言不匹配时保留原始异常，避免覆盖其路径、错误码及调用栈。验证构建的 `AUTOSAR_NATIVE_REQUEST` 记录 projection／capabilities 命令进入和返回；返回标记位于 Tauri 响应序列化之前，不能视为页面已收到。启动就绪条件使用同步脚本探测，避免 Chromium 将导航中止异步脚本映射为 `script timeout`；错误仍直接失败，不重试。`owned-execution.test.mjs` 还需要用 `AUTOSAR_CC`、`AUTOSAR_OBJDUMP`、`AUTOSAR_GIT`、`AUTOSAR_PYTHON` 声明受控 GCC、binutils、Git 与受支持 Python 的绝对路径；应用环境仍剥除这些变量，由场景通过真实的“执行工具”设置填写，未声明时该文件失败。

异步求值将完整 JSON 结果封装为文本交给 WebDriver，在 Node 端解码，避免大型投影在 native driver 中再次展开为对象树；源字节、字段和深比较仍保留完整值。序列化异常也通过同一错误回调结束请求。

页面求值在 `window.__AUTOSAR_DESKTOP_EVALUATION__` 保留当前编号及 entered／resolved／rejected／callback 阶段，可用只读 observer 同步取证；callback 标记表示向 WebDriver 交出结果，不代表驱动完成序列化或传输。原生 transport 不被替换。页面条件采样写入 `dom-queries.jsonl`，包含完整查询、发出／页面采样／收到时间、返回值及同次 DOM 的活动操作和编辑器状态。失败时，在截图前保留最后采样 `failure-query.json`、随后读取的 `failure-capabilities.json` 与现场进程树 `failure-processes.json`；Linux 另存每个所属进程的线程状态、内存映射及应用、WebKitWebDriver、WebKit WebProcess 的全线程 GDB backtrace（需安装 `gdb`）。每次调试器采集另存 `failure-debugger-<pid>.json`（开始／结束时间、退出结果和采集后进程状态），关闭自动加载和 debuginfod，并在原 20 秒期限用 SIGKILL 结束调试器，避免符号下载／初始化或调试器信号处理拖住采集。关闭所属 Linux 进程组时，在 SIGTERM 后发送 SIGCONT，使调试器遗留的停止态应用能接收终止信号；仍须通过原 15 秒进程身份消失断言。采集错误原样保留，`failure-capture.json` 标明各项采集结果；后续截图不能替代最后查询的现场。每个会话的 `host-start.json`／`host-end.json` 记录宿主 uptime、可用 CPU 数、空闲内存及并行桌面／编译进程，失败另存 `host-failure.json`；时序结论须结合这些现场，不把争用下的失败归因为产品缺陷或计作通过。

- Linux：需要 `WebKitWebDriver`（Debian／Ubuntu 软件包 `webkit2gtk-driver`）、`Xvfb`、`dbus-daemon`、`xsettingsd`、`xdotool` 与 `xclip`。每个会话使用私有 Xvfb display 和不激活服务的私有 D-Bus，原生文件选择框也在该 display 中操作，不触及正在使用的桌面，可在开发机直接运行。D-Bus socket 放在独立的短私有临时目录中，避免长结果目录或场景名超过 Unix 地址长度；正常关闭及启动失败都清理已登记的私有进程和目录。
- Windows：需要与 Edge／WebView2 版本一致的 `msedgedriver` 位于 PATH 中。会话显示真实窗口、读取会话剪贴板并操作原生文件选择框，因此只在 GitHub Actions、Windows Sandbox 或设置了 `AUTOSAR_DESKTOP_DEDICATED_SESSION=1` 的专用会话中启动，否则直接拒绝；不得在日常桌面运行。各会话共用桌面与剪贴板，必须串行运行。
- macOS：`tauri-driver` 不支持 WKWebView，不运行桌面场景。

transport 请求中止或响应体解析失败后，会话保留首个错误并拒绝后续请求，避免在仍执行旧命令的 driver 上排队诊断和关闭操作；WebDriver 返回的正常协议错误仍可继续诊断。关闭会话时先清理所属进程，再断言应用进程树已全部退出并保存证据，最后传播关闭错误，残留时列出 pid、名称和命令行；按 Ctrl-C 或取消 CI 也会结束私有 display 与 driver 进程组。

同一台 Linux 机器可以在不同工作树中并行运行多个场景。手动中止或清理时只结束自己启动并记下的进程或进程组（如用 `setsid` 启动后 `kill -- -<pgid>`），不要按名称或命令行匹配（`pkill -f`、`killall`），否则会连带结束其他工作树正在运行的场景。

在沙箱或容器外执行的会话，须在执行会话的同一宿主 PID 命名空间核对进程退出；另一命名空间的 `/proc` 看不到目标 PID，不能证明它已退出。归因时记录每次进程／线程读取的实际时间，集合入口时间不能代表所有后续读取；缺页次数和 CPU 时间是生命周期累计量，只有同一请求区间的增量及对应栈才能说明该次等待中的活动。

`desktop.yml` 在 GitHub 托管的 `ubuntu-24.04` 与 `windows-latest` 上各构建一次验证版，将应用、资源与 tauri-driver 上传为工件，再启动 Linux 两个、Windows 三个场景分片。文件名单按 run 38993367434 的基线与优化后 run 39005768979、39008074660、39013110980 的耗时均衡；Windows 的第三个分片为已观察到的执行波动留出余量，明确写在 workflow 的 `matrix.include` 中；每个文件只属于一个分片，Python 开发测试检查全量覆盖。Linux 分片内文件并发数为 2，Windows 为 1；文件内有状态的检查保持顺序。每个分片都保留自己的结果工件 14 天，任一分片失败都会使调用方的 `Extended checks result` 失败。它由 [`Extended checks`](README.md#ci) 调用：改动 `ui/`、`src-tauri/` 或 `tests/desktop/` 的 PR，以及改动这些目录、`core/` 或 Cargo 依赖的 master 推送；PR 加 `full-ci` 标签时也运行。手动运行 `desktop.yml` 可在 `scenarios` 中填写以空格分隔的场景文件或 glob，只运行受影响的场景，默认全部。它目前不在必需检查中。这些场景验证开发构建的界面与 IPC，不提供安装包、无源码 checkout 或断网环境的验收证据。

## 编写场景

- 主动刷新页面使用 WebDriver `POST /session/{id}/refresh` 并等待导航响应，再查询新文档的 timeOrigin、公开界面与原生 transport。不要用定时 `location.reload()` 后立即探测脚本：脚本及其返回的 Promise 可能跨越旧 frame 的 unload；`execute/sync` 本身不能等待尚未开始的定时导航。导航错误直接失败，不能通过重试掩盖。
- 原生交互按真实异步操作边界同步：对话框出现后等待其可操作且获得焦点，再测试键盘；保存配置后同时核对界面控件解除忙态，不能把 capability 值已更新当作界面动作已完成；对话框关闭后等待后台事务完成，再发起后续 IPC。不能用弱化并发拒绝或固定延时掩盖操作竞态。
- 需要显示对象行的非空工程场景，导入后等待工程树发布对象行再读取完整原生投影。工作区标签页先于异步对象投影出现，操作状态消失也不代表对象树已接纳结果；过早读取会与界面的完整投影请求重叠。虚拟化场景只检查已挂载对象行，完整内容和末端对象仍由原生投影及键盘断言核对。
- 大型工程树按完整逻辑树检查内容和滚动范围，按视口检查挂载行数；使用真实 Home／End、父子方向键和搜索验证离屏对象可达，不把当前 DOM 行数当成工程对象总数。
- 场景按 IPC 错误的消息 key 判断后端错误，不匹配界面翻译文本。
- Windows 上 msedgedriver 的参数只能经上述 WebView2 API 转交，不要另设 `WEBVIEW2_USER_DATA_FOLDER`。改写应用的 `USERPROFILE` 时须提供 `Desktop`，否则 Shell 会在原生选择框之外再弹出“位置不可用”；场景遇到选择框以外的原生对话框应失败并报告其文字，不能静默关闭。

## 必需门禁的证据条件

#91 的门禁提升仍须先取得两个平台连续至少五次正常触发、无重试的完整成功运行，场景阶段目标为每个平台不超过 15 分钟。保留原有场景与断言，不通过手动重跑矩阵构造连续成功记录。文档改动导致的 skipped desktop 和绿色汇总不计入执行序列；失败、取消和后续成功的重试均保留，不能从序列中删掉失败。

核查每次运行的 commit、触发事件和 `run_attempt`，再从五个分片的实际日志核对文件名单、执行／失败／跳过数量及 Node 场景耗时。构建依赖、验证版应用构建、分片准备、场景步骤和排队分别计时；并行分片的平台场景关键路径取最慢分片，不把所有分片相加，也不把构建到末片结束的墙钟时间称为场景耗时。部分选择的手动运行不能代替全量证据。jobs、日志或工件不可读时保留证据缺口，不凭汇总成功提升门禁。

证据成立后复用现有 `desktop.yml` 调用与共享构建，在 Development checks 按相关路径选择并加入 `CI required`，同时移除 Extended 中同一事件的重复调用。提升前仍保留现有手动与路径触发；#311 的历史请求归因须依其自身出口判断，后续绿色运行不构成其根因修复证明。
