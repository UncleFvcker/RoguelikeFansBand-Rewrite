# Contract v99：小型效果杂项包（推离 / 吸取资源 / 失忆 / 驱散）

状态：当前 active baseline。协议 1.99，内容包 1.90.0（content hash `b425bafec4d4108b9eab4fd323b7b592f1e65ffb4197d45bcb1bc59567b61eff`）；save 容器 v1；explored/revealedTerrain 早已是权威存档字段，state hash 沿用 Schema v40。active baseline 共 318 个 exact fixtures、零 waiver。

## 1. 原版参考与本轮边界

法术缺口尾部的四件小型形态一次打包（DISPEL 走零新机制映射）：

- **TELE_OTHER 69**（`ESCAPE_TELE_OTHER`）：把玩家传送到远处 → 新效果 `teleport-away { minimumDistance }`，与 v91 teleport-self 同族反向：候选 = 与**施法者**切比雪夫距离 ≥ 下限的开阔格，一次有界抽取，空候选减半重筛再空则拒；玩家走 relocate_player 管线（到达陷阱/感知照常）。事件 `monster.banished-target` 复用 v91 `MonsterDisplacement` outcome（零新 DTO）。
- **DRAIN_MANA 83**（GF_DRAIN_MANA，默认量 1+等级/2，施法者以吸取量回血）：新效果 `drain-resource { amount }`：吸取玩家施法档资源池（无档则按 id 序首个非空池；全无则吸取 0），施法者以实际吸取量回血（封顶最大生命）。新 resolution 变体 `drain-resource { resourceId, requested, drained, casterHealed }`。
- **AMNESIA 64**（豁免门 + `lose_all_info`）：新效果 `amnesia`：先掷 v72 豁免（难度 = 施法者等级，与 v98 诅咒同门）；成功 → `saved` 跳过零后续 RNG；失败 → 清空当前层探索记忆与 revealedTerrain，新 resolution 变体 `amnesia { clearedCells }`。中性化：只忘当前层地图记忆；物品知识按长期约束（存档级权威）不动；原版忘全部楼层记为差异。
- **DISPEL_MAGIC 48**（驱散玩家增益）：映射为既有 `remove-status`（rfb.status.haste）——玩家增益现役仅加速，零新机制；未来增益扩族后再扩驱散列表。

留缺口：DARKNESS 85（原版 `unlite_room`——房间光照状态未建立）、ANIM_DEAD 58（无尸体系统）、ANTI_MAGIC 47（反魔法场）、TELE_LEVEL 25（跨层强制传送）、HAND_DOOM 30。

三个新效果首版仅限 monsterCasting（玩家规划 None）。非玩家目标简化：teleport-away 可推离召唤物（实体重定位）；drain-resource/amnesia 对召唤物以 `no-target` 跳过（无资源池/无知识概念）。

## 2. 内容格式（1.90.0）

- `teleport-away { minimumDistance: 1–64 }`、`drain-resource { amount: 1–1 000 000 }`、`amnesia {}`：目标规则均与 `damage` 相同（entity/position + 射程 + LOE）；不进 Sequence。
- demo 接入：新增 `demo.actor.veil-warden`（帷障守卫，等级 4，频率 50，四能力等权）：`veil-banish`（teleport-away min 8）、`veil-drain`（drain-resource 5）、`veil-amnesia`、`veil-dispel`（remove-status haste）。

## 3. 执行与 RNG 边界

- teleport-away：候选规范序收集零 RNG，一次有界抽取落点；减半回退与 v91 teleport-self 一致；
- drain-resource：零 RNG（固定量，取 min(amount, 池当前)）；施法者回血不溢出；
- amnesia：豁免 1–2 抽（v72 检定）；失败后清记忆零 RNG；`SavingThrowChecked` 事件复用；
- 频率骰、加权选择骰、冷却零改动。

## 4. 协议与事件（1.99）

- `AbilityEffectSpecDto` 新增三变体；`AbilityEffectResolutionDto` 新增 `drain-resource` 与 `amnesia` 变体；
- 新事件种类 `monster.banished-target`（复用 `MonsterDisplacement` outcome 与 v91 文案族）；豁免复用 `SavingThrowChecked`。

## 5. 导入器映射（随后小步）

TELE_OTHER → `rfb-legacy.ability.banish`（min 10，与 escape 对称）；DRAIN_MANA → `drain-mana-{amount}`（amount = 1+等级/2 去重）；AMNESIA → `rfb-legacy.ability.amnesia`；DISPEL_MAGIC → `rfb-legacy.ability.dispel`（remove-status haste）。实测收割 **264 实例全数**（casting 怪 844、映射累计 4601、未映射 627）。

## 6. 契约场景（v99）

迁移 314 条（零语义漂移）后新增 315-318 共 4 条（种子狩猎按 1/4 权重选中各能力）：

- 315 推离：玩家被传送至距守卫 ≥8 的落点（`monster.banished-target` + relocate 管线）；
- 316 吸取：playerResources 预置法力，池 -5、施法者 +5 血（resolution 全录）；
- 317 失忆（豁免失败种子）：explored/revealedTerrain 清空实录（changed cells 覆盖原已探索格）；
- 318 驱散：playerStatuses 预置加速，remove-status 移除实录。

豁免成功路径（`saved` 跳过）与 v98 同构，由 313 先例与核心单测覆盖。

## 7. 验证

常规全套 + `migrate-baseline` 零漂移核对 + 新场景 `refresh` 人工审阅；clippy 单跑验退出码；五件套（协议 1.99 / pack 1.90.0 / content.lock / BUILT_IN+PREVIOUS / README）；本地桌面 E2E（contentVisualCount 78→79）。
