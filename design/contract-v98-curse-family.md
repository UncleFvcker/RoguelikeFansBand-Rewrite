# Contract v98：诅咒族（curse-damage）与首个法术豁免门

状态：历史 baseline；当前 active baseline 见 [contract-v99](contract-v99-misc-effects.md)。协议 1.98，内容包 1.89.0（content hash `086d65709052cee99f2ddd3e44ed5b8776c3a3d52f9d96799bbddec9282cda34`）；save 容器 v1；state hash 沿用 Schema v40。该 baseline 共 314 个 exact fixtures、零 waiver。

## 1. 原版参考与本轮边界

CAUSE 系共 240 实例（CAUSE_1 14 / CAUSE_2 31 / CAUSE_3 97 / CAUSE_4 98，固定骰 3d8 / 8d8 / 10d15 / 15d15）。原版语义（gf.c）：`_plr_save`（odds = 玩家豁免技能×100 / (100 + 施法者等级/2 + dam/5)）**成功则全免**；失败全额伤害 + 装备诅咒几率。伤害不走护甲也不走元素抗性——豁免是唯一防线。

本轮建立**首个法术豁免门**：新效果 `curse-damage`，复用 v72 的 saving-throw 检定机件（`resolve_check` + 既有 `SavingThrowChecked` 事件——与陷阱豁免同一事件族，零新检定机制）。中性化差异（记录不复刻）：豁免公式用 v72 中性检定（技能 vs 难度=施法者等级）而非原版连续概率；dam/5 难度加成、装备诅咒副作用（无装备诅咒系统）、HAND_DOOM（当前 HP 百分比 + 豁免，30 实例）留缺口。心灵族的豁免门回补另行排期。

新增伤害类型 **`curse`**（原版 GF_CAUSE_1..4 折算；核心与协议枚举，内容层 ActorDamageType 不开放——诅咒类型只经由 curse-damage 效果隐式产生）。无人可声明诅咒抗性（含 RES_ALL 展开表刻意排除）：非物理不吃护甲、恒 Normal 不吃抗性，豁免即唯一防线，与原版一致。

## 2. 内容格式（1.89.0）

新效果 `curse-damage { damageDice, damageSides, damageBonus }`（伤害类型隐式 curse）：

- 校验：dice 1–100、sides 1–10 000、bonus ≤10 000；
- 目标规则与 `damage` 相同（entity/position + 射程 + LOE）；
- 仅限 monsterCasting（玩家规划层拒绝，v91/v94/v95 同款）；不进 Sequence。

demo 接入：新增 `demo.actor.hex-chanter`（咒殃咏者，glyph "p"，等级 4）承载 `demo.ability.woe-curse`（curse-damage 2d4+1，射程 6 LOE）。

## 3. 执行与 RNG 边界

怪物→玩家：先掷豁免（v72 检定：1 次百分位 + 视分支 1 次对抗掷，能力=玩家豁免技能派生值、难度=施法者定义等级经 ActionDifficulty 管线）→ 推送既有 `SavingThrowChecked` 事件（skill.saving-throw-success/-failure）：

- 豁免成功：效果以新 skip 原因 `saved` 跳过，**不掷伤害骰**（零后续 RNG）；
- 豁免失败：掷 `XdY+bonus`，经既有管线结算——curse 非物理不吃护甲、无抗性来源恒 Normal，全额落账。

怪物→玩家召唤物：无豁免技能概念，v1 直接全额（记录为已知简化）。频率骰、加权选择骰、冷却零改动。

## 4. 协议与事件（1.98）

- `DamageTypeDto` 新增 `curse`；`AbilityEffectSpecDto` 新增 `curse-damage`；`AbilityEffectSkipReasonDto` 新增 `saved`；
- 豁免过程复用既有 `SavingThrowChecked` 事件与 `CheckResolutionDto`（v72 形状），零新事件。

## 5. 导入器映射（随后小步）

CAUSE_1→3d8、CAUSE_2→8d8、CAUSE_3→10d15、CAUSE_4→15d15（显式 `(XdY+Z)`/`(N)` 覆盖沿用既有解析），id 形如 `rfb-legacy.ability.curse-10d15`，目标 entity/position 射程 8 LOE。HAND_DOOM 留缺口。实测收割 **240 实例全数**（casting 怪 832、映射累计 4337、未映射 891——首次跌破千）。

## 6. 契约场景（v98）

迁移 312 条（零语义漂移）后新增 313-314 同设置双种子孪生：

- 313 豁免成功：`skill.saving-throw-success` 事件 + 效果 `saved` 跳过 + RNG 计数实证未掷伤害骰；
- 314 豁免失败（种子狩猎自动失败/低对抗掷）：`skill.saving-throw-failure` + 全额伤害（raw = final，护甲/抗性零参与）。

豁免难度取施法者等级、召唤物目标无豁免由核心单元测试补充。

## 7. 验证

常规全套 + `migrate-baseline` 零漂移核对 + 新场景 `refresh` 人工审阅；clippy 单跑验退出码；五件套（协议 1.98 / pack 1.89.0 / content.lock / BUILT_IN+PREVIOUS / README）；本地桌面 E2E（contentVisualCount 77→78）。
