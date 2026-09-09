# E7 非 Craft Ego 导入与运行时审计

核对日期：2026-09-09。权威来源为 `D:/codex/Frogcomposband/master` 的 Git `master` 对象，提交 `a0d92b6378d148c5262cc236b8fa6ed2ca06a54c`。

38 条包含首饰 17、光源 9、箭袋 4、装置 7、特殊 1。逐条比对 source index、原版中英文名、rarity 和生成等级，全部一致；中文 unresolved 为 0。原版名称中的 `&`、`~` 等格式标记原样保留。

## 身份与入口

| source index | 权威中文名 | 类型 | 稀有度 | 稳定 affix ID 后缀 | 入口 |
| ---: | --- | --- | ---: | --- | --- |
| 200 | (防御者的) | RING / AMULET | 6 | `defender-jewelry` | 对应类型自然生成 |
| 201 | (元素的) | RING / AMULET | 2 | `elemental-jewelry` | 对应类型自然生成 |
| 205 | 保护之 | RING | 2 | `protection-ring` | 对应类型自然生成 |
| 206 | 战斗之 | RING | 2 | `combat-ring` | 对应类型自然生成 |
| 207 | 箭术之 | RING | 3 | `archery-ring` | 对应类型自然生成 |
| 208 | 巫术之 | RING | 3 | `wizardry-ring` | 对应类型自然生成 |
| 209 | 速度之 | RING | 5 | `speed-ring` | 对应类型自然生成 |
| 210 | 戒灵的 | RING | 0 | `nazgul-ring` | 显式指定；禁止普通随机池 |
| 211 | 矮人的 | RING | 0 | `dwarves-ring` | 显式指定；禁止普通随机池 |
| 220 | & 野蛮人护身符~ | AMULET | 2 | `barbarian-talisman` | 对应类型自然生成 |
| 221 | & 神圣吊坠~ | AMULET | 2 | `sacred-pendant` | 对应类型自然生成 |
| 222 | & 地狱挽具~ | AMULET | 3 | `hell-harness` | 对应类型自然生成 |
| 223 | & 矮人项链~ | AMULET | 2 | `dwarven-necklace` | 对应类型自然生成 |
| 224 | 贤者的 | AMULET | 3 | `magi-amulet` | 对应类型自然生成 |
| 225 | & 英雄项圈~ | AMULET | 16 | `hero-torc` | 对应类型自然生成 |
| 226 | 奉献之 | AMULET | 3 | `devotion-amulet` | 对应类型自然生成 |
| 227 | 诡计之 | AMULET | 3 | `trickery-amulet` | 对应类型自然生成 |
| 235 | 额外光明之 | LITE | 1 | `extra-light-light` | 对应类型自然生成 |
| 236 | 照明之 | LITE | 1 | `illumination-light` | 对应类型自然生成 |
| 237 | 持续之 | LITE | 1 | `duration-light` | 对应类型自然生成 |
| 238 | 红外视觉之 | LITE | 2 | `infravision-light` | 对应类型自然生成 |
| 239 | 献祭之 | LITE | 4 | `immolation-light` | 对应类型自然生成 |
| 240 | 黑暗之 | LITE | 8 | `darkness-light` | 对应类型自然生成 |
| 241 | 不朽之眼之 | LITE | 8 | `immortal-eye-light` | 对应类型自然生成 |
| 242 | 维林诺的 | LITE | 16 | `valinor-light` | 对应类型自然生成 |
| 243 | 探知之 | LITE | 8 | `scrying-light` | 对应类型自然生成 |
| 250 | 抗性之 | DEVICE | 1 | `resistance-device` | 对应类型自然生成 |
| 251 | 容量之 | DEVICE | 1 | `capacity-device` | 对应类型自然生成 |
| 252 | 再生之 | DEVICE | 2 | `regeneration-device` | 对应类型自然生成 |
| 253 | 简易之 | DEVICE | 1 | `simplicity-device` | 对应类型自然生成 |
| 254 | 力量之 | DEVICE | 8 | `power-device` | 对应类型自然生成 |
| 255 | 容纳之 | DEVICE | 3 | `holding-device` | 对应类型自然生成 |
| 256 | 迅捷之 | DEVICE | 16 | `quickness-device` | 对应类型自然生成 |
| 260 | (炸毁的) | SPECIAL | 0 | `blasted` | 装备诅咒强制替换 |
| 265 | 容纳之 | QUIVER | 1 | `holding-quiver` | 对应类型自然生成 |
| 266 | 保护之 | QUIVER | 1 | `quiver-protection` | 对应类型自然生成 |
| 267 | & 无尽箭袋~ | QUIVER | 4 | `endless-quiver` | 对应类型自然生成 |
| 268 | & 相位箭袋~ | QUIVER | 4 | `phase-quiver` | 对应类型自然生成 |

35 条可进入对应类型的普通随机池；210、211、260 的 rarity 为 0。复用既有 elemental-jewelry、wizardry-ring、magi-amulet、sacred-pendant、quiver-protection 五个 ID；正式包现在有 169 条 affix，其中 160 条具有唯一原版 source 身份，另外 9 条为既有任务辅助或 demo 定义。

## 实际生成与消费者

