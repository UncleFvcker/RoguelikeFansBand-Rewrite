<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Contract v161: General Store transactions

状态：active baseline；Phase 18 Gate 5–6 完成。协议为 `1.128`，save 容器保持 v1，state hash Schema 为 `60`。demo 内容包为 `1.151.0`，content hash 为 `6af8e97c7c2e4f1fa56b6c6d004d267cfb24d238f5921478740a45f5a567d478`；active baseline 包含 461 条 exact fixtures、零 waiver。

## 内容与原版价值

- `ItemDefinition.baseValue` 成为普通商店定价的权威基础值；首批 RFB v1.3.0.7 数值为口粮 `3`、木制火把 `1`、黄铜灯笼 `30`、油瓶 `3`。
- `RaceDefinition.shopAdjustPercent` 默认 `110`，当前 Human 为 `100`。来源事实核对自固定本地源码的 `lib/edit/k_info.txt`、`src/shop.c`、`src/tables.c` 与 `src/object3.c`。
- General Store 严格且仅初始维护这四类补给。初始/维护数量范围分别为口粮 `1..5`、火把 `1..5`、灯笼 `1..2`、油瓶 `1..5`；维护间隔为 `10,000 worldTick`。
- 稳定店主为 Human `demo.shop-owner.outpost-mara-venn`，greed `108`，`purchasePriceCap = 500`。该 cap 是店主每单位最高报价，不是会递减的钱包。
- Gate 4 的 Outpost 地图、General Store `4×3` 占地和入口 `(16,8)` 保持不变。

## 定价与库存

- 价格因子按种族、Charisma 表、店主 greed 和同族九折依次使用确定性整数计算。当前 Warrior 有效 Charisma 14，与店主同为 Human，最终因子为 `100`。
- 当前买入/卖出单价为：口粮 `3/2`、火把 `1/1`、灯笼 `30/28`、油瓶 `3/2`。
- 首次建档在既有出生金币、食物、火把和职业物品 RNG 之后生成库存。店主、完整 `ItemInstance` 库存和最后维护 tick 进入 save、replay 与 state hash。
- 维护只在达到间隔后于商店入口触发，只补足短缺；未到期不抽 RNG。商店库存不会随 Warrens 的 reset-on-surface 刷新而清除。

## 权威交易

- `BuyFromShop` 与 `SellToShop` 是零世界时间的权威命令，但正常增加 revision/turn。命令先验证入口可达、数量、库存、金币、负重、合法性和算术溢出，再原子提交。
- 任一拒绝都不改变 RNG、世界时间、金币、玩家物品或商店状态。Warrior 负重上限调整为 120.0 磅，使 71.4 磅出生装备后仍有真实购买空间。
- General Store 收购正价值且非 corpse/remains 的背包物品。出售实例保留 fuel、charges、activation、affix、enchantment、curse 等完整运行时状态，可按同一实例状态回购；拆分数量时使用稳定的新实例 ID。
- 从商店购买的物品按原版商店边界设为 aware、appraised 和 identified。协议投影只在玩家位于入口时提供 stock、sell quotes、价格、可交易数量和店主信息。
- RFB 原版在购买后调用 `pack_carry`，由 `inv_combine_ex` 逐个吸收到相容的既有堆叠。重写版因此按稳定实例 ID 合并背包物品，并把相同种类且完整运行时状态相容的商店库存/出售报价投影为一行；交易数量可跨内部实例。燃料、充能、附魔、诅咒或知识不同的同名物品不合并。

## 存档、契约与暂缓

- 本项目仍处开发期且每次以新存档测试。Gate 5 不为缺少 `shopStates` 的旧开发存档猜测库存或补抽 RNG；此类存档严格拒绝。
- fixture 461 固定 Warrior 走到入口、购买两份口粮、出售一份出生口粮，并验证两笔交易后 `worldTick` 仍为 190、RNG 不变、余额 342、库存/负重变化和 save round-trip hash 一致。
- 核心测试覆盖种子库存、价格、批量交易、负重/金币/尸体拒绝、完整实例回购、维护和存档校验；replay 测试覆盖买卖与中途存档续跑；全部 461 条 active fixtures 保持 exact。
- Gate 6 已实现购买/出售标签、数量控件、金币/负重/饱食/光源展示和完整桌面 supply-loop 验收。本阶段不加入其他商店、家、博物馆、旅店、Pest Control、城镇 NPC、昼夜或荒野。
