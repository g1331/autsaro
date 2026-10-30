# 受控 owner 时钟观察出口

本记录由当前明确工程委托下的实现者作出有界项目选择，不属于6513137017的精确124键，
不扩大该批准，不把主机接口保留说成AUTOSAR必需冲突或MISRA符合。

| 规则 | 精确范围 | 形态与适用条件 | 项目选择来源 |
| --- | --- | --- | --- |
| MISRA C:2012 Rule 8.7 | `runtime/ecu/src/Ecu_Target.c` 中 `Ecu_TargetNow` | shape=`uint64_t Ecu_TargetNow ( void ) {`; 两个既有受控C99 x64目标的四个既有profile、host-batch／probe；当程序没有外部TU调用时保留真实报告，函数体仍先Ecu_TargetAssertOwner再返回私有epoch。 | 维护者 当前工程委托下精确时钟出口选择 |

R8.7为Advisory/System（主文印刷70／PDF78，TC2.15印刷6／PDF12）。
公共声明不构成第二TU使用；TC2示例明确区分其他TU的调用与仅本TU定义／调用。
#382删除没有生产消费者的Ecu_ExecutionNow后，在其v3样本揭示本函数的R8.7，
原integration/src/Ecu_Target.c:144、host-batch／probe。旧封存程序未报告本键不能推定
删除后仍无报告；各程序的raw new／removed及source seal分别保留。

实际跨TU调用仍存在于owner_probe、ecu_scheduler_control、ecu_arti_control、
host_batch_copy_timeout及zero_dlc_output控制程序，均通过Ecu_Target.h读取owner逻辑毫秒。
这些程序的使用支持交付接口用途，不反写未调用它的样本报告为误报。
epoch由目标owner推进，函数先验证初始化线程或READY下的Task owner，随后只读epoch；
不公开私有对象，不增加同步／状态，不允许外部native线程任意观察未同步的epoch。

选择保留一个已有、具备真实控制消费者的外部链接出口。删除或静态化将破坏这些消费者；
头内实现还需暴露私有epoch或另增取值出口，并非仅搬动无状态函数体。
按程序裁剪导出、改造消费者或重新设计时钟接口仍是可行替代，本次不改公开合同，
不恢复无消费者的生成wrapper，也不制造虚假调用使样本消警。
风险是无意调用与同名外部符号；既有Ecu前缀、owner拒绝、公开头及实际控制程序约束用途。
目标、时钟单位、owner／并发、消费者或程序组成变化时重评；不按规则、文件或前缀外推。

新增源码仅为MR12注释，函数体、签名与ABI不变；双向记录／形态关联和限定独立审阅
验证这项处置，既有消费者运行证据复用。具体profile报告及后续CI归各自PR收据，
不将来源于#382删除的报告记成旧#381分析身份或抹去全部not_assessed／refusal。
