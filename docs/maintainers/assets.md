# 资产清单维护

`runtime/contracts/assets.json` 登记运行时交付范围、路径、角色、许可、目标、profile 与责任，不保存一方文件的 SHA-256。`core/build.rs` 从实际检出字节计算并嵌入摘要；安装资源必须匹配构建时的来源，篡改仍报 `backend.resources.identity_differs`。

## 项目源码变更

~~~sh
uv run --locked python -m autosar_tooling assets check
~~~

check 核对运行时清单结构、文件存在性与上游固定身份。运行时源码修改不需要更新摘要；新增资产仍须明确登记元数据。构建只写 OUT_DIR，在其中生成带摘要的交付版 `meta/assets.json`，保持原 v2 格式与字节排版；来源清单自身按其原始字节检查。`meta/files.sha256` 继续封存真实交付内容。

`tools/python/src/ecu_tools/build-tool-inventory.json` 固定独立 seal guard `sealed_package.py` 的身份，与运行时资产清单分开。修改此工具并审阅后执行 `uv run --locked python -m autosar_tooling assets update`，该命令只更新此工具清单。FreeRTOS 有任何未登记变化时 update 拒绝执行。

BSW ABI、交付契约与第三方身份须单独审阅并将变化写入 CHANGELOG，不能用机械摘要更新代替。规则集清单包含 build.rs 和生成器源码，因此修改它们会改变规则集身份及引用身份的 seal 元数据；不得冻结旧摘要掩盖来源变化。

项目原创资产使用 SPDX Apache-2.0；LICENSE 和 NOTICE 随工程交付。FreeRTOS 保留 MIT 与固定上游身份。

## 行尾与新 checkout

.gitattributes 固定实际交付字节；指定的运行时文件使用 CRLF，其余指定文件使用 LF。编辑 CRLF 文件时按字节保留换行，不全仓归一化。`git ls-files --eol` 可检查实际检出状态。

`tests/python/test_development.py` 的 AssetLineEndingTests 通过 Git 属性检查固定换行，并在违反 `eol=lf` 或 `eol=crlf` 时指出具体文件。用正常入口运行：

~~~sh
uv run --locked python -B -m unittest discover -s tests/python -p test_development.py
~~~

## 上游及工具链升级

FreeRTOS 或固定 GCC/binutils 升级是独立改动：核对来源、许可、上游 commit/tag、两份 source manifest 与目标锁，审阅补丁兼容性并完整执行原生消费者。构建继续对 FreeRTOS 内核和端口实际字节与 source manifest 比较；source-manifest.json 的 `patches` map 单独固定受控采用补丁的仓库相对路径与摘要，update 不接纳补丁变化；不要用项目资产更新命令吸收上游变化。

## 补丁后内核身份

Cargo 构建需要 Git（默认 `git`，可用 `AUTOSAR_GIT` 指定），在 OUT_DIR 中按共享目标定义为两个目标复制已校验的内核、应用各自补丁并计算最终字节摘要。所有内核补丁入口用命令级 `core.autocrlf=false` 保留固定来源的换行字节，不受用户全局 Git 换行配置影响。生成器把完整文件 map 写入 `autosar-build-target-v3` 的 `kernelSha256`，不维护手抄的结果摘要。包内工具及 CMake 在编译前核对补丁后的副本，错误指出 `kernel/` 下的文件。旧构建清单须重新生成。

这不改变 source manifest 的上游／补丁身份校验，也不需要 `assets update`；只有 seal guard 本身改变才更新独立清单。规则集与封存身份仍随实际来源重算。

链接检查以所选包根／构建输出及内核根为边界，根本身和树内的 symlink／reparse point 仍拒绝；宿主祖先目录的系统别名（如 macOS `/var`、`/tmp`）不属于交付树。不要通过 resolve 树内成员或仅调整测试临时目录来绕过链接检查。
