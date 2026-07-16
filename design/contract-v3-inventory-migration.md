# Contract v3 背包权威状态迁移

状态：已完成

`contract-v3` 是加入拾取命令和背包权威状态后的历史基准，现已由 `contract-v6` 接替为 active。`contract-v1` 至 `contract-v5` 均保留用于迁移审计，不由当前核心执行 active exact state hash 验证。

本次迁移原因：

- 协议新增 `PickUp` 命令和 `InventoryItemDto`；
- 地面物品拾取后从地图状态移入背包，并按稳定实例 ID 确定拾取与堆叠顺序；
- 背包进入 `GameSnapshot`、`GameUpdate`、`.rfbsave` 和 `.rfbreplay` 权威状态；
- state hash 输入新增背包物品堆，因此 `stateHashSchemaVersion` 从 2 升级为 3；
- active fixtures 从 20 个增加到 22 个，新增成功拾取与空地拾取场景。

移动、碰撞、战斗、RNG 和内容包身份规则没有在本次迁移中主动修改。旧 fixture 的 hash 变化来自存档状态 Schema 增加背包字段，因此按政策建立新版本目录，不使用普通 waiver。

迁移命令拒绝覆盖已有目录：

```powershell
cargo run -p rfb-contract -- migrate-baseline `
  tests/fixtures/contract-v2/scenarios `
  tests/fixtures/contract-v3/scenarios
```

当前 active 验证入口：

```powershell
cargo run -p rfb-contract -- validate-policy tests/fixtures/contract-v3/baseline-policy.json
cargo test -p rfb-contract
```
