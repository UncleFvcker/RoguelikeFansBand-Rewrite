# Contract v115：装备附魔卷轴与实例强化

状态：已实现

Contract v115 接入原版卷轴 sval 16/17/18/20/21：Enchant Armor、Enchant Weapon To-Hit、Enchant Weapon To-Dam、*Enchant Armor* 与 *Enchant Weapon*。协议为 `1.115`，demo 内容包为 `1.106.0`，state hash Schema 为 `51`，active baseline 包含 405 条 exact fixtures、零 waiver。内置内容 hash 为 `9bfa2632f2be9129e39a59dad72f7bb9a64fd2f403d74c3feaee1302fb0fe459`。

## 1. 内容模型与目标

物品效果新增 `enchant-item`，三个可选分支 `toHit`、`toDamage`、`toArmor` 分别使用 `{ dice, sides, bonus }` 描述尝试次数。普通卷轴使用固定一次 `{ dice: 0, sides: 0, bonus: 1 }`；强力卷轴使用 `1d3+3`。编译器要求至少一个分支，禁止武器分支与护甲分支混用，并验证骰值与 item-only 目标模式。

武器卷轴只接受带 `weapon`、`launcher` 或 `ammunition` tag 的实例；护甲卷轴只接受 `armor`。`no-enchant` 明确禁止强化。目标可以位于背包、装备栏或玩家脚下，但不能是来源卷轴自身；缺失目标、错误目标、错误种类和远处地面物品均在消费、RNG 与 world tick 前返回 `item.use-unavailable`。

demo 新增 Accuracy、Impact、Armor Tempering、Masterwork Weapon、Masterwork Armor 五种卷轴，以及可装备的 Resonance Mail 护甲目标。

## 2. 原版概率与事务顺序

每个属性分别掷尝试次数，再按声明顺序逐次处理。当前单项强化值作为下表索引；掷 `1..1000`，只有结果严格大于失败值才成功：

```text
[5, 10, 50, 100, 200, 300, 400, 500,
 650, 800, 950, 987, 993, 995, 998, 1000]
```

强化值上限为 +15。每次尝试的 RNG 顺序与原版一致：

1. 先作堆叠门，普通堆叠的分母为 `quantity * 100`，弹药分母再除以 20；
2. 再作当前强化值的千分失败检定；
3. 神器在基础检定成功后再作 50% 二次门；
4. 成功时该单项增加 1，后续尝试立即使用新的失败率。

选中合法目标后，即使所有尝试失败也消耗卷轴、推进正常行动时间并把来源种类标为 aware。成功和全失败分别产生 `item.use-enchanted` 与 `item.use-enchantment-failed`，结构化结果记录每个分支的 attempts、successes、before 和 after。

## 3. 实例、战斗与存档

`ItemInstance` 和地面/背包/装备/怪物携带四类 save DTO 都保存 `enchantments { toHit, toDamage, toArmor }`。缺失字段按全零迁移，不补抽 RNG；任一值超过 15 时拒绝载入。物品拆分保留强化值，拾取只合并强化、质量、affix、动态 affix、设备状态与知识均兼容的实例。新增权威实例字段使 state hash 升至 Schema v51，save 容器仍为 v1。

`toHit`/`toDamage` 分别进入持有武器近战、发射器、弹药和投掷档案；发射器与实际消耗的弹药加值相加。`toArmor` 进入装备防御与 armor class 派生。Web 在地面、背包与装备行显示非零强化，并格式化成功/失败事件；显示缓存不入档。

## 4. Fixtures 与导入结果

fixtures 399–405 固定：

- 399：普通命中强化从 +0 成功到 +1；
- 400：+15 必定失败、卷轴仍消费且强化值完成存档回读；
- 401：强力武器卷轴各作 6 次尝试，命中 +5→+8、伤害 +7→+9；
- 402：强力护甲 4 次尝试，to-AC +5→+7；
- 403：20 发弹药堆通过原版弹药门并整体强化；
- 404：无目标、缺失、自身和错误类型目标均零 RNG、零 world tick 拒绝；
- 405：高位伤害强化按递减概率失败。

核心单元测试另外固定神器 50% 二次门、普通/强力卷轴确定性、四条战斗派生路径、+15 校验以及旧档缺字段迁移。

真实导入包保持 937 items、128 affixes、1260 abilities 和 4 ability books；五种卷轴退出缺口后，`scroll-effect` 从 47 降至 42。严格源校验、编译和二进制回读 hash 均为 `a727f0ef817eefe5d790699da84e88f942a23246b4fd0b4af23b96385649dc57`。

下一轮优先比较剩余召唤卷轴、解除/施加诅咒卷轴和其他世界效果。装备诅咒、解除诅咒、随机神器、重铸与强化服务不在本轮范围。
