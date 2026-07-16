# Contract v4 装备与批量丢弃迁移

状态：已完成

`contract-v4` 是加入权威装备列表、装备/卸下命令和批量丢弃后的历史 contract 基准，现已由 `contract-v5` 接替为 active。`contract-v1` 至 `contract-v4` 继续作为历史记录保留，不由当前核心执行 active exact state hash 验证。

## 1. 迁移内容

- CoreTransport 协议升级到 1.4；
- `GameCommand` 新增 `Equip`、`Unequip` 和 `Drop`；
- `GameSnapshot`、`GameUpdate` 与 `SavePayloadV1` 新增 `equipment`；
- `InventoryItemDto` 提供由内容定义派生的可选 `equipmentSlot`；
- 内容物品定义新增可选 `equipmentSlot`，可装备物品当前必须 `maxStack = 1`；
- 原创内容包升级到 1.1.0，新增 `demo.item.echo-charm` 和 `charm` 槽位；
- HTML 背包加入复选框、多选计数、装备所选、丢弃所选和独立装备列表；
- 批量丢弃一次移动所选完整物品堆，并只产生一个权威命令和一次回合推进。

本版本中的装备尚未增加属性加成，丢弃也只支持完整物品堆。这两个限制已在[Contract v5 装备属性与物品实例迁移](contract-v5-item-instance-migration.md)中解决。

## 2. 兼容性

`equipment` 和物品的装备元数据都使用 Serde 默认值，因此旧 `.rfbsave` 缺失字段时仍可解码。

原创内容包 hash 从 `880610557b...` 变为 `0a76daadea...`。核心只对这个已知的 1.0.0 → 1.1.0 内置内容变化提供显式迁移：

- 旧存档中的地图、地面物品、背包和角色保持原样；
- 不会向旧存档凭空补发新增的回声护符；
- 载入后的快照使用当前 content hash；
- 下一次保存写入当前格式；
- 其他未知或模组 content hash 仍严格拒绝。

## 3. State hash 与回放

装备列表进入权威存档 payload 和 state hash，`stateHashSchemaVersion` 从 3 升级到 4。回放新增装备、卸下和批量丢弃的完整复验测试。

本次基准迁移使用：

```powershell
cargo run -p rfb-contract -- migrate-baseline `
  tests/fixtures/contract-v3/scenarios `
  tests/fixtures/contract-v4/scenarios
```

active fixtures 从 22 个增加到 26 个，新增：

- 装备回声护符并执行存档回环；
- 卸下护符；
- 一次丢弃两堆物品；
- 尝试装备不可装备的发光碎片。

当前验证入口：

```powershell
cargo run -p rfb-contract -- validate-policy tests/fixtures/contract-v4/baseline-policy.json
cargo test -p rfb-contract
```

`contract-v4` 没有 waiver；所有变化通过新版本基准显式记录。
