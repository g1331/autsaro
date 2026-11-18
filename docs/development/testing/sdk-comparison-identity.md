# C 分析的 SDK 比较身份

基线与当前侧能否比较，由 `c_comparison` 按语义身份版本 3 判定：双树不回退门（`source_tree`）与本地 `c-check --baseline` 共用同一套输入，只在使用集合是否允许变化上不同。完整环境清单、包版本和其余 provider 文件仍写入摘要供追溯，只是不参与判定。

## 参与判定的输入

| 输入 | 内容 | 记录位置 |
| --- | --- | --- |
| include 搜索序列 | `gcc -E -v` 的 `#include "..."` 与 `#include <...>` 两段，按顺序；`CPATH`、`C_INCLUDE_PATH` 的取值。目录写成相对 provider 根（Linux 为 `/`，Windows 为 MSYS2 根）的路径 | `compilerSemanticInputs.includeSearch`，属编译器身份 |
| 搜索空间库存 | provider 清单中位于任一搜索目录（递归）下的全部文件，不论是否被打开：provider、包内相对路径、实际 SHA-256、包清单摘要 | `scope.sdkInventory` |
| 实际打开的头与宏 | 每个头的相同字段，每个 TU 的打开集合与宏使用点的活动定义，行指令标志 | `scope.sdkHeaders` 等 |
| 编译器 | machine、完整版本、全部预定义宏、内建定义 | `compilerSemanticInputs` |

项目的 `-I` 在目标配置中另行比较。目标配置只允许固定的编译参数，不含 `-isystem` 与 `--sysroot`，搜索序列已反映最终生效的目录。

## 不参与判定的输入

- 包版本。每个打开的头仍在 `scope.sdkHeaders` 记录版本。
- 搜索目录之外的 provider 文件，如 `libc.a`、`.so`、文档。预处理器和 Cppcheck 读不到它们。完整已安装库存的摘要保存在 `scope.sdkEnvironment`，并对照包清单校验。

依据：runner 镜像滚动时 `libc6-dev` 2.39-0ubuntu8.9 与 8.12 各 507 个文件，只有 `libc.a` 和 `audit/sotruss-lib.so` 两个二进制不同，`usr/include` 下 477 个头逐字节相同。按完整库存比较会让这类无关更新拒绝比较；按搜索空间比较则可比。

## 两种比较

| 情形 | 双树（gate） | 同源（`--baseline`） |
| --- | --- | --- |
| 搜索序列或环境变量不同 | 拒绝 `compiler` | 拒绝 `compiler` |
| 搜索空间内任一文件不同、缺失或多出 | 拒绝 `sdk inventory: <头>` | 拒绝 `sdk` |
| 共有的打开头字节、共有宏值不同 | 拒绝 | 拒绝 |
| 多开或少开一个头 | 允许（项目源码改 include 时发生） | 拒绝 `sdk` |
| 仅包版本、搜索目录外文件不同 | 可比 | 可比 |
| 语义身份 2 的旧工件 | 拒绝，须用当前工具重扫 | 同左 |

搜索目录内不属于受支持 provider 包的文件：被打开时记为未分类，来源分类不完整，验收拒绝；未被打开时不检查。

合并后第一轮 CI 的基线由新代码现场生成，无需人工操作；旧语义身份的 master 工件因分析代码摘要不符而回到现场扫描。
