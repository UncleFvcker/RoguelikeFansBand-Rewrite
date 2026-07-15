# Contract v2 内容运行时迁移

状态：已完成

`contract-v2` 是启用真实编译内容包后的首个 active contract 基准。`contract-v1` 保留为历史记录，不再由当前核心执行 exact state hash 验证。

本次迁移原因：

- 旧核心使用手写地图、固定角色 ID 和占位 content hash；
- 新核心在运行时解码构建期生成的 `.rfbcontent`；
- 世界、地形、玩家、怪物和物品实例改由 `rfb.demo.original-v1` 定义；
- state hash 现在包含真实 content ID/hash、world ID 和内容包生成的物品状态；
- 回放 `stateHashSchemaVersion` 从 1 升级为 2。

玩家移动、碰撞、战斗伤害、事件顺序和 changed cells 规则没有在本次迁移中主动修改。大范围 hash 变化来自权威初始状态和内容身份变化，因此按政策新增版本目录，而不是为 20 个 fixture 创建普通差异豁免。

迁移命令只允许写入不存在的新目录，拒绝覆盖现有 baseline：

```powershell
cargo run -p rfb-contract -- migrate-baseline `
  tests/fixtures/contract-v1/scenarios `
  tests/fixtures/contract-v2/scenarios
```

当前 active 验证入口：

```powershell
cargo run -p rfb-contract -- validate-policy tests/fixtures/contract-v2/baseline-policy.json
cargo test -p rfb-contract
```
