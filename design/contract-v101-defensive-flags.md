# Contract v101：装备/内在旗标系统·防御面（抗性/免疫/速度）

状态：已实现。协议 1.101 / 内容包 1.92.0（hash `83dc1e5a…`）/ state hash Schema v41（未变更）/ 323 个 exact fixtures、零 waiver。

## 1. 动机

v96 建立了 actor 声明式抗性档，但玩家侧的防御性旗标（旧版 `RES_*`/`IM_*`/`VULN_*`/`FREE_ACT`/`SPEED`）仍无处安放：词条 72 条 ego 因不可表达全数跳过、神器旗标主体躺在缺口报告、种族 calc_bonuses 的内在抗性无内容表面。本轮（T2 前半，防御面）为装备/词条/种族三处补齐同一套声明表面，进攻面（斩杀/品牌）留待下一轮。

## 2. 内容层

- `ItemDefinition` / `AffixDefinition` / `RaceDefinition` 新增 `resistances`（伤害类型 → 档位，复用 v96 的 actor 抗性词表与档位枚举）与 `statusImmunities`（状态 id 列表，`FREE_ACT` 即 `rfb.status.paralysis`）。
- `StatModifiers` 新增 `speed`（±100 界内，随其余修正同款校验）。
- demo 三件新物品锻炼全部三条路径：御火指环（fire → resistant）、疾行靴（speed +10）、镇静吊坠（麻痹免疫）。

## 3. 运行时语义

- **有效抗性** `effective_player_resistances()`：玩家基础档 ∪ 种族 ∪ 已装备物品 ∪ 其词条。确定性合并：任一来源 immune 即 immune；否则取最高正档（strong > resistant），若同时存在任一 vulnerable 来源则降一档；只有 vulnerable 时保持 vulnerable。玩家存档内的基础抗性保持权威、不吸收装备来源（穿脱即时生效、零迁移）。
- **状态免疫** `player_status_immunities()`：种族 ∪ 装备 ∪ 词条并集；`apply_ability_status_effect` 与近战骑手在落状态前查免疫表，免疫则跳过（沿用既有 skip/immune 事件形状），零后续 RNG。
- **装备速度**：`StatModifiers.speed` 进玩家派生速度管线（与 haste/slow 同一汇总点），影响调度器 energy 结算。
- 显示状态不变原则不受影响：有效抗性是派生值，不入存档/state hash（Schema 保持 v41）。

## 4. 协议与前端

- 协议 1.101：`InventoryItemDto`/`EquipmentItemDto` 新增可选 `resistances`/`statusImmunities`，按物品知识门控（未鉴定不显示）；快照玩家 `resistances` 改为暴露有效值。
- 前端物品行追加速度修正（`item-modifier-speed`）、抗性标签（`{类型}抗性：{档位}`）与免疫标签（`免疫：{状态}`），双语入 Fluent。

## 5. 契约

- contract-v101 基线：320 条 v100 fixtures 迁移后零行为漂移（差异仅 protocolVersion 与协议格式扩展）。
- 新场景 321（疾行靴：speed +10 进派生管线）、322（御火指环：火焰伤害按 resistant 减免）、323（镇静吊坠：麻痹免疫 skip 形状）。`minimumFixtureCount` 323。

## 6. 导入器（同轮回灌）

- ego（e_info）/神器（a_info）：`RES_*`/`IM_*`/`VULN_*` → `resistances`，`FREE_ACT` → `statusImmunities`，`SPEED` → `modifiers.speed`（pval 驱动）；ego 的"全部力量在不可表达旗标"跳过条件随之收窄。
- 种族：`calc_bonuses` 钩子体解析 `res_add`/`res_add_immune`/`res_add_vuln`/`free_act`/`pspeed` 语句折算为内在 `resistances`/`statusImmunities`/`modifiers.speed`。
- 实跑收割：ego 导入 105/160（+17）、神器 392/392 防御旗标落地、35 个词条 / 33 个种族 / 321 件物品带上防御表面；`RES_*`/`IM_*`/`SPEED`/`FREE_ACT` 全部退出未映射清单（`RES_FEAR` 属状态豁免类，待状态族对齐后处理）。

## 7. 遗留

- 进攻面旗标（SLAY_*/BRAND_*/血肉刃）：T2 后半。
- SEE_INVIS/REGEN/HOLD_LIFE/STEALTH/SUST_* 等能力性旗标：待对应系统。
- 种族 rank 动态（21 怪物种族）与 dynamic-adjustment 钩子仍在缺口报告。
