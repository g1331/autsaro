# 测试

测试按实际前置条件选择，不需要恢复代理会话。

## 基础检查

```powershell
npm run lint --prefix ui
npm run test --prefix ui
npm run build --prefix ui
cargo test --locked --manifest-path core/Cargo.toml
uv run --locked python -B -m unittest discover -s tests/python
uv run --locked ruff check tools tests/python scripts
```

前端使用 Vitest；`npm run test:watch --prefix ui` 用于开发。Python 普通测试位于 `tests/python`，进程和平台测试独立选择。默认 Rust 测试不需要外部 Python 或官方档案。

## 进程、资源和原生行为

```powershell
uv run --locked python -B -m unittest discover -s tests/python/process
cargo test --locked --manifest-path core/Cargo.toml --lib --features native-tests execution::
cargo test --locked --manifest-path core/Cargo.toml --features official-oracles
cargo test --locked --manifest-path core/Cargo.toml --features native-tests
```

进程测试需 `AUTOSAR_PYTHON` 指向支持的绝对解释器路径。官方对照需正式资源清单声明的合法档案；完整 native 测试需对应平台的受控 GCC/binutils，26 个 OS suites 通过 Cargo 注册运行，不重复执行。Windows 的历史交接对照会编译收到的包，因此 `official-oracles` 也需对应受控工具链。

需要保留 OS 的编译产物和进程日志时，设置 `AUTOSAR_OS_OUTPUT_DIRECTORY` 为新的结果目录，Cargo 测试按 suite 保存到该目录。单独调用 `autosar_tooling os` 可传 `--output-directory`。已有目录不会被覆盖；未指定时仍只保留失败现场。

BMad 的兼容入口 `uv run --locked python -m autosar_tooling verify --scope all --base <基准提交>` 组合原有完整检查，不安装依赖，不编排另一套任务或写汇总报告。失败直接保留原输出和退出状态。

## 隔离桌面与安装包

```powershell
uv run --locked python -m autosar_tooling desktop --platform windows --binary <应用绝对路径> --output-directory <新的空结果目录>
```

桌面驱动位于 `tests/desktop/`，平台控制和输入准备位于 `autosar_tooling.acceptance`。首次执行历史兼容场景前，用 `cargo build --locked --manifest-path core/Cargo.toml --bins` 构建测试所用的独立生成器。

Linux 和 macOS 使用对应 platform。`--installed --source-checkout <原构建目录>` 验证解包产物及原 checkout 不可用的接收场景。`--builtin-only` 用于安装包内置配置检查；计数器、受管错误和延迟测试使用 `native-webdriver` 验证构建，普通发行构建不会启用这些能力。Linux 驱动需要 Xvfb、WebKitWebDriver、tauri-driver、xdotool、xclip、xsettingsd 与 dump_xsettings；安装后离线验收还使用普通用户的私有 user/network namespace。平台隔离前置条件不满足时明确失败；不得在用户正在使用的桌面补测。

离线 ECU 包可由 Python 3.11 或更新版本运行自带 `tools/ecu-tool.py`，不需要 uv、Rust、Node 或源 checkout。构建和验证仍检查封存内容与目标工具链。支持版本的 Windows/Linux 实际构建由受控验收验证，未运行的平台或版本不宣称通过。
