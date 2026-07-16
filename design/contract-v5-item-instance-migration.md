# Contract v5 装备属性与物品实例迁移

状态：已完成

`contract-v5` 是加入装备实际属性修正、稳定生成物品实例 ID 和部分数量丢弃后的 active contract 基准。`contract-v1` 至 `contract-v4` 继续作为历史迁移记录保留。

## 1. 协议与规则变化

- CoreTransport 协议升级到 1.5；
- `GameCommand` 新增 `DropQuantity { itemId, quantity }`，原有 `Drop { itemIds }` 继续负责整堆批量丢弃并保持旧回放可解码；
- 新增 `StatModifiersDto`，背包和装备 DTO 都携带由内容定义派生的属性修正；
- `PlayerDto` 同时输出基础最大生命、装备修正和最终最大生命；
- 回声护符提供 `maxHp +4`，装备后玩家最大生命从 10 变为 14，卸下或替换导致上限下降时当前生命会被安全钳制；
- 发光碎片的原创测试堆调整为 5 个，用于覆盖部分数量操作。

属性修正是派生数据：存档载入时以当前内容定义和权威装备列表重新计算，不信任存档中重复保存的派生 modifier 字段。

## 2. 稳定实例 ID 分配器

拆分物品堆时，核心生成 `generated.item.<序号>`：

- 序号严格单调递增，不复用已经消失的 ID；
- `nextItemInstanceSerial` 写入 `SavePayloadV1` 并进入 state hash；
- 旧存档缺失该字段时，核心扫描玩家、实体、地面、背包和装备中的现有 ID，从最大生成序号继续；
- 新存档中的序号若落后于现有生成 ID，载入时明确拒绝；
- 分配器不会消耗 RNG，因此拆堆不会改变随机数调用序列；
- 无效数量不会分配 ID，但仍遵守现有“有效命令消耗一回合并产生 none 事件”的规则。

部分丢弃少于整堆数量时，原背包实例保留原 ID 和剩余数量，地面拆分实例获得新 ID；丢弃整堆时仍移动原实例，不生成新 ID。

## 3. 内容与存档兼容

原创内容包升级到 1.2.0，content hash 为：

`cd2c813d224189c925a940e60a915fe3dcf6efa0ccadfc7363d06d428f56525f`

核心显式接受两个已知旧内置 hash：

- 1.0.0：`880610557b...`；
- 1.1.0：`0a76daadea...`。

旧状态原样载入，不补发物品，也不修改已有堆数量。1.1.0 存档中的回声护符在载入 1.2.0 内容后自动获得当前 `maxHp +4` 派生效果；下一次保存写入当前 content hash 和实例分配序号。未知或模组 hash 仍严格拒绝。

## 4. State hash、回放与 Contract

`nextItemInstanceSerial`、装备派生后的玩家最大生命以及新的 DTO 字段改变权威序列化结果，因此 `stateHashSchemaVersion` 从 4 升级到 5，active baseline 迁移到 `contract-v5`。

v5 共 28 个 fixtures、0 个 waiver，并在迁移后的 26 个场景之外新增：

- 连续两次部分丢弃，断言生成序号推进到 3并完成存档回环；
- 数量为 0 和超过现有堆数量的无效部分丢弃，断言分配器保持为 1。

现有装备场景同时断言回声护符装备后的玩家最大生命为 14。

验证入口：

```powershell
cargo run -p rfb-contract -- validate-policy tests/fixtures/contract-v5/baseline-policy.json
cargo test -p rfb-contract
```
