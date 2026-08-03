<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Contract v157: Gold source and wallet

状态：historical baseline；Phase 18 Gate 1 完成。协议升至 `1.124`，save 容器保持 v1，state hash Schema 升至 `56`。demo 内容包升至 `1.147.0`，content hash 为 `70f21e8d8f28a2102a8b28e5c6cabf83137afb4532e5c2868d10fb7c1e5e5012`；该基线包含 457 条 exact fixtures、零 waiver。

## 固定来源结论

固定 RFB v1.3.0.7 来源中，普通角色出生金币为 `2d300+200`，玩家钱包上限为 `999999999`。普通地牢通过 `randnor(2,3)` 分配金币堆，再按地图相对标准面积随机舍入；`make_gold` 先按对象深度选择币种和基础值，再生成金额。怪物同时允许物品和金币时，成功掉落有 20% 选择金币；Small kobold 的 `DROP_60 | DROP_WARRIOR` 因此先判定 60% 掉落，再判定金币分支。

## 实现

- 玩家权威状态新增 `gold: u32`。生产 Warrior 在既有 build 与 HP 序列之后固定抽两次出生金币；其他历史测试 build 保持 0 金币和原 RNG 顺序。
- 地面金币使用独立 `GoldPile { id, position, amount, appearance }`，不进入 `ItemInstance`、背包、装备或负重。实例 ID 使用全局单调的 `generated.gold.N`，当前层与所有 stored floor 共享分配器。
- Warrens 每层在 terrain/connection、actor 和普通物品之后生成面积缩放的金币堆。金额与铜、银、石榴石、金币、宝石、秘银和精金外观按当前对象深度确定；怪物金币额外使用原版深度修正。
- Small kobold 的 60% 掉落成功后先抽 20% 金币替换分支；金币和普通 Warrior 物品互斥。遗骸仍在普通掉落事务之后生成。
- `PickUp` 先按稳定实例 ID 收集玩家格上的全部金币，再处理普通物品。钱包达到上限时仍移除金币堆，事件只报告实际增加的金额和最终余额；金币拾取不增加携带重量。
- 金币、地面金币堆与下一实例序号随当前层、stored floor、save/replay 和 state hash 保存。验证拒绝零金额、非法余额、不可行走落点、非法生成 ID、跨楼层重复 ID，以及落后于现有 ID 的分配器。
- 协议公开玩家余额、可见金币堆、金币外观，以及拾取/怪物掉落事件。Web 右侧玩家栏显示余额，地图与附近面板使用 `$`、外观、金额、方向和距离展示可见金币；中英文事件文本覆盖拾取和掉落。

## 迁移与确定性

- save 容器仍为 v1。旧 save 缺少钱包时确定性迁移为 0，缺少金币堆时迁移为空，下一金币 ID 从已保存的当前层与 stored floor 推导；完全没有历史金币时为 1。
- 旧档载入不补掷出生金币，不回填已生成楼层的金币，也不推进 RNG、世界时间或 revision。重新返回地表并进入 `reset-on-surface` Warrens 后，新的地牢实例按当前规则正常生成金币。
- state hash Schema 56 纳入玩家余额、当前层/离层金币堆和下一实例序号。协议 1.124 的 TypeScript bindings 与 JSON Schema 同步生成。

## 暂缓边界

- 本 Gate 不增加购买、出售、店主钱包或商店价格；金币先建立可验证来源和玩家持有端。
- 食物、饱食度、光源燃料和 Outpost 商店分别留给 Gate 2–5。
- Fame、virtue、no-selling、coffee-break、特殊种族金币修正，以及低概率 `GREAT_OBJ` 超深度提升继续暂缓。
- 不为金币增加特殊无限刷怪。返回地表后重新进入仍按既有 `reset-on-surface` 生命周期刷新 Warrens 地图、怪物、物品和金币。

## 验收

- 核心测试覆盖 Warrior 出生范围与固定 seed、楼层金币生成、拾取优先级、钱包饱和、独立负重、当前层/离层保存、旧档迁移、ID 唯一性和 Small kobold 金币/物品/无掉落分支。
- fixture 457 固定 seed 42 Warrior 出生余额 346、两次 RNG draw、save round-trip 与 Schema 56 state hash；全部 457 条 active fixtures 保持 exact，零 waiver。
- 内容源码验证、协议/schema 生成检查、Rust workspace、contract、Web 测试、类型检查与 UI 构建共同构成 Gate 1 验收。
