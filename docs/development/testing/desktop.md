# 桌面场景

`tests/desktop/*.test.mjs` 通过 Tauri 官方 `tauri-driver` 与平台 WebDriver 驱动真实应用和 IPC，由 Node 内置的 `node:test` 运行。每个文件启动独立的应用会话，使用私有的配置、主目录和临时目录，文件之间没有顺序依赖；场景只使用内置规则，共用辅助位于 `tests/desktop/support/`。

先构建验证版应用（前端打包进程序，不需要开发服务器）并安装 `tauri-driver`：

```powershell
npm run tauri --prefix ui -- build --debug --no-bundle --features native-webdriver
cargo install tauri-driver --version 2.1.0 --locked
node --test --test-concurrency=1 "tests/desktop/*.test.mjs"
```

运行前把 `AUTOSAR_DESKTOP_APP` 设为构建出的 `target/debug/autosar-config-desktop`（Windows 带 `.exe`）的绝对路径。Node 24 的 `--test` 不接受目录，glob 加引号后由 Node 展开；也可只传单个文件，或用 `--test-name-pattern` 选择检查。`native-webdriver` 只用于验证构建：提供计数器、受管失败和延迟注入，并在 Windows 上通过 WebView2 API 转交 msedgedriver 的启动参数（WebView2 在提权进程中忽略 `WEBVIEW2_*` 环境变量，GitHub 托管 runner 以管理员身份运行）。发行构建不包含这些能力。

`AUTOSAR_DESKTOP_RESULTS` 指定结果目录，默认使用系统临时目录。每个文件在其中新建私有目录，保存各项检查的截图、`ipc.jsonl` 和会话关闭后的 `process-tree.json`；失败时另存 `failure.png` 与页面文本 `failure.txt`。`owned-execution.test.mjs` 还需要用 `AUTOSAR_CC`、`AUTOSAR_OBJDUMP`、`AUTOSAR_GIT`、`AUTOSAR_PYTHON` 声明受控 GCC、binutils、Git 与受支持 Python 的绝对路径；应用环境仍剥除这些变量，由场景通过真实的“执行工具”设置填写，未声明时该文件失败。

- Linux：需要 `WebKitWebDriver`（Debian／Ubuntu 软件包 `webkit2gtk-driver`）、`Xvfb`、`dbus-daemon`、`xsettingsd`、`xdotool` 与 `xclip`。每个会话使用私有 Xvfb display 和不激活服务的私有 D-Bus，原生文件选择框也在该 display 中操作，不触及正在使用的桌面，可在开发机直接运行。
- Windows：需要与 Edge／WebView2 版本一致的 `msedgedriver` 位于 PATH 中。会话显示真实窗口、读取会话剪贴板并操作原生文件选择框，因此只在 GitHub Actions、Windows Sandbox 或设置了 `AUTOSAR_DESKTOP_DEDICATED_SESSION=1` 的专用会话中启动，否则直接拒绝；不得在日常桌面运行。各会话共用桌面与剪贴板，必须串行运行。
- macOS：`tauri-driver` 不支持 WKWebView，不运行桌面场景。

关闭会话后，测试断言应用进程树已全部退出，残留时列出 pid、名称和命令行；按 Ctrl-C 或取消 CI 也会结束私有 display 与 driver 进程组。

`desktop.yml` 在 GitHub 托管的 `ubuntu-24.04` 与 `windows-latest` 上构建验证版并运行全部场景；改动应用、界面或场景的 PR 及手动触发都会运行，结果目录作为工件保留 14 天。它目前不在必需检查中。这些场景验证开发构建的界面与 IPC，不提供安装包、无源码 checkout 或断网环境的验收证据。
