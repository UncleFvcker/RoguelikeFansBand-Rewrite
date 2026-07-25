# Contract v73：法术资源与能力书基础

状态：协议 1.73 / contract-v73 历史基准；内容包 1.65.0；state hash Schema v32；active baseline 已由 contract-v74 接替

## 范围

v73 在 v71 的角色构筑和 v72 的可观察检定之上，建立第一条完整的玩家能力书施法循环：内容定义稳定资源、能力与能力书，职业提供施法配置，玩家持有实体书本后学习能力，并以可保存资源和确定性失败率进行施放。

本纵切只证明公共边界，不录入旧 RFB 的领域、法术文本或完整职业矩阵。首个示例使用原创 Scholar/Mage 构筑、Mana、Echo Primer 与 Resonant Bolt。

## 内容模型

内容包新增三个严格类型根：

- `resources`：声明稳定资源 ID、名称、描述和标签；
- `abilities`：声明等级、资源成本、基础失败率、目标规格和首个伤害效果；
- `abilityBooks`：以稳定能力 ID 列表声明一本书包含的能力。

`ItemDefinition.abilityBookId` 把实体物品连接到能力书。书本物品必须不可堆叠、不可装备且不能同时声明普通使用效果。`ClassDefinition.castingProfile` 声明：

- 使用的资源 ID；
- Intelligence、Wisdom 或 Charisma 施法属性；
- 基础容量、每级容量和每属性桶容量；
- 职业最低失败率；
- 该职业支持的能力书 ID。

编译器严格验证所有引用、能力书去重、目标范围、效果骰、资源一致性和书本物品形状。若一本职业能力书包含使用其他资源的能力，内容编译直接失败。

demo 内容包升至 1.65.0，并新增：

- `demo.resource.mana`；
- `demo.ability.resonant-bolt`：等级 1、消耗 3 Mana、基础失败率 35%、射程 6、`2d4` electricity；
- `demo.ability-book.echo-primer`；
- `demo.item.echo-primer`，作为 Scholar 的出生物品；
- Mage 的 Intelligence 施法配置。

## 权威资源与失败率

首版资源上限为：

```text
maximum = baseCapacity
        + playerLevel × capacityPerLevel
        + castingAttributeIndex × capacityPerAttributeIndex
```

Scholar 等级 1、Intelligence index 15，因此初始 Mana 为 `4 + 1×2 + 15×1 = 21`。等级或有效属性变化时重新计算上限；当前值不会因为上限提高而自动补满，也不会高于新上限。

首版失败率为：

```text
failure = baseFailurePercent
        - 3 × (playerLevel - minimumLevel)
        - castingAttributeIndex
```

结果限制在职业 `minimumFailurePercent..=95`。Resonant Bolt 在初始 Scholar 上为 `35 - 0 - 15 = 20%`。

施法先检查职业、能力、等级、已学习状态、支持的书本、实体书本和资源。资源不足等前置拒绝不会抽 RNG。通过前置检查后先扣资源，再掷 `0..99`：掷骰小于失败率则失败，失败仍消耗资源。只有检定成功后才解析目标和路径。

## 学习、施法与事件

协议 1.73 新增：

- `StudyAbility { bookItemId, abilityId }`；
- `CastAbility { abilityId, target }`；
- `ResourcePoolDto`、`AbilityDto` 和 `AbilityCastResolutionDto`；
- `GameEventOutcomeDto.ability-cast`。

学习要求玩家背包中持有指定实体书本、书本属于职业支持集合且确实包含该能力。成功学习写入稳定能力 ID；重复学习、等级不足、书本不匹配和非施法职业均产生结构化拒绝事件。学习消耗一个普通行动，但不消耗书本和 RNG。

施法复用既有 `TargetSpec`、`TargetSelection`、整数投射路径、伤害、抗性、击杀、经验、任务和掉落管线。事件明确区分：

- 学习成功与学习拒绝；
- 施法前置拒绝；
- 检定成功与检定失败；
- 成功检定后的无效目标、落空、命中和击杀。

Web 能力面板显示资源、等级、成本、失败率、学习状态，并提供研习和施放按钮；施放继续使用既有键盘目标模式。中英文 Fluent 资源覆盖能力、书本、资源、事件与拒绝原因。

## 存档、hash 与兼容

`PlayerSaveDto` 新增可选资源池列表和已学能力 ID 列表；`PlayerDto` 投影资源与能力可用性。存档只保存当前/最大资源和稳定能力 ID，不保存失败率、目标规格或 `canStudy/canCast` 等派生字段。

v72 及更早存档缺失这些字段时：

- 施法职业按当前构筑与属性建立满资源；
- 已学能力为空；
- 非施法职业保持空资源与空能力状态；
- 迁移不抽 RNG、不补做学习，也不改变已有物品。

资源、已学能力、施法后的生命/经验/物品结果和完整 RNG 位置进入 state hash Schema v32。正式 save 容器仍为 v1；v72 内容 hash `3188f4cf0937f44292980e8ca8fffc1db9c310e961af4502bd9380124e53d54a` 保留在迁移白名单中。

## 确定性覆盖

历史基准位于 [`tests/fixtures/contract-v73/scenarios`](../tests/fixtures/contract-v73/scenarios)，共有 166 个 exact fixtures、零 waiver。v72 的 160 个场景全部迁移，并新增：

| Fixture | seed | 固定行为 |
| --- | ---: | --- |
| `ability.study.success` | 0 | 持书学习成功，不抽 RNG，已学状态可回读 |
| `ability.cast.unlearned` | 0 | 未学习拒绝，Mana 与 RNG 不变 |
| `ability.cast.success` | 0 | roll 32，消耗 3 Mana，造成 5 点伤害并击杀，升级后上限 23 |
| `ability.cast.failure-costs-resource` | 2 | roll 13，检定失败但 Mana 由 21 降到 18 |
| `ability.cast.insufficient-resource` | 0 | 已学习但只有 2 Mana，拒绝且 RNG 不推进 |
| `ability.legacy-save-migration` | 0 | 缺字段存档恢复 21/21 Mana、空已学列表并完成 round-trip |

内容与核心专项单元测试同时锁定悬空引用、非法书本物品、职业资源一致性、成功/失败施法、资源不足和旧存档迁移。

## 明确不在 v73 的范围

- Mana 自然恢复、休息命令、恢复中断与多种职业资源；
- 学习容量、随机学习、遗忘、首次施放奖励、熟练度和冷却；
- 自身、方向、范围、锥形、治疗、位移、召唤、侦测和地形改变能力；
- 装备负重、状态和环境对失败率的完整修正；
- 怪物施法、能力评分、智能学习与完整领域/职业矩阵。

上述资源恢复、休息边界、稳定自身目标与第二类治疗效果已由 [Contract v74](contract-v74-resource-recovery-and-healing.md) 完成。后续优先推进能力熟练度与冷却；多资源和怪物施法继续等待玩家能力状态边界稳定。
