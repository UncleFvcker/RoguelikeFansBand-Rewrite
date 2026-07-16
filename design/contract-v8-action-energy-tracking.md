# Contract v8：行动能量、速度与怪物追踪

状态：active contract baseline

## 1. 目标

contract-v8 把 contract-v7 的“每个玩家命令后统一执行一次邻接反击”替换为真正的确定性行动调度。规则参考旧 RFB 的核心关系：角色速度决定每个世界脉冲获得的行动进度，标准行动消耗 100；实现使用原创整数分段曲线，不复制旧随机能量表或 C 调度代码。

本阶段不加入大量怪物、职业或状态内容，只建立后续加速、减速、多次攻击、自动探索和怪物 AI 共用的时间底座。

## 2. 权威时间模型

- `turn`：已完成的玩家行动数，继续用于 UI、存档摘要和命令上下文；
- `worldTick`：能量调度脉冲数；
- `speed`：0–199 的基础速度，当前普通玩家和余烬微光均为 110；
- `energyNeed`：距离下次行动仍需补足的能量；
- 标准行动成本：100；
- 新游戏玩家从 `energyNeed = 0` 开始，可以立即行动；
- 新怪物从 `energyNeed = 100` 开始，不会在玩家第一次行动前抢先行动。

每次玩家行动后，调度器重复执行：

1. `worldTick + 1`；
2. 怪物按稳定实例 ID 顺序获得速度对应的能量；
3. `energyNeed <= 0` 的怪物执行一次行动并增加 100 成本；
4. 玩家获得速度对应的能量；
5. 玩家重新就绪或死亡时停止，等待下一条命令。

普通速度 110 每个脉冲获得 10，因此标准速度双方保持一比一行动。速度 120 每脉冲获得 20，可在普通玩家一次行动期间行动两次；速度 100 每脉冲获得 5，约每两次普通玩家行动执行一次。

当前所有已接入玩家命令仍消耗 100。不同物品、射击、施法和免费取消操作将在其所属系统加入时通过 `GameAction::energy_cost()` 扩展。

## 3. 怪物行动与追踪

怪物行动按以下顺序选择：

1. 与玩家相邻时执行现有 RFB 风格近战；
2. 否则使用八方向 BFS 寻找可到达的最近相邻格；
3. 墙体、地图边界、玩家格和其他怪物格不可穿越；
4. 同长度路径按“更接近玩家，然后固定方向顺序”选择；
5. 多怪物每个脉冲按实例 ID 排序，后行动者读取前一行动者已经更新的位置；
6. 无路可走时消耗本次行动并原地等待；
7. 玩家死亡后立即停止剩余怪物队列。

怪物移动会把旧位置和新位置写入 `changedCells`。带光源标签的怪物移动也会通过 `changedVisualCells` 更新照明和可见性。

## 4. 协议、存档和确定性

- 协议升级到 `1.8`；
- state hash Schema 升级到 `8`；
- active baseline 升级到 `contract-v8`，保留 32 个迁移后的 exact fixtures；
- 原创内容包升级到 `1.5.0`；
- content hash：`e597eb10e3eec454ea78e8ad4e874a8ef41732c6f497083f4fb698d9a1935c69`；
- `PlayerDto`、`EntityDto` 输出 `speed` 和 `energyNeed`；
- `GameSnapshot`、`GameUpdate` 和 `SavePayloadV1` 输出 `worldTick`；
- 玩家和怪物存档保存基础速度与当前 `energyNeed`；
- 旧 v1 存档缺失字段时，玩家默认速度 110/能量 0，怪物默认速度 110/能量 100；
- 核心继续接受内置内容包 1.0.0–1.4.0 的已知 hash。

contract-v7 继续作为历史近战基准保存，不再由 active 核心执行其精确 state hash。

## 5. 验收

- 普通速度怪物每次玩家行动追踪一步；
- 快速/缓慢怪物遵守同一能量曲线；
- 多怪物调换内部数组顺序后得到相同位置、事件和 state hash；
- 通道争用不产生重叠位置；
- 玩家死亡立即中断剩余怪物队列；
- 保存—载入后 `worldTick`、速度和剩余能量一致；
- 10,000 个命令的回放检查点无漂移；
- Tauri debug E2E 使用 webdriver 专属无怪物 fixture，避免存档/背包/渲染长流程被战斗打断；正式游戏和 contract 不使用该 fixture。

验证命令：

```powershell
cargo test --workspace
cargo run -p rfb-contract -- validate-policy tests/fixtures/contract-v8/baseline-policy.json
cargo run -p rfb-protocol --features bindings --bin generate-bindings -- --check
cargo run -p rfb-content --features schemas --bin generate-content-schemas -- --check
cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original
cargo clippy --workspace --exclude rfb-tauri --all-targets -- -D warnings
cd web
npm test
npm run typecheck
npm run e2e
```
