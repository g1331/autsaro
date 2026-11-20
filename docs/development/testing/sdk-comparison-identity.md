# C 分析的 SDK 比较身份

基线与当前侧能否比较，由 `c_comparison` 按语义身份版本 4 判定：双树不回退门（`source_tree`）与本地 `c-check --baseline` 共用同一套输入，只在使用集合是否允许变化上不同。完整已安装库存的摘要、包版本和其余 provider 文件仍写入摘要供追溯，只是不参与判定。

## 参与判定的输入

| 输入 | 内容 | 记录位置 |
| --- | --- | --- |
| include 搜索序列 | 对空输入运行 `gcc -E -Wp,-v`：`#include "..."` 与 `#include <...>` 两段按顺序；`CPATH`、`C_INCLUDE_PATH` 的取值。目录按目标的路径语法写成相对 provider 根（Linux 为 `/`，Windows 为 MSYS2 根）的路径，根之外的目录保留绝对路径 | `compilerSemanticInputs.includeSearch`，属编译器身份 |
| 搜索空间库存 | 从每个搜索目录出发遍历，跟随目录符号链接和 junction。`files`：编译器走到的路径 → 其后的 provider 文件（provider、包内相对路径、实际 SHA-256、包清单摘要），不论是否打开；实际打开的 SDK 头另按自己的路径加入，使 `#include "../x.h"` 逃出搜索目录到达的头也在库存里。`links`：遇到的符号链接、junction（目录链接、文件链接，无论是否继续进入）以及经上层链接再次到达的已进入目录，且只记目标是 provider 文件、或其下含 provider 文件的目录的那些，写“到达路径 → 解析后的目标”，目标相对 provider 根，根之外写成 `outside:<绝对路径>` | `scope.sdkInventory` |
| 实际打开的头与宏 | 每个头的相同字段，每个 TU 的打开集合与宏使用点的活动定义，行指令标志 | `scope.sdkHeaders` 等 |
| 编译器 | machine、完整版本、带项目 `-I` 探测的全部预定义宏、内建定义 | `compilerSemanticInputs` |

遍历规则：每个真实目录只在第一条到达它的路径下展开，其余路径（包括回到祖先的链接）只记 `links` 条目。这样环路终止，路径数不会爆炸。经链接到达的路径都有条目；展开后再经真实子目录到达的路径没有条目，其内容仍由首条路径的 `files` 覆盖。目标逃出搜索空间的链接，目标目录里的 provider 文件在第一次到达时展开，包外文件不计（见下）。可解析的链接链记录最终目标；相对链接与指向同一目标的绝对链接等价。悬空、环路或 Windows 无效目标名在遍历事件中使用链接自身写入的目标，按链接所在目录规范化。库存以记录的目标判断是否保留。目标是 provider 文件或其下包含 provider 文件时，即使条目为悬空也保留。权限和 I/O 错误中止分析。记录名去掉 Windows 扩展路径前缀。

GCC 只列出存在的目录，不存在的写成 `ignoring nonexistent directory`，所以一侧有、另一侧没有的目录会让序列不同。没有盘符的 rooted 条目（MinGW 的 `/mingw64/include`）按编译器工作目录所在盘解析：CI 的工作目录在 D 盘，该目录不存在，序列里没有它；工作目录在 C 盘时它指向镜像自带的 `C:\mingw64\include`，以绝对路径进入序列，其下文件不属于受支持 provider，按下文“包外文件”处理。

库存与链接记录必须存在、格式正确，库存非空并覆盖每个打开的 SDK 头；缺失、为空或不完整的回执（任一侧或两侧）一律拒绝，原因为 `SDK search inventory missing, empty or incomplete`。回执摘要与记录不符拒绝为 `sdk receipt`。

项目的 `-I` 路径和顺序已在封存的目标配置中比较。分析和重放都在实际 TU 的 `-dM` 宏发现与 `-dD -MD` 来源追踪中读取有效搜索序列。`-Wp,-v` 把详细输出选项直接传给预处理器，驱动的 specs 仍按 TU 选项匹配。有效序列仅在原系统序列前加入 GCC 去重后的项目目录，引号搜索序列保持一致。若 GCC specs 等配置按 `-I` 加入新 SDK 目录、移除或重排目录，立即拒绝。这样 SDK 比较仍可使用不带项目路径的序列，避免把分析副本的绝对路径写入 SDK 身份。预定义宏探测同样使用 TU 的项目 `-I`，按该选项生效的 GCC specs 宏进入 `compiler` 身份。

