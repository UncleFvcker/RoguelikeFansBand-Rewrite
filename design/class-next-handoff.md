# class-next 职业方向交接

更新时间：2026-08-12

分支：`codex/class-next`

起始基线：`main@3fb94bcd`

## 已完成批次

- 固定死亡领域的正式 High-Mage，以及后续领域可追加的 build/casting profile 接口；
- 正式 Archer 的职业身份、出生物、箭袋、制造弹药、射击暴击/破损率与射击能量；
- Archer 制造弹药的 RFB 中性 `apply_magic`：普通、优秀、极好、诅咒，正负命中/伤害附魔，
  `of Slaying` / `(Elemental)` ammo ego，伤害骰强化上限 9，完全鉴定，player-made 来源与 99% 折价；
- 陶器碎片、断木棍、骨骸和骷髅类尸体均可作为箭/弩栓材料；背包与脚下来源均只消耗一个。

## 稳定 ID 与跨分支边界

- 新增物品：`demo.item.shard-of-pottery`、`demo.item.broken-stick`；
- 复用 items 方向已确定的 `rfb-legacy.affix.slaying`，本分支携带同一份内容定义以保持可独立验证；
- 新增且仅供弹药使用：`demo.affix.ammo-elemental`；
- 没有新增 ability ID；制造弹药继续使用三个既有 `demo.ability.archer-create-*` ID。

集成 `items-next` 时，`rfb-legacy.affix.slaying` 必须保留 items 方向的同 ID 定义，不得生成第二个
ammo-slaying ID。若 items 方向随后导入陶器碎片或断木棍，也应统一到以上已声明 ID。

## 共享机制与集成注意

- `ItemEnchantmentsDto` 已从无符号改为有符号，以承载原版诅咒物品的负命中/负伤害；协议 JSON Schema
  因此改变，TypeScript 仍为 `number`。
- 物品持久状态新增 `damageDiceOverride`、`originKind`、`discountPercent`；普通物品默认值不写入 JSON，
  player-made 弹药会进入存档与 State Hash。
- 本方向没有自行提升 `STATE_HASH_SCHEMA_VERSION`，由 main 集成时与其他持久结构变化统一提升并判断
  fixture 刷新范围。
- demo pack 为 `1.260.0`，方向分支 lock hash 是
  `5b08136028dc48ead87b0118488f7803b858c60458e87117ae84897b00ccccfe`；main 合并其他内容后应统一重算。

## 明确未做

- 当前 `apply_magic` 只实现 RFB 的中性生成路径。Good/Bad Luck、Chance virtue、coffee/special mode、
  dungeon-specific good/great cap 与全局 `no_egos` 属于共享物品生成上下文，不在 Archer 内伪造；
- 其余 ammo ego（Holy Might、Returning、Endurance、Exploding）仍未导入；
- 法术按书分组的 UI 是下一独立批次，不属于本次 Archer Commit 2。

## 验收

- `cargo test -p rfb-core --no-fail-fast`
- `cargo test -p rfb-content`
- `cargo test -p rfb-protocol --features bindings`
- `cargo run -p rfb-protocol --features bindings --bin generate-bindings -- --check`
- `cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original`
