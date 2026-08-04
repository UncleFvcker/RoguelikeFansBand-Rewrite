<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Contract v166: Outpost Home

状态：active baseline；Outpost Home 完成。协议为 `1.133`，save 容器保持 v1，state hash Schema 升至 `61`。demo 内容包为 `1.156.0`，content hash 为 `4b6e95cdd3c09be3f08c68a458d1c81965fb81d457fe6a0908ffcacb6a15b400`；active baseline 包含 466 条 exact fixtures、零 waiver。

## 原版建筑审计

- 固定参考为 RFB commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 的 `lib/edit/f_info.txt`、`src/home.c` 与 `src/shop.c`。原版基础数字入口 1–9 中，当前已完成 General Store、Armoury、Weaponsmith、Temple、Alchemist、Magic Shop、Home、Bookstore，八类已可交互，只剩 Black Market。
- 新版 `shop.c` 另定义 Jeweler、Shroomery、Dragon。因此 Home 完成后仍待实现的正式交易设施共四类：Black Market、Jeweler、Shroomery、Dragon。
- Museum、White Horse Inn、城堡、Pest Control 与 `BUILDING_0..31` 属于建筑服务、任务或槽位系统，不作为普通商店数量计算，也不以不可交互门面提前放入 Outpost。
- Home 位于北侧独立建筑，入口 `(27,8)`，使用原版数字 glyph `8`。terrain 继续禁止 ASCII 大小写字母，所有字母保留给 actor。

## 仓储模型

- `townFacilities` 是独立内容根，`TownDefinition.facilityIds` 严格引用设施；首个 `TownFacilityDefinition` 类别为 `home`，不伪装成零价商店。
- Home 没有店主、钱包、价格、出生库存或按时间刷新。存入和取出免费且不耗世界时间，不改变金币，不推进 RNG。
- Home 库存是权威持久状态，进入 save、snapshot/update 和 state hash。所有物品实例 ID 在背包、地面、装备、商店和 Home 间全局唯一。
- 相容实例仅在物品属性与玩家知识状态均一致时聚合显示或合并；取回遵守物品 `maxStack`，源实例完全并入背包栈后清理无主知识记录。取出仍受玩家负重上限约束。

## UI 与验收

- 踩入 Home 入口自动打开独立仓储页，提供“取出/存入”标签、数量与最大值控制、当前和操作后负重、结构化成功或失败反馈。
- fixture 466 覆盖存入口粮、取回、最终 Home 库存和 save round-trip hash。核心测试另锁死背包相容堆叠与 Home ID 分配冲突。
- 桌面 supply-loop E2E 在购物、Warrens 消耗、拾取金币和回城补给之后进入 Home，存入口粮、创建原生存档、取回造成状态变化，再载入存档验证 Home 状态恢复。
- 当前仍在开发期，不为旧开发存档新增兼容迁移；缺少权威 Home 状态的旧内容状态不进入本契约。
