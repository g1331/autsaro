# 资产清单维护

构建时核对运行时、补丁、目标锁和交付工具的原始字节身份。这个约束继续保留；普通 build/check 不接受新的源码身份。

## 项目源码变更

~~~sh
uv run dev assets check
uv run dev assets update
git diff -- runtime/contracts scripts/ecu_tools/workbench-v2-assets.json
~~~

update 显式更新项目所有的 BSW、交付资产及 workbench 工具清单，输出发生变化的源文件。先审阅源码，再审阅清单 diff，最后重新运行相应测试。更新只重算已列出的项目资产，不发现并自动接纳新文件；新增资产必须明确填写角色、许可、目标及责任信息。

所有上游文件先核对固定清单。第三方来源被改动时，update 拒绝运行，不自动改写 FreeRTOS 身份。多份派生清单更新不是跨文件事务；中断后重新运行 check/update 并审阅完整 diff。

## 行尾与新 checkout

.gitattributes 固定实际交付字节；部分历史运行时使用 CRLF，其余交付工具和补丁使用 LF。不要全仓归一化行尾，也不要为消除环境错误直接改锁定哈希。

新增交付文件时同时核对 Git 属性、清单和干净 checkout。assets check 应在新 checkout 中通过；编辑器保存后出现 mismatch，先检查行尾，再检查实际源码修改。

## 上游及工具链升级

FreeRTOS 或固定 GCC/binutils 升级是独立改动：核对来源、许可、上游 commit/tag、源清单与目标锁，审阅补丁兼容性并完整执行原生消费者。不要用项目资产更新命令吸收这些变化。
