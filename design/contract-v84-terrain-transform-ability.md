# Contract v84：首个地形改变能力

状态：当前 active baseline。协议版本为 1.84，demo 内容包版本为 1.76.0，content hash 为 `6e3906fff5447c3b83630e85e6c789a0dc151d9e16e1faa484ed10dda41a3ee4`。save 容器继续使用 v1；地形数组原本已经进入存档和 state hash，因此 state hash 继续使用 Schema v36。

## 1. 原版参照与纵切范围

本纵切参考原版系 `wall_to_mud()`、`GF_KILL_WALL` 和 `GF_MAKE_WALL` 的地形投影思路：效果只修改明确允许的 terrain，不从 glyph 或显示文本推断规则。新核心不复制旧版 feature flag 表，而是使用内容驱动的稳定 terrain ID 集。

demo 内容包加入两个互补能力：

- `demo.ability.echo-delving`：把墙、瓦砾、共鸣废墟和矿脉转换为普通地面；
- `demo.ability.echo-rampart`：把普通地面、天然洞穴、浅水和谐振地板转换为阻挡通行的回声瓦砾。

两者都只处理当前楼层，不自动修复连通性，也不移动或销毁 actor、物品、楼梯和入口。

## 2. 内容定义

`AbilityEffectDefinition::TransformTerrain` / JSON `transform-terrain` 声明：

- `sourceTerrainIds`：1–32 个互不重复的合法 terrain ID；规范化时稳定排序；
- `targetTerrainId`：合法且不在来源集合中的 terrain ID；
- `radius`：0–8。

地形改变能力只允许 `position` 目标，内容射程为 1–64，且必须声明 `requiresLineOfEffect: true`。来源或目标引用缺失、重复来源、来源等于目标、越界半径或其他目标模式都会在内容加载时拒绝。

## 3. 目标与候选格

目标中心必须：

- 位于当前地图；
- 与玩家的 Chebyshev 距离不超过内容射程；
- 当前可见；
- 满足玩家到中心的 line of effect。

无效目标在 Mana、施法失败率 RNG、熟练度和 cooldown 之前拒绝。

有效中心按 RFB 距离收集半径内候选。每个候选必须：

- 当前可见，并与中心存在 line of effect；
- 当前 terrain ID 位于 `sourceTerrainIds`；
- 不在地图最外圈；
- 不由玩家、存活 actor 或地面物品占用；
- 不位于活动 floor connection；
- 不带 `stairs-down`、`stairs-up`、`shaft`、`dungeon-entry` 或 `task-entry` 连接标签。

候选按距中心距离、`y`、`x` 稳定排序。普通门、墙和瓦砾是否可改只由内容来源集合决定；连接 terrain 和地图边界始终受到核心保护。

## 4. 原子提交与 RNG

候选集合在支付资源和抽施法失败率之前完整计算，但空集合仍是合法目标。

- 资源不足：不抽 RNG、不改熟练度、不改地形；
- 施法失败：支付 Mana、记录失败并推进一次既有施法 RNG，但不改任何 terrain；
- 施法成功：不增加额外 RNG，一次提交预先收集的全部候选；
- 空结果成功：仍支付 Mana、记录成功、推进一次施法 RNG，并返回空的结构化结果。

同一次施法不会让较早的地形写入改变较晚候选的资格。部分格因占用、连接或边界被过滤时，其余合法候选仍作为一个确定性集合提交。

## 5. 协议、渲染与知识

协议新增：

- `AbilityTerrainTransformSpecDto` 与 `AbilityDto.terrainTransform`；
- `AbilityTerrainTransformResolutionDto`；
- `GameEventOutcomeDto::AbilityTerrainTransform`；
- `ability.terrain-transform` / `ability-terrain-transform`。

resolution 返回中心、半径、规范化来源集合、目标 terrain ID 和稳定排序的 `transformedPositions`。所有实际修改格进入既有 `changedCells`，Web 因而只重建受影响 terrain chunk。

修改格会从 `revealedTerrain` 移除，防止旧隐藏投影知识继续附着在已经替换的 terrain 上。能力不会把视野外真值写入事件或增量。

## 6. 存档、回放与基线

当前楼层 terrain 数组和离层 `FloorSaveDto` 原本已经完整保存并进入 state hash；本纵切没有新增持久字段，所以 save v1 与 state hash Schema v36 保持不变。旧 v83 及更早 built-in content hash 继续接受并迁移到当前内容，旧存档不会自动产生地形改变。

活动 baseline 位于 [`tests/fixtures/contract-v84/scenarios`](../tests/fixtures/contract-v84/scenarios)，共 231 个 exact fixtures、零 waiver。新增 222–231 覆盖：

- 掘进多格稳定顺序、`changedCells` 和 save round-trip；
- 壁垒对玩家、地面物品和地图边界的过滤；
- 空结果与 FOV 后方 terrain 过滤；
- 非 position、超距和资源不足的零 RNG 前置拒绝；
- 失败施法支付资源但不修改 terrain；
- 重复施法第一次修改、第二次空结果的确定性。

核心单元测试另外覆盖存活 actor、静态楼梯/连接标签、revealed terrain 清理和地图边界；`rfb-replay` 覆盖地形修改后的终态哈希回放。

## 7. 下一步

P25 候选为首个状态能力与多 effect 组合：让能力复用现有状态添加/移除原语，明确逐效果顺序、部分无效、资源/RNG 原子边界和存档/回放。多职业资源和怪物施法继续等待公共效果组合边界稳定。
