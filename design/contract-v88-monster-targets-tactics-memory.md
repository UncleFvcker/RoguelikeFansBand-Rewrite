# Contract v88：怪物目标、战术移动与施法记忆

状态：当前 active baseline。协议版本为 1.88，demo 内容包版本为 1.80.0，content hash 为 `29116f924e1ef4ddf6b0aa43f3b1b1bd0b4d28245ac086bce30d7a008e8e9e8e`。save 容器继续使用 v1；怪物已观察抗性是新增权威状态，state hash 升至 Schema v38。

## 1. 原版参考

FrogComposband 把目标、撤退和知识分成不同层：

- `melee2.c` 的敌对目标与 `mon_will_run()`/`find_safety()` 决定追击或远离；
- pack 的 `AI_MAINTAIN_DISTANCE` 在距离过近时采取回避移动；
- `monster2.c::update_smart_learn()` 只在效果实际结算后记录“已观察”抗性，愚笨怪物不学习，smart 怪物稳定学习，普通怪物还可能额外抽学习骰。

本纵切保留这些边界，但不复制原版依赖全局选项和随机普通怪物学习的实现：只有内容显式声明 `smart` 的施法者学习，观察本身不抽 RNG。

## 2. 敌对目标与稳定选择

敌对怪物的合法目标集合现在包括玩家本体和所有 `ownerId` 指向玩家的存活召唤物。每个攻击能力独立按以下顺序尝试目标：

1. Chebyshev 距离较近者优先；
2. 同距离时玩家本体优先；
3. 其后按稳定实体 ID 排序。

目标仍需通过该能力的射程、墙体、line-of-effect 与 clean-shot 检查；较近目标被阻挡时可以继续尝试后续合法目标。普通追踪与近战复用同一主目标，因此玩家召唤物可以拦在玩家之前成为法术或近战目标。友方召唤物自身的行动 AI 仍未在本纵切开放。

## 3. 多目标敌我评分

范围爆发、延长射线和锥形先生成既有确定性 footprint，再分别统计：

- `enemyTargetCount`：玩家及玩家拥有的召唤物；
- `friendlyRiskCount`：施法者以外的普通敌对怪物或怪物拥有的召唤物。

存在友方风险时仍以 `friendly-risk` 拒绝，避免未由能力声明的友军误伤；没有友方风险时，有效权重乘以敌方命中数。`monster.ability-cast.targets` 按主目标优先、其后稳定实体 ID 返回每个实际目标和逐效果结果。

多目标伤害共用一次基础伤害骰。范围与锥形分别按每个目标到中心或中心线的距离执行既有整数衰减；射线对全部命中目标使用同一基础伤害。被击败的玩家召唤物在本次结算内立即从权威实体集合移除，不授予玩家经验或掉落。

## 4. 保持距离与受伤撤退

`MonsterCastingDefinition` 新增：

- `preferredDistance`：合法范围 2–16；
- `fleeHpPercent`：0 表示关闭，1–99 表示生命百分比阈值；
- `smart`：是否拥有确定性观察学习能力。

Echo Cantor 声明 `preferredDistance = 3`、`fleeHpPercent = 25`。施法失败、无可用法术或处于施法冷却时，它先尝试走到能提高“距所有玩家阵营目标的最小距离”的相邻合法格；候选按最大安全距离和固定八方向顺序确定，不抽 RNG。没有更安全位置时继续原有近战、追踪或 pack 行为，不会凭空跳过普通行动回退。

## 5. 有限抗性记忆

聪明施法者只能在自身伤害或带抗性类型的状态效果实际作用于玩家后，记录该伤害类型当时的 `vulnerable/normal/resistant/strong/immune`。规则明确禁止：

- 在首次选择前读取玩家完整抗性表；
- 从命中玩家召唤物推断玩家抗性；
- 因候选评估、被墙阻挡、频率失败或冷却而更新知识；
- 为观察额外抽取 RNG。

后续针对玩家的对应候选按 150%/100%/50%/35%/0% 调整权重；已知 immune 以 `no-utility` 剔除。记忆按伤害类型稳定排序，进入 `ActorSaveDto.observedPlayerResistances`、`EntityDto`、离层存档、回放检查点和 state hash Schema v38。旧存档缺失字段时恢复为空记忆，不补观察、不推进 RNG。

## 6. 协议、内容与验证

协议 1.88 增加：

- `MonsterAbilityCandidateResolutionDto.enemyTargetCount/friendlyRiskCount`；
- `MonsterAbilityTargetResolutionDto`；
- `MonsterAbilityCastResolutionDto.targets`；
- `EntityDto/ActorSaveDto.observedPlayerResistances`。

demo 内容包升级为 1.80.0；Echo Cantor 启用 smart、保持距离和受伤撤退。contract-v88 从 v87 迁移全部历史场景并新增 8 个 exact fixtures，覆盖召唤物主目标、双敌目标评分与结算、友军风险计数、保持距离、受伤撤退、观察学习、抗性降权、免疫剔除和存档回环。active baseline 共 265 个 exact fixtures、零 waiver。

## 7. 后续

P29 优先候选为玩家友方召唤物行动与首版命令：复用已经稳定的阵营目标集合，让召唤物跟随、攻击、保持距离或守卫，并明确主人、跨楼层与生命周期边界。多资源职业、装备激活、怪物反制/沉默和完整原版法术表继续后置。
