# Contract v93：怪物直伤弹族（bolt / ball）与伤害平坦加值

状态：当前 active baseline。协议 1.93，内容包 1.84.0（content hash `134479da14e58dfd8c52d6587a33ad61ac97f7c430632ffca6ccd378b9ba7f30`）；save 容器继续 v1；伤害加值不新增权威状态字段，state hash 沿用 Schema v40。active baseline 共 303 个 exact fixtures、零 waiver。

## 1. 原版参考与本轮边界

FrogComposband 怪物法术中直伤三族（吐息 BR\_ 790 / 球 BA\_ 422 / 弹 BO\_ 370 实例）是导入报告里最大的未映射缺口。本轮覆盖**骰伤害制的弹（bolt）与球（ball）**两族；吐息（BR\_，伤害按当前 HP 比例折算）机制不同，留给后续迭代单独设计。

原候选 DETECT 侦测族经源码核实取消：`DETECT_MONSTERS` / `DETECT_OBJECTS` 等全部位于 `MST_POSSESSOR` 组，源码注释明确「这些法术不会被怪物施放，仅供拟态/附身玩家访问不影响怪物的玩法侧面」。r_info 里的 149/99 处出现均为附身素材，怪物侧无行为可映射；该结论记入缺口报告口径（DETECT 族按不适用处理，不算映射欠账）。

核心早在 v86 弹族（Echo Cantor 的 resonant-bolt / echo-burst / echo-lance / echo-fan）就已支持怪物施放 `damage` / `area-damage` / `beam-damage` / `cone-damage`，本轮不需要新效果种类。唯一机制缺口：原版默认伤害公式普遍带**平坦加值**（火弹 9d8+L/3、酸球 1d(3L)+15、投石纯平坦 3L），而现行效果只有 XdY 两个骰参数。

## 2. 内容格式（1.84.0）

四种伤害效果（`damage` / `area-damage` / `beam-damage` / `cone-damage`）各新增可选字段 `damageBonus: u16`（serde 默认 0，上限 10 000，与 damage_sides 同界）。既有内容不写该字段、编译后语义不变（伤害掷骰恒加 0）。

demo 接入：新增 `demo.actor.cinder-adept`（烬火修士，速度 100 显式写出），承载三个带非零加值的能力：

- `demo.ability.cinder-bolt`：`damage` 2d3+3 fire，entity/position 射程 6 LOE（权重 2）；
- `demo.ability.cinder-burst`：`area-damage` 1d4+2 fire 半径 2，entity/position 射程 6 LOE（权重 1）；
- `demo.ability.cinder-fan`：`cone-damage` 1d3+2 fire 半径 2，direction 射程 6 LOE（权重 1）。

沿用 v91/v92 教训：新形态只放新怪物，既有怪物加权池零触碰，历史场景选择骰映射不移动。`beam-damage` 的加值算术与 bolt 完全同路（roll 站点共用模式），由核心单元测试覆盖，不再增设 demo 能力。

## 3. 执行与 RNG 边界

- 伤害掷骰改为 `roll(XdY) + damageBonus`，随后照常进 `resolve_armored_damage`（护甲减免 + 抗性百分比）；加值参与减免前的原始伤害，不引入新 RNG。
- 平坦伤害（原版 0d0+F 形态）编码为 `1d1 + (F-1)`：1d1 恒掷 1，总和恒为 F，不新增零骰特例、不改动内容校验的 dice ≥ 1 约束。
- 频率骰、加权选择骰、逆频率冷却、目标规划（projectile / direction）与 v86–v92 完全一致，本轮零改动。

## 4. 协议与事件（1.93）

- `AbilityEffectSpecDto` 四个伤害变体同步新增 `damageBonus: u16` 字段（唯一协议变更；序列化必写，故 bump 1.93 并再生 bindings/schema）。
- 无新事件、无新 DTO：命中/击杀沿用既有 `AbilityHit` / `AbilitySlew` 家族，事件里只出现折算后的伤害结果。
- Web：protocol.ts 再生；无新文案 key、无新面板。

## 5. 导入器映射（随后小步）

