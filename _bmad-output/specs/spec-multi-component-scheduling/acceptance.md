# 独立验收预期

期望在实现前冻结。真实 OS／RTE／BSW 用正常 Cargo／Python 测试和离线工程入口执行，不用函数桩、源码文本断言或纯编译替代行为。

| ID | 输入／动作 | 独立预期 |
| --- | --- | --- |
| AC-1 | 原创多文件三应用组合：Ingress、Process、Observe；common period=10ms，positions依次4／5／6，原Com发送与Dcm顺延；Ingress CAN RX为0x320／4byte，TX0x321绑定Ingress同一接收值，DID0x1234的byte[4] OUT server绑定Ingress提交值并由DcmService调用；组件／端口名另做变名向量 | 所有full identity、ECU mapping、类型和runnable来自真实输入，非模板名硬编码；生成两次字节一致；Process和Observe均为纯local应用 |
| AC-2 | epoch0注入CAN78563412；epoch10 Ingress发布local uint32 0x12345678并提交相同CAN／DID值；Process用户测试算法加1并发布；Observe消费、经自身Rte_Call调用Process的uint32 OUT服务，复制Process提交值 | Observe=0x12345679，scalar服务OUT同值；顺序=[Ingress,Process,Observe]，server在Observe调用上下文；TX0x321/4=78563412；UDS221234响应62123412345678，SF0762123412345678；未运行首周期前DID四个00 |
| AC-3 | 每个 common tick；epoch10 后重复 COMMIT10，再推进到20 | 10／20各周期 runnable执行一次，同epoch不重复；服务器仅在真正调用时执行，顺序由位置决定，不依赖源文件顺序 |
| AC-4 | 一个P连接两个R、各endpoint明确init：P=1／R1=7／R2=9；R均handleNeverReceived=false／aliveTimeout0／NONE；两个接收组件同端口短名；随后write42；新multi已启动Rx组及未分组Tx PDU停止CAN后Read／Write及实际Tx；Com_DeInit后再调用 | 首次R1=7／R2=9，均E_OK；首次write后两个R=42／E_OK，独立无覆写。local handleNeverReceived=true、缺init分别拒绝；停止CAN仍能在合法owner执行本地通信。新multi网络Read仍回填last/init与真实freshness，Write接受buffer更新E_OK，lower Tx失败独立；不把CAN停止报告为COM_STOPPED。未初始化／DeInit后按所选错误契约拒绝且不伪成功；旧profile保持历史失败状态 |
| AC-5 | P→P、R→R、错误instance ownership、悬空DEST、同一R两个producer、local+network双绑定、未连接已使用端口 | 生成前定位到来源／完整对象路径拒绝；既有输入、产物和会话保持，无部分工程 |
| AC-6 | 相同bit width但不同接口／不兼容应用或implementation type、缺映射、不同data element／operation、C/S参数方向或array长度不一致 | 生成前拒绝；不自动cast／alias／按SHORT-NAME猜类型 |
| AC-7 | 缺event／instance／ECU mapping、重复位置、同runnable多TimingEvent、task不属owner、period与alarm不符、非法OsEvent；server缺无task的RteEventToTaskMapping容器或增加taskRef／周期触发；同类型重复实例 | 明确拒绝且无副作用；server合法无task容器保留，不将“无taskRef”误当作“无mapping”；不同于保留无关有效输入 |
| AC-8 | 同步服务成功、空OUT／INOUT、未支持应用错误、递归call graph／异步server point | server无possibleErrors时编译为void，客户端Std_ReturnType／E_OK；成功值匹配；拒绝不能修改输出或伪成功，不绕过RTE直调server；无配置服务不生成空壳API |
| AC-9 | 按真实槽create-only初始化；修改每份用户C；重新生成；旧预览后再改一份live源／manifest；在一个初始化路径预先放用户文件 | 逐份源码原字节保留；外部变化使旧确认拒绝；已存在文件不覆盖，失败不留下部分manifest／live成员 |
| AC-10 | 完整v2 multi工程移到新目录；重导入→新目录生成→离线build→真实运行AC-2；篡改sealed application或owner／producer字段 | 新目录保持所有输入／源码字节和完整producer闭包；构建／运行成功；篡改拒绝，路径不泄漏原机器绝对路径 |
| AC-11 | 旧host-v1、单组件标准ECU、旧v1／当前v2交接与源码初始化 | 保留正式已有成功／拒绝回归，不因multi默认改变旧API／wire格式／原始输入 |
| AC-12 | 对实际生成的multi和受影响single profile运行c-check | 记录工具身份、真实翻译单元／链接范围、完整性、退出码和源码诊断；没有扫描完整性／人工规范证据时不声明完整MISRA符合 |

先完成 AC-1–AC-8 的真实生成／调度关口，再展开依赖它的编辑与 AC-9–AC-10 交接。2026-10-10 用户撤回 8.5 延期安排：8.1–8.4 阶段完成后提交、创建 PR、通过必需 CI 并合并，随后在本任务继续 8.5。AC-10 的独立多组件搬移／恢复／再生成／离线构建运行闭包是整个 Epic 完成门槛，未执行前不记作通过；它不阻塞先行的 8.1–8.4 阶段合并。既有源码保护、所选生成／运行及受影响 profile 的实际 c-check 证据仍按各故事核验，不因分阶段合并降低要求。全部必需验收完成前 Epic 8 保持进行中，真实MCU、硬实时、认证和未执行的平台／安装验收保持未验证。

内部值、计数、执行顺序与server调用上下文由实际用户测试应用记录不可变观测，使用既有probe／test的工程外 `--control-source` 在真实OS owner阶段读取，宿主线程仅输出已复制结果。生产HostBatch另验上述固定CAN／DID和重复epoch向量；控制源不能链接进生产入口，不新增产品调度／诊断接口或函数桩。

8.2范围已由用户确定：最小真实Rx group＋标准DM及COM→RTE回调。网络独立向量：ComFirstTimeout=0，首次接收前任意tick仍NEVER_RECEIVED且保留明确初值；接收后30ms到期产生真实timeout通知，Read返回MAX_AGE_EXCEEDED并保留last value，持续无接收按每30ms重复通知；同epoch先接收后DM检查避免边界假超时。DisableReceptionDM后不通知且不停止接收；Enable重新按first timeout启动，重复enable不重置已启用timer；GroupStop禁收并取消DM，Read返回实际COM_STOPPED，Start(FALSE)保留buffer，Start(TRUE)按init重置；恢复帧清除超时并回填新值。零ComTimeout禁用监测，错误/重复group handle、dangling或Tx成员、缺member和不一致timebase/timeout/callback生成前拒绝。原有旧profile失败语义与Dem主机计时保持回归；该向量只定义预期，未冒称已实现。
