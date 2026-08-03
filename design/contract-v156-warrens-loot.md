<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Contract v156: Warrens floor and monster loot

状态：active baseline。协议保持 `1.123`，save 容器保持 v1，state hash Schema 保持 `55`。demo 内容包升至 `1.146.0`，content hash 为 `91ac518116420421305410a9435e002648c5538deba102780ce5e1359d7e33be`；active baseline 继续包含 456 条 exact fixture、零 waiver。

## 固定来源结论

固定 RFB v1.3.0.7 来源中，普通地牢生成分别调用 `randnor(8, 3)` 分配房间物品、`randnor(2, 3)` 分配全图物品，再按小地图面积随机舍入。Warrens 的 66×22 面积是标准 66×198 地图的 `1/9`。Small kobold 声明 `DROP_60` 与 `DROP_WARRIOR`；Giant white mouse 和 Warg 没有普通物品掉落旗标。Mughash 声明 `ONLY_ITEM | DROP_1D2 | DROP_GOOD` 与 `DROP_WARRIOR`。四类怪物按默认 `one_in_(3)` 尝试生成遗骸；同时允许尸体与骨骸时，非过量伤害的常规分支约为 4:1。Warrens 最终对象 `FINAL_OBJECT_75_29` 是一瓶 Speed 药水。

## 实现

- loot table 条目新增 `minDepth/maxDepth`；表可用 `rollChancePercent` 表达单次百分比掉落，也可用 `rollDice` 在固定 roll 之外增加掉落数量骰。旧表缺少这些字段时保持原有固定 roll 与 RNG 顺序。
- 程序化楼层可声明独立的房间物品与全图物品正态分配。当前实现以 12 次独立 d6 的中心和近似正态偏移，不复制旧版正态表；结果按地图面积随机舍入，Warrens 每类至少保留一件，总预算为 2–5 件。
- 楼层物品从当前已实现且有实际用途的低层物品中按原版分配深度过滤；同一原版分配稀有度的候选保持等权。房间物品只放在房间格，全图物品可放在房间或走廊，二者分别消耗确定性 RNG。
- Small kobold 每次死亡有 60% 生成一件当前可承载的普通/Warrior 主题物品。其原版对象等级按 `(dungeon depth + monster level) / 2` 映射，因此 3 级皮具在 Warrens 深度 5 起进入候选，5 级武器在深度 9 起进入候选。
- Giant white mouse 与 Warg 不生成普通物品。mouse 的遗骸只可能是尸体；kobold、Warg 和 Mughash 成功生成遗骸时按 4:1 选择尸体或骨骸。只有一种遗骸候选时不额外抽选择 RNG。
- Mughash 死亡生成 `1d2` 件 Fine 品质的当前 Warrior 主题装备；最终守卫奖励使用独立表，额外必掉一瓶 Speed 药水。守卫奖励与普通死亡表分别生成并共同进入既有死亡、事件、存档和 state-hash 事务。
- 新增 Leather Gloves、Soft Leather Boots、Hard Leather Cap、Small Leather Shield、Sabre、Spear 与 Skeleton Remains；名称和数值按本次用户指定的固定来源字段录入，描述保持独立撰写。

## 暂缓边界

- 原版还会生成金币，并以概率保证食物、光源和较深层 Recall。当前旅程没有经济、饥饿或照明消耗系统，因此本批不生成无功能对象；Recall 已作为普通可用卷轴进入深层候选。
- 原版对象选择有低概率 `GREAT_OBJ` 深度提升。当前 loot table 没有这一通用对象等级提升阶段；最低分配深度 10 的 Teleportation/Farstep 不作为 1–9 层常规 Warrens 掉落。
- `Fine` 是当前模型对 `DROP_GOOD` 的品质表达；旧版 `apply_magic` 的完整附魔、ego、神器与全量 Warrior 主题池仍未导入。
- 不增加 ambient 无限刷怪。返回地表后重新进入仍按既有 `reset-on-surface` 生命周期刷新地图、怪物和物品。

## 验收

- 内容验证覆盖非法百分比、非法数量骰、反转深度范围、悬空 guardian reward，以及遗骸概率、权重和物品引用错误。
- 16-seed 九层矩阵固定每层 2–5 件地面物品、浅层深度过滤、深层候选可达、同 seed 确定性和 stored-floor 往返持久性。
- 128-seed 死亡矩阵覆盖 Small kobold 的有/无普通掉落、无遗骸/尸体/骨骸三支；mouse 与 Warg 被固定为无普通物品掉落。
- 完整 Warrens 胜利测试固定 Mughash 的 `1d2` 件 Fine 装备、一瓶 Speed 药水、掉落位置、胜利事件顺序与存档往返。