- 首饰：以 `ego.c` 的类型分支选择、共享 pval、能力次数和等级判定物化 17 条；普通首饰不依赖武器的 exceptional 门槛。属性、抗性、免疫、slay/brand、诅咒和随机激活均落入实例。巫术戒指/贤者护符的伤害加值进入法术；箭术戒指进入射击；战斗戒指的命中、伤害、额外攻击、伤害骰和 brand 跟随对应持武器的手，另一只戒指仅在双手持握时合并。
- 光源：九条分别连接光照、黑暗、燃料消耗、红外视觉、隐形感知、感知类别和激活。自然生成保留燃料随机化及 good 燃料光源的升级判定；持续之减慢燃料消耗，黑暗之清空燃料。维林诺的限定费诺提灯；探知之不用于消耗燃料的光源。
- 箭袋：生成容量从 60 起按连续二选一追加 10；good 加 20，ego 加 50，容纳之先加倍。相位箭袋去除箭袋及实际装入弹药的重量，超过容量的弹药仍计重。保护之作用于弹药破坏；无尽箭袋按当前发射器类型补普通弹药，每次至多 50 且不超过容量，完全鉴定并记录来源。
- 装置：先初始化激活与能量，再选择 ego；抗性之跳过 rod 类型和已经免疫酸的底材。容量、简易、再生、力量、迅捷使用实例 pval；分别连接能量上限、检定难度、恢复进度、装置威力与行动能量。抗性和容纳分别接入元素破坏保护与充能吸取保护。ego 不覆盖已经选出的装置效果。
- `(炸毁的)`：由诅咒武器/护甲及相应混沌神奖励强制替换，普通随机池不可达。清除既有 ego/正式神器身份和实例属性，归零基础 AC 或武器骰子，施加负附魔并保留源版保留的激活；价值为零，库存价值判断同步。正式神器通过原有 `artifactGeneration.baseItemKindId` 返回底材。

代码入口：[首饰物化](../crates/rfb-core/src/game/ego/jewelry.rs)、[光源/箭袋/装置物化](../crates/rfb-core/src/game/ego/noncraft.rs)、[自然生成调度](../crates/rfb-core/src/game/loot.rs)、[物品效果](../crates/rfb-core/src/game/item_use.rs)、[箭袋与装备诅咒](../crates/rfb-core/src/game/inventory.rs)。导入使用 [noncraft_egos.rs](../crates/rfb-legacy-import/src/content/noncraft_egos.rs)，从 Git 对象读取原版名称与设备激活名。

## 审计 flag 与共用激活

`audit-egos` 的 `currentImporterExpressible` / `unmappedFlags` 只描述旧静态转换器，不能作为运行时完成数。

| 原始审计项 | E7 处理 |
| --- | --- |
| SPEED、STEALTH、DEC_STEALTH、TUNNEL、INFRA、DEC_LIFE | 在对应首饰/光源分支按共享 pval 物化，进入实际速度、潜行、挖掘、视觉和生命计算 |
| DEVICE_POWER、装置 SPEED | 装置实例 pval 进入实际威力和使用能量；不误加为穿戴者速度 |
| DRAIN_EXP、HEAVY_CURSE、RANDOM_CURSE2 | 显式戒灵/矮人戒指生成诅咒级别和诅咒效果，进入共用周期消费者 |
| XTRA_H_RES | 生成时抽取高阶抗性，保存结果 |
| AWARE | 原版未发现实际执行消费者；不编造自动鉴定效果 |
| HIDE_TYPE、SHOW_MODS、FULL_NAME | 名称/属性展示标记，沿用展示机制；不计作战斗能力 |
| SPECIAL | 仅 `(炸毁的)` 强制入口 |

随机激活补入音/碎片弹束与吐息、神之愤怒、星爆、星之球、逃脱、视线内短距移动、地震及清除陷阱/门。共用转换同时修正混乱/失明状态 ID、混乱之光的五种控制、防邪恶与幽灵形态时长；重新生成直接受影响的 E3/E5 定义。激活数值采用装置威力，避免遗漏通过能力执行器复用的固定伤害、治疗、状态时长和控制强度。

## 持久状态与验证边界

Protocol 1.234，State Hash Schema v111，save header/payload v6，容器仍为 v1；内容包 1.390.0。
新增实例装置 pval、箭袋容量和武器伤害骰加值，保存已掷结果；载入不重掷。只有 `(炸毁的)` 允许保存 0d0 武器骰子。反传送、抑制召唤和新增物品效果同步进入协议/内容 schema 与前端属性展示。

验证覆盖 17 条首饰在两档 power、80 个种子下的物化与存档往返，15 条可自然生成的首饰、九类光源、四类箭袋和七类装置的候选可达性，以及实际装备、激活、恢复、重量、补弹和炸毁行为。新增共用状态结构和自然生成 RNG 调度，按基线政策刷新并复验全部 26 条 active fixture。

2026-09-09 自动验证通过：核心 906 项、内容 133 项、导入器 174 项、协议 7 项、回放 8 项、存档 2 项；前端 181 项及类型检查通过。核心全量回归之后收紧了抑制召唤的适用范围，相关专项再次通过，并重新验证全部 26 条 active fixture。内容源校验、协议绑定与内容 schema 生成一致性检查、格式及 diff 检查均通过。

E8 继续核对各类型最外层 quality、神器、luck/RNG 和固定奖励的完整原版顺序，审计底材与重复 demo 定义，并执行集成与桌面验收。当前不覆盖未接入的 Monster Ring、Mauler、Vortex 等种族/职业专属修正；变身后的天生攻击与多手装备也留在 E8 的装备审计。既有合成 `demo.item.relic-blade` 没有原版底材元数据，不能据其验证正式神器炸毁后的底材身份；本批用正式神器验证身份清除和激活保留。未运行桌面 E2E、standalone 构建或人工试玩。
