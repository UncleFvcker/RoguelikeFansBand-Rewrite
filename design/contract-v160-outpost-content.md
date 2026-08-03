<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Contract v160: Outpost content model

状态：active baseline；Phase 18 Gate 4 完成。协议升至 `1.127`，save 容器保持 v1，state hash Schema 升至 `59`。demo 内容包升至 `1.150.0`，content hash 为 `a03b7c96e880a8c1d1e0c86a323e9e3333d84d2670eef9404d884cbad6d50779`；active baseline 包含 460 条 exact fixtures、零 waiver。

## 内容模型

- 内容格式新增严格的 `towns` 与 `shops` 根、对应 JSON Schema，以及 `TownDefinition`、`ShopDefinition` 和首个 `general-store` 类别。
- `demo.town.outpost` 拥有稳定地表层 `demo.floor.surface`，并显式列出 `demo.shop.outpost-general-store`；世界通过 `townId` 指向该城镇。
- General Store 当前只声明城镇归属、类别、入口坐标 `(16,8)` 和入口 terrain。店主、库存、价格、钱包、维护与交易命令仍属于 Gate 5，不以占位字段提前进入内容。
- 内容验证要求 world、town、shop、floor、入口坐标和入口 terrain 双向一致，并拒绝未归属、重复归属、错误入口或跨城镇引用。

## Outpost 地表

- `demo.floor.surface` 保持原有稳定 ID，但使用独立设计的固定 Outpost 室外布局。玩家出生点为 `(32,11)`，General Store 入口为 `(16,8)`，Warrens 入口为 `(51,16)`。
- General Store 使用 `x=14..17、y=6..8` 的 `4×3` 紧凑占地；除 `(16,8)` 的入口外，建筑内部与轮廓全部由 `demo.terrain.outpost-wall` 填满。
- 新增 `demo.terrain.outpost-wall` 与 `demo.terrain.general-store-entrance`。ASCII 和 image-demo tileset 均有明确映射，商店入口使用可辨认的 `1` glyph。
- 地表继续使用环境光；返回地表仍结束当前 Warrens 实例，城镇与商店访问状态不随地牢刷新而清除。

## 权威状态与投影

- core 保存持久 `TownState` 与 `ShopState`。Outpost 在生产流程出生时已访问；General Store 初始未访问，玩家进入声明的入口格后变为已访问。
- 协议新增 `TownDto`、`ShopDto`、`TownStateSaveDto` 与 `ShopStateSaveDto`。snapshot 和每次 update 都只在当前地表投影城镇、商店、访问状态及 `playerAtEntrance`。
- 进入入口只记录访问事实，不自动打开商店 UI，也不推进额外回合。实际交易状态与界面在 Gate 5/6 建立。

## 迁移与确定性

- 旧内容 hash 载入时，仅当 `demo.floor.surface` 缺失当前声明的商店入口才重建固定 Outpost 地表。合法可行走的旧玩家坐标保留；无效或不可行走坐标回退到新出生点。
- 当前层和已存储的旧地表都使用相同确定性迁移；探索记忆、旧连接和旧区域随被替换地表清除。
- 迁移不改变 revision、turn、world tick 或 RNG。当前位于 Warrens 时，地牢地形、实体、物品、金币、连接、区域、探索状态和 RNG 保持不变；已存储 Warrens 楼层不会重建。
- state hash Schema 59 纳入 town/shop 访问状态。协议 1.127、TypeScript bindings 与 JSON Schema 同步更新。

## 验收与暂缓边界

- fixture 460 固定 Warrior 从 Outpost 出生、进入 General Store 入口、访问状态更新和 save round-trip；全部 460 条 active fixtures 保持 exact，零 waiver。
- 核心测试覆盖内容投影、入口访问、状态拒绝、当前/已存储地表迁移、玩家坐标保留和活跃 Warrens 层不变。
- 本 Gate 不加入店主、库存、定价、商店钱包、买卖、每日维护或商店面板。Gate 5 将在已有稳定身份与入口上建立这些权威交易规则。
