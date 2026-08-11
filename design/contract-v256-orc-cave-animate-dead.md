# Contract v256：Orc Cave 尸体复苏

本批以权威 RFB `master` 为来源，将怪物法术 `ANIM_DEAD` 接入既有
`AnimateDead`、地面遗骸和召唤事务，不增加第二套法术脚本。

## 行为边界

- 以施法者为中心检查半径 5 内最多 8 个对应遗骸。
- 尸体有 20% 几率化为尘埃，骸骨有 40% 几率化为尘埃；无论成败都消耗遗骸。
- 成功时在遗骸位置生成永久、敌对的 `risen-thrall`，并记录施法者为召唤来源。
- 没有可复苏遗骸时该能力不进入怪物施法候选，不额外消耗 RNG。
- 玩家已有的 Death `AnimateDead` 默认失败率仍为零，行为与 RNG 顺序不变。

## 内容边界

Arch-vile 与 Orc warlock 已进入 Orc Cave 选择清单。Kharis the Powerslave
仍被接触光环阻塞，Ghoulking 仍被 `UNLIFE` 近战阻塞；本批不顺带实现这些机制。

内容包升级到 1.247.0，共 607 actors、266 abilities 和 13 loot tables；严格
同步 560 条。协议保持 1.169，State Hash Schema 保持 v85，save 容器保持 v1。
基线推进到 contract-v256，继续保留 22 个聚焦 exact fixture，不恢复旧 E2E。
