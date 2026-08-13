# Contract v298：通用种族等级变异奖励

状态：已实现。Protocol `1.199`，State Hash Schema `v98`，内容包 `1.314.0`，save v1；
active baseline 为 `contract-v298`，共 26 条 exact fixture、零 waiver。

## 范围

`RaceDefinition.levelMutationRewards` 支持两种当前所需的奖励选择：`choice` 明确列出玩家
候选，`casting-attribute` 按职业施法属性选择变异并为无施法属性职业提供默认项。奖励按
最低等级和稳定 ID 顺序处理；自动项立即获得并锁定，玩家选择项通过零时间、零资源、零
RNG 的 `ChooseRaceMutation` 命令完成。

待选择状态不单独持久化。核心根据当前等级、种族配置和既有 `lockedMutationIds` 派生第一
个未完成选择；因此降级不会移除已经锁定的奖励，重新升级也不会重复发放。待选择期间除
选择命令外的游戏命令全部拒绝。Web 端在现有变异面板顶部显示强制选择卡，候选继续使用
既有变异名称、说明和评级。

内容编译拒绝空候选、未知或重复变异、重复奖励 ID、非正等级，以及 `randomWeight` 非零的
奖励变异。该约束让锁定集合能够无歧义地表达完成状态，并避免等级奖励进入随机变异池。

## 内容与契约影响

本提交只以测试专用 Race 验证选择、职业施法属性映射、跨级自动奖励、命令门禁、降级、
重新升级和存档派生；正式 `demo.race.rfb-human` 尚未配置 20 级候选池或 35 级弱点。
因此内容包、内容哈希、公共初始化、权威状态和 RNG 顺序均不改变。Protocol 因新增命令与
投影升至 1.199；State Hash Schema 与 save 保持不变。26 条 active fixture 经复验零漂移，
不刷新 assertions。
