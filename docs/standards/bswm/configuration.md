# BswM：ECUC 配置与选择边界

本篇属于 [BswM](README.md)。条款来自固定 R24-11 原始 SpecificationsARXML；标记仅作用于同一格中紧随其后的编号。行为测试的 `Verifies:` 与这里的处置共同核算。

内置 ECUC metadata 只定义已选配置涉及的容器和参数，未定义的容器与参数在生成前被拒绝；已定义参数的取值范围由 mode.rs 进一步收窄，未收窄的取值同样在源对象位置拒绝。

| 对象／条件 | 处置 |
| --- | --- |
| BswM 模块、BswMGeneral 与 ComM 开关 | 已核对（内置 ECUC metadata 与 mode.rs 校验：BswMGeneral 恰一个、BswMComMEnabled 必须为 true、BswMUserIncludeFile 只接受 Ecu_HostBusSM.h）：ECUC_BswM_00800、00939、00954、00955、01063 |
| BswMGeneral 的模块开关定义 | 已核对（内置 ECUC metadata 保留 Boolean／默认 false；mode.rs 的 disabled() 对其余开关只接受 false，配置测试 mode_features_outside_the_selection_are_refused_at_their_source 的 Rejects 覆盖）：ECUC_BswM_00811、00812、00938、00940–00949、00965、00987、01047、01071、01072、01093 |
| 仲裁容器、模式请求端口与 ComM 指示源 | 已核对（内置 ECUC metadata 与 mode.rs 校验：BswMArbitration 恰一个，BswMModeRequestPort 恰一个且 BSWM_IMMEDIATE，来源仅 BswMComMIndication 且引用 ComM 通道，初值 COMM_NO_COMMUNICATION）：ECUC_BswM_00801、00805、00822、00856、00880、00883、00928、00932 |
| 模式条件与逻辑表达式 | 已核对（内置 ECUC metadata 与 mode.rs 校验：每条规则一个 BSWM_EQUALS 的 BswMModeCondition，BswMBswMode 取 COMM 三种模式，表达式为 BSWM_AND 或省略）：ECUC_BswM_00807、00808、00814–00816、00820、00821、00866、00869 |
| 规则 | 已核对（内置 ECUC metadata 与 mode.rs 校验：恰三条规则、BSWM_FALSE、无 false 列表、不嵌套执行、true 列表唯一）：ECUC_BswM_00806、00817–00819、00888、00935 |
| 模式控制、动作与动作列表 | 已核对（内置 ECUC metadata 与 mode.rs 校验：BswMModeControl 恰一个，动作只能是 BswMUserCallout，调用文本须为固定的 Ecu_HostBusSM_ApplyMode，动作列表 BSWM_CONDITION、Priority 0 或省略、单个 Index 0 且 AbortOnFail false、引用 BswMAction）：ECUC_BswM_00802、00809、00810、00823–00826、00834、00843、00894、00902、01076 |
| BswMConfig | 已核对（内置 ECUC metadata 与 mode.rs 校验：BswMConfig 只含 BswMArbitration 与 BswMModeControl，不含分区引用与数据类型映射）：ECUC_BswM_00895 |
| CanSM 输入 | 不适用（配置：ECUC_BswM_00938）：ECUC_BswM_00857、00870 |
| Dcm 输入 | 不适用（配置：ECUC_BswM_00940）：ECUC_BswM_00863、00925、00969 |
| EcuM 输入与动作 | 不适用（配置：ECUC_BswM_00941）：ECUC_BswM_00904、00905、00961–00964、00993、00994、01044、01045、01064、01065、01085–01087 |
| EthIf 输入与动作 | 不适用（配置：ECUC_BswM_01072）：ECUC_BswM_01066–01070、01088 |
| EthSM 输入 | 不适用（配置：ECUC_BswM_00942）：ECUC_BswM_00860、00873 |
| FrSM 输入与动作 | 不适用（配置：ECUC_BswM_00943）：ECUC_BswM_00858、00872、01037、01038 |
| 通用请求 | 不适用（配置：ECUC_BswM_00949）：ECUC_BswM_00861、00874 |
| J1939Dcm 输入与动作 | 不适用（配置：ECUC_BswM_00987）：ECUC_BswM_00985、00988、01032–01035 |
| J1939Nm 输入 | 不适用（配置：ECUC_BswM_00965）：ECUC_BswM_00966、00967、00997 |
| LinSM 输入与动作 | 不适用（配置：ECUC_BswM_00944）：ECUC_BswM_00827、00842、00859、00871、00885、00886、01028 |
| LinTp 输入 | 不适用（配置：ECUC_BswM_00945）：ECUC_BswM_00914、00915 |
| Nm 输入与动作 | 不适用（配置：ECUC_BswM_01071）：ECUC_BswM_00837、00838、00999、01049、01075、01081、01082 |
| NvM 输入 | 不适用（配置：ECUC_BswM_00946）：ECUC_BswM_00890、00891、00956、00957 |
| Sd 输入与服务动作 | 不适用（配置：ECUC_BswM_01047）：ECUC_BswM_00974、01004–01013、01015–01017、01077–01079 |
| ComM PNC 请求 | 不适用（配置：ECUC_ComM_00839）：ECUC_BswM_00922、00924 |
| ComM 复位事件 | 不适用（配置：ECUC_ComM_00558）：ECUC_BswM_01014 |
| ComM 模式限制动作 | 不适用（配置：ECUC_ComM_00560）：ECUC_BswM_00908、00910、00911 |
| 其他模式请求源与定时器 | 不适用（配置：ECUC_BswM_00856）：ECUC_BswM_00862、00892、00893、00926、00927、01046、01058、01091、01092 |
| 事件请求端口、事件源与事件动作 | 不适用（配置：ECUC_BswM_01052）：ECUC_BswM_00989、00990、01030、01052–01056、01083、01084 |
| BswMCompuScaleModeValue 与 ModeDeclaration 条件值 | 不适用（配置：ECUC_BswM_00816）：ECUC_BswM_00864、00868、01039–01041 |
| SW-C 模式请求端口与模式开关端口 | 不适用（配置：ECUC_BswM_00802）：ECUC_BswM_00950、00951、01022、01025、01031、01057 |
| 分区引用与数据类型映射 | 不适用（配置：ECUC_BswM_00895）：ECUC_BswM_00936、00937、00984 |
| 运行时错误 Id | 不适用（配置：ECUC_BswM_01050）：ECUC_BswM_01050 |
| 未由 BswMUserCallout 承担的动作 | 不适用（配置：ECUC_BswM_00826）：ECUC_BswM_00145、00803、00828–00831、00840、00841、00849–00855、00896–00901、00906、00907、00909、00912、00913、00918、00952、00958–00960、00970–00972、00998、01000–01002、01021、01023、01024、01036、01059–01062、01073、01074、01080、01089、01090、01094–01096 |
| BswM 主函数周期 | 不适用（配置：ECUC_BswM_00813）：ECUC_BswM_00813 |
