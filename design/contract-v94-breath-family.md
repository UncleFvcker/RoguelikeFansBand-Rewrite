# Contract v94：怪物吐息族（breath）与 HP 比例伤害

状态：当前 active baseline。协议 1.94，内容包 1.85.0（content hash `2646a2fe3c9bd4f56f22bbc604a4e303bf15f28d9ba6445645b396ef03f27dae`）；save 容器继续 v1；吐息不新增权威状态字段，state hash 沿用 Schema v40。active baseline 共 306 个 exact fixtures、零 waiver。

## 1. 原版参考与本轮边界

BR\_ 吐息是导入报告中最大的未映射单族（46 种 token、790 实例）。原版语义（`monspell.c` `_breath_parm` / `_breath()`）：

- 伤害 = `min(施法者当前 HP × pct / 100, max)`，**不掷伤害骰**——"受伤的龙吐息变弱"是核心机制；元素默认档如酸/电/火/冷 20%/900、毒/辐射 17%/600、地狱 14%/550；
- r_info 可用 `BR_X(N%)` 覆盖百分比（0–100 夹取），上限沿用元素默认；
- 形状为锥（`rad = -2`，等级 ≥50 或字形 'D' 的龙为 `-3`）。

本轮以新效果 `breath-damage` 落地，**锥形几何全量复用 v79 机制**（八向 footprint、逐层展开、墙体遮挡、横向整数衰减）与 v87 的怪物 Cone 规划/执行路径；与 `cone-damage` 的唯一差异是基础伤害来源（HP 比例封顶、零 RNG）。首版仅限 monsterCasting（沿用 v91 位移族先例：玩家目标规划层不产出计划）。异种元素吐息（NETHER/DARK/CHAOS/NEXUS/SOUND/…约 460 实例）继续留缺口等伤害类型扩展。

## 2. 内容格式（1.85.0）

新效果 `breath-damage { hpPercent: u8, maxDamage: u16, damageType, radius: u8 }`：

- 校验：hpPercent 1–100、maxDamage 1–10 000、radius 1–9；
- 目标规则与 `cone-damage` 相同：仅 `direction` 模式 + 射程 1–64 + LOE；
- monsterCasting 白名单同 cone；不进入 Sequence 组合器；玩家能力书/技法不可用（规划层拒绝）。

demo 接入：新增 `demo.actor.ash-drake`（灰鳞幼龙，速度 100 显式，maxHp 12），承载单能力 `demo.ability.ash-breath`（breath-damage fire，hpPercent 60、maxDamage 6、radius 2，direction 射程 6 LOE）。满血 12×60% = 7 超过上限 → 封顶 6；受伤后按当前 HP 折算。延续教训：新形态只放新怪物。

## 3. 执行与 RNG 边界

- 施法时读取施法者当前 HP 计算基础伤害，**零伤害骰**；频率骰、加权选择骰、逆频率冷却与 v86–v93 完全一致；
- 锥形 footprint、方向推导（朝向目标的规范方向）、横向衰减 `rfb_area_damage(base, lateral)`、逐目标 `resolve_monster_damage_to_hostile`（护甲仅物理参与 + 抗性百分比）全部沿用既有 Cone 执行臂；
- 施法者 HP 在结算开始时读取一次，同轮内不随目标结算变化。

## 4. 协议与事件（1.94）

- `AbilityEffectSpecDto` 新增 `breath-damage` 变体（唯一协议变更；bump 1.94 并再生 bindings/schema）；
- 无新事件：沿用 `monster.ability-decision` / `monster.ability-cast` 与逐目标 Damage 结果；
- Web：protocol.ts 再生；无新文案 key。

## 5. 导入器（随后小步，含两件搭车项）

1. **吐息映射**：`BR_X` / `BR_X(N%)` → breath-damage 能力，direction 射程 8；半径按原版规则逐怪折算——等级 ≥50 或字形 'D' → 3，否则 2；能力按（元素-pct-max-半径）去重，id 形如 `rfb-legacy.ability.breath-fire-20-900-r2`。元素表：

| token | 默认 | 伤害类型 |
| --- | --- | --- |
| BR_ACID / BR_ELEC / BR_FIRE / BR_COLD | 20%/900 | 对应元素 |
| BR_POISON / BR_POIS | 17%/600 | poison |
| BR_NUKE | 17%/600 | poison（近似） |
| BR_PLASMA | 17%/250 | fire（近似） |

   预计收割约 330 实例；NETHER 51/DARK 50/CHAOS 36/NEXUS 35/SOUND 29 等异种元素留缺口。

2. **FREQ_N 频率修复**：`S:` 行频率的并列语法 `FREQ_N`（百分比直写，init1.c `freq = n`）此前未解析，297 只怪错用默认 10%；与 `1_IN_N` 同点解析，夹取 1–100。
3. **附身组重分类**：`MST_POSSESSOR` 专用 token（DETECT_TRAPS/EVIL/MONSTERS/OBJECTS、IDENTIFY、MAPPING、CLAIRVOYANCE、MULTIPLY、BLESS、HEROISM、BERSERK，共 522 实例）从 unmappedSpells 挪入新的 notApplicableSpells 桶，缺口报告只反映真实欠账。
4. **施法表上限 32→64**：吐息映射让旧版最大«大杂烩»施法者达到 34 技能；上限是纯校验常量（不进序列化/哈希），提至 64 与能力书上限对齐，导入器保留 64 截断守卫（当前零触发）。

## 6. 契约场景（v94）

迁移 303 条（零语义漂移，仅哈希更新——新增枚举变体不改既有内容序列化）后新增 304-306 共 3 条：

- 304 满血吐息封顶：12×60% = 7 → 封顶 6，事件伤害 = 6；
- 305 受伤吐息衰减：entityEffects 置幼龙 HP 5 → 5×60% = 3，实证 HP 比例；
- 306 冷却致死闭环：吐息-冷却-再吐息，`combat.player-death` 收尾于末条指令。

全部场景 saveRoundTrip。半径 3 折算、`(N%)` 覆盖解析、FREQ_N、附身组重分类由导入器单元测试覆盖；玩家侧规划拒绝由核心单元测试覆盖。

## 7. 验证

常规全套 + `migrate-baseline` 零漂移核对 + 新场景 `refresh` 人工审阅；clippy 单跑验退出码；五件套同步（协议 1.94 / pack 1.85.0 / content.lock / BUILT_IN+PREVIOUS / README）；本地桌面 E2E（contentVisualCount 73→74）。
