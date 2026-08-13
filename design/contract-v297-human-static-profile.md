# Contract v297：Human 静态资料收口

状态：已实现。Protocol `1.198`，State Hash Schema `v98`，内容包 `1.314.0`，save v1；
active baseline 为 `contract-v297`，共 26 条 exact fixture、零 waiver。

## 权威来源与范围

本批以 Frogcomposband/RFB `master`（`efd63661302866038f58d8cd2553b23e6af3bf9d`）的
`src/races_a.c` 中 `human_get_race` 为权威来源，
只收口 Human 的静态种族资料：六项属性修正均为 0，生命、经验与商店倍率均为 100%，
基础 HP 为 20，察觉基础值为 +10。Standard 类人身体、个人主义美德选择和 Human kin
沿用既有通用机制。

中英文说明明确记录原版 20 级特殊天赋与 35 级人类弱点；本批不实现这两个等级机制，
也不预造对应能力或状态字段。

## 契约影响

七个正式 build 均继续引用 `demo.race.rfb-human`。基础 HP 与察觉参与共同角色初始化，
因此 26 条 active fixture 全量刷新并复验。此次没有协议字段、权威状态结构、RNG 顺序
或存档容器变化，Protocol、State Hash Schema 与 save 版本保持不变。
