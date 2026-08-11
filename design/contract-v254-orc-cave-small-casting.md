# Contract v254：Orc Cave 小型施法机制

本批以权威 RFB `master` 为来源，扩展既有怪物专用 `jump-damage`：显式骰仍按
原参数掷骰；无显式参数的元素跳跃以 `0d0 + 怪物等级` 表达，因此伤害阶段不新增
RNG，再统一乘以原版怪物侧 `5/4`、作用于施法者周围半径 5，最后在半径 10 内
闪现。正式映射火焰、毒素、混乱和黑暗四种既有伤害类型。

`S_HYDRA` 继续使用 `summon-category`。原版以 glyph `M` 定义 Hydra 类别，数量
为 `1d3+1`，最大等级取施法者等级。Zoopi 的 `S_SPECIAL` 只映射为一条窄的
Gelatinous Cube 分类召唤，生成 `1d3` 只等级 16 的凝胶方块；其他
`S_SPECIAL` 仍然是诚实缺口，没有增加通用特殊召唤脚本。Ninja 的
`DROP_NINJA` 使用正式包中已存在的 Dagger、Falcon Sword、Ninjato 和
Sealed Amulet 建立窄掉落表，不引入缺少运行时行为的新物品。

新增 It、Zoopi、Gachapin、Xylibbogaz、Mario、Luigi 和 Ninja 共 7 条
Orc Cave actor。审计现为 186 imported、176 selected、0 direct、22 blocked、
28 excluded 和 1 guardian。内容包升级到 1.245.0，共 602 actors、263 abilities
和 13 loot tables，内容 hash 为
`7c978853943a6bb4a81e46072bbd089cad526bfc0688711664c3ce709a4aa217`。

协议保持 1.169，State Hash Schema 保持 v85，save 容器保持 v1；active baseline
推进到 contract-v254，继续保留 22 个聚焦 exact fixture，不恢复旧 E2E。
