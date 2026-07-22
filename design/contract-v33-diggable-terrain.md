# Contract v33：挖掘与可破坏地形

状态：协议 1.33 / contract-v33 active baseline

## 已完成边界

- `TerrainDefinition` 新增成对的 `digToTerrainId` / `digCheckDifficulty`；源 terrain 必须不可行走且阻光，目标必须可行走且透光。
- 玩家新增 `digSkill`，`DigTerrain` 复用结构化检定和 v30 权威相邻交互查询。失败保持原 terrain，允许再次提交；成功转换 terrain 并刷新碰撞、视野和存档真值。
- 地表玩家西南侧新增稳定 `demo.terrain.echo-rubble`，成功后转换为普通地板。
- 怪物或地面物品占据目标时，交互标记不可用，直接命令也不会消耗检定 RNG。
- 前端大写 `T` 进入方向挖掘模式，小写 `t` 不被占用。

协议 1.33，内容包 1.27.0，content hash `4fdb1018d89fadee287aeff70b2ca059f62b867cfd8db8ed7f6409f7bbbd4765`，terrain 11。save v1 与 state-hash Schema v15 不变。active baseline 迁移 70 个并新增挖掘转换场景，共 71 个 exact fixtures。

## 与原版 RFB 对比

相同点：原版 `cmd2.c::do_cmd_tunnel_aux()` 同样读取角色挖掘能力和 feature power；失败可继续挖掘，成功通过 feature 转换移除阻挡地形；每次有效尝试消耗一个标准行动。

主动差异：重构用稳定 terrain ID、显式转换目标、结构化检定和权威交互查询表达规则。原版目标有怪物时会转为近战攻击；当前实现统一把被怪物或物品占据的挖掘目标判为不可用，避免一个地形命令隐式改变为战斗命令。

暂未实现：工具和装备加成、自动重复、疲劳/声音/德行、树木与矿脉、永久岩石、挖掘产物，以及挖掘秘密门时偶发搜索。