GCC 的依赖清单可能遗漏只检查路径是否存在的运算。分析逐 TU 用 GCC 复查，把 `__has_include`、`__has_include_next` 及编译器在当前选项下提供的 `__has_embed` 替换成求值时报错的宏，并保留 `defined` 检测的结果。Linux GCC 13.3 在 C99 模式下不提供 `__has_embed`。Windows 锁定的 MinGW GCC 16.1 支持较早语言模式中的二进制资源扩展，能力由实际预处理器探测，`defined(__has_embed)` 为真时输出固定字符串字面量，specs 的同名宏无法改写它。只要运算符进入求值，或替换改变最终宏清单，分析就报错。原始宏发现、源码追踪与 Cppcheck 使用原编译环境。参见 [#567](https://github.com/g1331/autsaro/issues/567)、[#568](https://github.com/g1331/autsaro/issues/568) 与 [GCC 的二进制资源文档](https://gcc.gnu.org/onlinedocs/gcc-15.3.0/cpp/Binary-Resource-Inclusion.html)。

## 不参与判定的输入

- 包版本。每个打开的头仍在 `scope.sdkHeaders` 记录版本。
- 搜索目录不可达的 provider 文件，如 `libc.a`、`.so`、文档。预处理器和 Cppcheck 读不到它们。完整已安装库存的摘要保存在 `scope.sdkEnvironment`，并对照包清单校验。
- 包外文件及指向它们的链接：不属于任何受支持 provider 的文件，如 runner 自带的 `/usr/local/include`，以及 clang、ncurses 的链接（真实样本里 25 条链接全是这类）。把它们纳入身份会在镜像滚动时重新制造 [#564](https://github.com/g1331/autsaro/issues/564)。GCC 打开这类头时，依赖路径解析到包外真实文件，来源校验将它标为未分类，验收报 `Incomplete SDK source provenance`。`test_opening_a_header_through_a_link_to_a_non_provider_fails_acceptance` 验证了换用两个包外链接目标时库存一致、验收均拒绝。

依据：runner 镜像滚动时 `libc6-dev` 2.39-0ubuntu8.9 与 8.12 各 507 个文件，只有 `libc.a` 和 `audit/sotruss-lib.so` 两个二进制不同，`usr/include` 下 477 个头逐字节相同。按完整库存比较会让这类无关更新拒绝比较；按可达的搜索空间比较则可比。

## 两种比较

[#568](https://github.com/g1331/autsaro/issues/568) 的两项预处理校验只改变拒绝条件。通过校验的分析沿用原有宏、展开和 Cppcheck 输入，预处理器的详细日志记录实际搜索顺序。诊断指纹把这一次确切的 `c_check.py` 摘要映射到 #616 的登记身份，其他驱动版本继续使用自身的摘要。双树比较、基线复用和验收回执仍绑定实际分析代码的摘要。

| 情形 | 双树（gate） | 同源（`--baseline`） |
| --- | --- | --- |
| 搜索序列或环境变量不同 | 拒绝 `compiler` | 拒绝 `compiler` |
| 带项目 `-I` 的预定义宏不同 | 拒绝 `compiler` | 拒绝 `compiler` |
| GCC 求值文件存在性探测 | 生产阶段报错 | 同左 |
| 可达库存不同：文件不同、缺失、多出，或链接（含祖先链接）指向别处 | 拒绝 `sdk inventory: <路径>`／`link <路径>` | 拒绝 `sdk` |
| 共有的打开头字节、共有宏值不同 | 拒绝 | 拒绝 |
| 多开或少开一个头 | 允许（项目源码改 include 时发生）；经链接或 `../` 逃出搜索目录到达的头按其解析路径入库存，一侧不再打开则拒绝 | 拒绝 `sdk` |
| 仅包版本、不可达文件不同 | 可比 | 可比 |
| 语义身份 2 或 3 的旧工件 | 拒绝，须用当前工具重扫 | 同左 |

遍历与编译器解析可能不一致的来源：

- `#include "../x.h"` 逃出搜索目录：被打开的头进入库存（上表）、逐头比较并过来源校验；若只有一侧存在则双树模式拒绝。
- `#include_next`：沿搜索序列继续，序列与各目录的库存已覆盖；缺失的候选使库存不同。
- 头里的绝对路径 include：该文件被打开，同上。
- 大小写不敏感的文件系统：库存键取文件系统列出的真实大小写，打开的头取解析后的真实路径，Windows 的包含判断忽略大小写。

基线缓存的候选下载目录建在分析输出的父目录下，严格验收直接在分析工件旁重放。两条路径都在分析输出所在驱动器上重新探测搜索序列、环境变量和带项目 `-I` 的预定义宏，与记录不同就不复用。

合并后第一轮 CI 的基线由新代码现场生成，无需人工操作；旧语义身份的 master 工件因分析代码摘要不符而回到现场扫描。
