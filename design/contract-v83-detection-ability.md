# Contract v83：首个侦测能力

状态：历史 baseline。协议 1.83；内容包 1.75.0；state hash Schema v36

## 1. 目标

v83 在既有能力书、Mana、熟练度、失败率、冷却、秘密 terrain 知识和 FOV 管线之上加入首个内容驱动的侦测效果。首版只证明“玩家施法 → 在当前可见范围内筛选隐藏 terrain → 以瞬时或持久知识返回结果”的最小闭环，不把地图真值直接暴露给普通 UI，也不同时实现完整地图、怪物回忆、多资源或怪物施法。

## 2. 内容与协议

`AbilityEffectDefinition::Detect` 包含：

- `category`：必须匹配已注册 terrain tag；
- `radius`：1–8 的 Chebyshev 半径；
- `persistent`：是否把发现写入 `revealedTerrain`。

侦测能力必须只接受 `self` 目标、零射程且不要求 line of effect。demo 内容包新增：

- `demo.ability.echo-pulse`：`perception-cue`，半径 4，瞬时知识，3 Mana；
- `demo.ability.echo-sight`：`hidden`，半径 6，持久知识，4 Mana。

协议 1.83 增加 `AbilityDetectSpecDto`、`AbilityDetectResolutionDto`、`AbilityDto.detect` 和 `ability.detect` / `ability-detect` outcome。持久侦测的命中格通过 `changedCells` 返回真实 terrain；瞬时侦测只通过结构化 outcome 返回位置，不修改地图知识或普通 cell projection。

## 3. 权威筛选顺序

每次合法施法先完成能力/目标/资源前置验证，再按既有失败率 RNG 结算。成功后，核心按以下顺序筛选候选格：

1. 当前地图且在半径内；
2. 当前玩家 FOV/line of sight 可见；
3. 尚未进入 `revealedTerrain`；
4. 真实 terrain 声明了 `concealedAsTerrainId`；
5. terrain tags 包含能力的 `category`。

结果按距离、`y`、`x` 稳定排序。墙体或 FOV 外的隐藏 terrain 不会被侦测，也不会通过普通 UI 泄漏真实 terrain。无候选仍是成功施法：支付 Mana、抽取一次失败率 RNG、增加成功熟练度，并返回空的 `detectedPositions`。非法目标在资源、RNG、熟练度和冷却之前拒绝。

## 4. 持久与瞬时知识

`persistent = true` 时，命中位置加入 `revealedTerrain`，进入 save/state hash Schema v36，并在 `changedCells` 中以真实 terrain 返回；存档回读后继续保持同一知识。重复施法会过滤已经发现的格子，但仍遵循正常施法成本和 RNG 规则。

`persistent = false` 时，命中位置只存在于本次 `ability.detect` outcome；`revealedTerrain`、state hash 和地图记忆不改变。UI 可以显示一次性提示，但不能把该结果当作持久地形知识。

## 5. 内容包与兼容

当前内置 content hash：

```text
8ac0aee6fe54abb2c97bbed3eedaaa510d32393126bd08f89d046d515a66213b
```

旧 v82 hash 进入内置迁移白名单；旧存档缺少侦测能力、`revealedTerrain` 仍按既有默认值迁移，不自动学习新能力、不补发能力书、不推进 RNG。save 容器仍为 v1，state hash 从 v35 升至 v36，以纳入持久侦测知识边界而不纳入瞬时 outcome。

## 6. Exact fixtures 与回放

历史 baseline 位于 [`tests/fixtures/contract-v83/scenarios`](../tests/fixtures/contract-v83/scenarios)，共 221 个 exact fixtures、零 waiver。新增场景覆盖：

- 214：持久侦测的类别过滤、稳定顺序、墙体遮挡和 save round-trip；
- 215：瞬时侦测只返回结构化结果、不写入持久知识；
- 216：FOV/line of sight 过滤；
- 217：空结果仍消耗正常施法资源；
- 218：非法目标在资源/RNG/熟练度前拒绝；
- 219：Mana 不足的零 RNG 拒绝；
- 220：失败率失败支付 Mana 但不产生侦测结果；
- 221：持久发现的重复施法过滤。

核心单元测试覆盖持久/瞬时知识边界；`rfb-replay` 覆盖侦测事件的回放往返。历史 `contract-v1` 至 `contract-v82` 和 v82 policy 保留为回归基线。

## 7. 下一步

后续 [Contract v84](contract-v84-terrain-transform-ability.md) 已完成首个地形改变能力，复用 terrain 转换、`changedCells` 和楼层持久化，并固定可见性、来源集合、占用格、连接/边界保护、原子提交及空结果/RNG 边界。