S: 行 token 解析扩展：`TOKEN`、`TOKEN(XdY)`、`TOKEN(XdY+Z)`、`TOKEN(N)`（纯平坦，编码为 1d1+(N-1)）。无显式骰时按源码默认公式以 r_info 等级 L 折算；生成能力按（形态-元素-骰面）去重共享，id 形如 `rfb-legacy.ability.bolt-fire-9d8-13`。

弹族（`damage`，entity/position 射程 8 LOE）：

| token | 默认骰 | 伤害类型 |
| --- | --- | --- |
| BO_ACID | 7d8+L/3 | acid |
| BO_ELEC | 4d8+L/3 | electricity |
| BO_FIRE | 9d8+L/3 | fire |
| BO_COLD | 6d8+L/3 | cold |
| BO_ICE | 6d8+L | cold（近似） |
| BO_PLASMA | 8d7+(10+L) | fire（近似） |
| BO_WATER | 10d10+L | physical（近似） |
| MISSILE | 2d6+L/3 | physical |
| SHOOT | min(6,4+L/24)d max(2,L/4) | physical |
| THROW | 1d1+(3L-1) | physical（原版 BALL0 单体，映射为单体弹） |

球族（`area-damage` 半径 2，entity/position 射程 8 LOE）：

| token | 默认骰 | 伤害类型 |
| --- | --- | --- |
| BA_ACID | 1d(3L)+15 | acid |
| BA_ELEC | 1d(3L/2)+8 | electricity |
| BA_FIRE | 1d(7L/2)+10 | fire |
| BA_COLD | 1d(3L/2)+10 | cold |
| BA_POISON | 12d2 | poison |
| BA_NUKE | 10d6+L | poison（近似） |
| BA_WATER | 1dL+50 | physical（近似） |
| ROCKET | 1d1+(6L-1) | physical |
| PULVERISE | 8d8 | physical（近似） |

异种元素（BO_MANA 88 / BO_NETHER / BO_TIME / BA_CHAOS / BA_DARK / BA_LITE / MANA_STORM / BRAIN_SMASH / MIND_BLAST / DRAIN_MANA / PSY_SPEAR / HELL_LANCE / HOLY_LANCE / HAND_DOOM / GAZE / CHICKEN）继续留在 unmappedSpells：物理/五元素之外的伤害类型语义（不可抗魔法、灵魂、混沌等）值得未来「伤害类型扩展」迭代原生支持，现在硬折 physical 会让护甲错误参与减免。导入产物只进 `.local/`，未来类型扩展后重跑导入即可无痛升级。

## 6. 契约场景（v93）

迁移 299 条（零语义漂移核对通过：唯一差异为快照能力规格新增 `damageBonus: 0` 字段——协议 1.93 的既定格式扩展）后新增 300-303 共 4 条：

- 300 bolt 命中玩家：raw 9 = 2d3 满骰 6 + 加值 3，**超出无加值上限**，加值在契约内自证；火焰非物理，不吃护甲减免；
- 301 burst 波及玩家（守卫经 entityEffects 移出爆炸半径——入口守卫在默认位 (2,1) 距玩家切比雪夫 2，会触发 friendly-risk 拒绝）；
- 302 fan 方向锥波及（12 个锥格实录）；
- 303 冷却节奏致死闭环：第 2 回合首发弹（raw 9 → HP 1）、逆频率冷却 + 频率骰空转数回合、第 8 回合次发弹致死，`combat.player-death`（method = cinder-bolt）恰在末条指令收尾。

全部场景 saveRoundTrip。怪物侧加值算术与零额外 RNG 由核心单元测试（`damage_bonus_adds_flat_amount_to_monster_cast_damage`）覆盖，1d1 平坦编码恒等式与显式骰解析由导入器单元测试覆盖。

## 7. 验证

常规全套 + `migrate-baseline` 零语义漂移核对 + 新场景 `refresh` 后人工审阅；clippy 单独跑并验退出码；内容五件套（PROTOCOL_VERSION / pack.json / content.lock.json / BUILT_IN_CONTENT_HASH+PREVIOUS / README 版本段）同步；本地桌面 E2E（contentVisualCount 随新怪物 +1）。
