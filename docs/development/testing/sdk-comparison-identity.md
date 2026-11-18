# C 分析的 SDK 比较身份

基线与当前侧能否比较，由 `c_comparison` 按语义身份版本 3 判定：双树不回退门（`source_tree`）与本地 `c-check --baseline` 共用同一套输入，只在使用集合是否允许变化上不同。完整已安装库存的摘要、包版本和其余 provider 文件仍写入摘要供追溯，只是不参与判定。

## 参与判定的输入

| 输入 | 内容 | 记录位置 |
| --- | --- | --- |
| include 搜索序列 | 对空输入运行 `gcc -E -v`：`#include "..."` 与 `#include <...>` 两段按顺序；`CPATH`、`C_INCLUDE_PATH` 的取值。目录按目标的路径语法写成相对 provider 根（Linux 为 `/`，Windows 为 MSYS2 根）的路径，根之外的目录保留绝对路径 | `compilerSemanticInputs.includeSearch`，属编译器身份 |
| 搜索空间库存 | 从每个搜索目录出发，跟随目录符号链接和 junction，逐个可达文件；真实文件属于受支持 provider 时记一条：键为编译器走到的路径，值为 provider、包内相对路径（链接指向的真实文件）、实际 SHA-256、包清单摘要。链接指向别处、或指向的字节不同，条目就不同 | `scope.sdkInventory` |
| 实际打开的头与宏 | 每个头的相同字段，每个 TU 的打开集合与宏使用点的活动定义，行指令标志 | `scope.sdkHeaders` 等 |
| 编译器 | machine、完整版本、全部预定义宏、内建定义 | `compilerSemanticInputs` |

GCC 只列出存在的目录，不存在的写成 `ignoring nonexistent directory`，所以一侧有、另一侧没有的目录会让序列不同。没有盘符的 rooted 条目（MinGW 的 `/mingw64/include`）按编译器工作目录所在盘解析：CI 的工作目录在 D 盘，该目录不存在，序列里没有它；工作目录在 C 盘时它指向镜像自带的 `C:\mingw64\include`，以绝对路径进入序列，其下文件不属于受支持 provider，按下文“包外文件”处理。

库存必须存在、格式正确、非空，并覆盖每个打开的 SDK 头；缺失、为空或不完整的回执（任一侧或两侧）一律拒绝，原因为 `SDK search inventory missing, empty or incomplete`。回执摘要与记录不符拒绝为 `sdk receipt`。

项目的 `-I` 在目标配置中另行比较，目标配置只允许固定的编译参数，不含 `-isystem` 与 `--sysroot`。序列取自空输入探测，不含项目 `-I`；探测的两处局限见 [#567](https://github.com/g1331/autsaro/issues/567)。

## 不参与判定的输入

- 包版本。每个打开的头仍在 `scope.sdkHeaders` 记录版本。
- 搜索目录不可达的 provider 文件，如 `libc.a`、`.so`、文档。预处理器和 Cppcheck 读不到它们。完整已安装库存的摘要保存在 `scope.sdkEnvironment`，并对照包清单校验。
- 包外文件：不属于任何受支持 provider 的文件，如 runner 自带的 `/usr/local/include`。把它们纳入身份会在镜像滚动时重新制造 [#564](https://github.com/g1331/autsaro/issues/564)。比较器不检查它们；被打开时，SDK 来源校验把该头记为未分类、来源分类不完整，验收报 `Incomplete SDK source provenance`，必需流水线因此失败（`test_an_opened_header_outside_every_provider_fails_the_acceptance`）。这条依赖关系是必需门的组成部分，修改来源校验时需同时核对。

依据：runner 镜像滚动时 `libc6-dev` 2.39-0ubuntu8.9 与 8.12 各 507 个文件，只有 `libc.a` 和 `audit/sotruss-lib.so` 两个二进制不同，`usr/include` 下 477 个头逐字节相同。按完整库存比较会让这类无关更新拒绝比较；按可达的搜索空间比较则可比。

## 两种比较

| 情形 | 双树（gate） | 同源（`--baseline`） |
| --- | --- | --- |
| 搜索序列或环境变量不同 | 拒绝 `compiler` | 拒绝 `compiler` |
| 可达库存不同：文件不同、缺失、多出，或链接指向别处 | 拒绝 `sdk inventory: <路径>` | 拒绝 `sdk` |
| 共有的打开头字节、共有宏值不同 | 拒绝 | 拒绝 |
| 多开或少开一个头 | 允许（项目源码改 include 时发生） | 拒绝 `sdk` |
| 仅包版本、不可达文件不同 | 可比 | 可比 |
| 语义身份 2 的旧工件 | 拒绝，须用当前工具重扫 | 同左 |

复用已有产物（`c-check` 基线缓存、严格验收重放）时，`recheck_include_search` 重新探测当前的搜索序列与环境变量，与记录不同就不复用。

合并后第一轮 CI 的基线由新代码现场生成，无需人工操作；旧语义身份的 master 工件因分析代码摘要不符而回到现场扫描。
