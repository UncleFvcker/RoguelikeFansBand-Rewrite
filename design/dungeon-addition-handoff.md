# 新增地牢交接文档

更新时间：2026-08-14

## 1. 接手点

- 工作树：`D:\codex\RoguelikeFansBand-Rewrite-monsters-next`
- 分支：`codex/monsters-next`
- 功能基线：`1f486d1bc`（`feat: add Troll cave dungeon content`）
- 内容包基线：`1.344.0`
- 当前已完成、可作为模板的正式地牢：Warrens、兽人洞穴、巨魔洞穴、卡美洛、
  潮汐洞穴、黏液洞穴、藏身处、隐秘天地。
- 日常开发只运行本批新增的聚焦测试、内容验证和相关类型检查。全量 replay、fixture、
  desktop/E2E 留到合并或明确的里程碑验收。

本文说明如何把一个 RFB 原版地牢从“权威事实”推进到“可进入、可探索、可征服、
可保存恢复”的完整内容。它不是新的地牢框架设计；优先复用现有定义和运行时。

## 2. 权威来源

所有新地牢事实以 `D:/codex/Frogcomposband` 仓库的 `master` Git ref 为准。只能通过
Git objects 读取，不读取该仓库当前检出的分支或工作树。

常用来源：

- `lib/edit/d_info.txt`：地牢索引、坐标、深度、生成标记、生态偏好、守卫和奖励；
- `lib/edit/r_info.txt`：守卫和地牢专属怪物；
- `lib/edit/k_info.txt`：普通最终物品；
- `lib/edit/e_info.txt`：最终 ego；
- `lib/edit/a_info.txt`：最终神器；
- `lib/edit/f_info.txt`：入口和楼层地形；
- `lib/edit/w_info.txt`：世界地点关系；
- `src/`：仅当编辑表不能说明替代选择、奖励或生成规则时，再追运行时代码。

推荐读取方式：

```powershell
git -C D:/codex/Frogcomposband show master:lib/edit/d_info.txt
git -C D:/codex/Frogcomposband show master:lib/edit/r_info.txt
git -C D:/codex/Frogcomposband grep -n "目标名称或标记" master -- lib src
```

中文显示名必须逐字采用 RFB `master` 中的中文表或源字符串。没有权威中文名时记录为
unresolved，不自行翻译。

## 3. 先判断改动属于哪一层

| 改动 | 通常需要更新 |
| --- | --- |
| 仅增加 actor、物品、地形、表和楼层 JSON | pack version、content lock、聚焦内容/核心测试 |
| 增加内容 DTO 字段 | Rust definition、validation、生成 schema、内容与运行时测试 |
| 增加持久化状态或 state-hash 输入 | state-hash schema、存档/恢复测试，合并阶段评估 replay fixture |
| 修改协议投影 | protocol version、Rust/TypeScript bindings、协议测试 |
| 修改公共初始化或共享 RNG 顺序 | 聚焦 RNG 测试，合并阶段评估完整 fixture 刷新 |

项目默认从新存档开始。除非用户明确要求，不为旧开发存档增加兼容分支。

一个普通地牢应当是内容改动。只有原版语义确实无法由当前 DTO 和运行时表达时，才扩张
共享机制；不要为了单个地牢另建第二套楼层、守卫、入口或奖励系统。

## 4. 推荐实施顺序

### 4.1 A：锁定原版计划

先在 `packs/rfb-demo-original/legacy-wilderness-selection.json` 的 `dungeonPlans` 中增加
计划，并在 `crates/rfb-legacy-import/src/content.rs` 增加一个聚焦同步测试。

计划至少锁定：

- `sourceIndex`、`sourceName`、稳定 `id`；
- 世界坐标、最小/最大深度；
- `monsterDivisor` 和 `monsterPreferences`；
- `generationFlags`、可用时的地面比例和 `tunnelPercent`；
- 守卫 source index、英文名、权威中文名和等级；
- 最终普通对象、ego 或神器；
- `substituteSourceIndex`，如果原版与另一地牢共享入口并存在替代关系。

示例骨架：

```json
{
  "sourceIndex": 99,
  "sourceName": "Source dungeon name",
  "id": "demo.dungeon.example",
  "position": { "x": 10, "y": 20 },
  "minimumDepth": 10,
  "maximumDepth": 20,
  "monsterDivisor": 16,
  "generationFlags": ["CAVE"],
  "monsterPreferences": ["R_CHAR_o"],
  "guardian": {
    "sourceIndex": 999,
    "sourceName": "Source guardian name",
    "chineseName": "权威中文名",
    "level": 20
  },
  "finalObject": { "tval": 1, "sval": 2 }
}
```

这一批只冻结来源事实，不急着让入口可进入。原版 source index 是同步和交接约束，不应
复制成永久的运行时硬编码名单。

### 4.2 B：补齐依赖内容

正式楼层前先确认以下依赖已经存在：

- 最终守卫 actor；
- 带 `legacyDungeonIndices` 的专属 actor；
- 入口、墙、地面、楼梯、竖井、河流和湖泊需要的 terrain；
- 最终普通物品、ego、affix 或 artifact；
- 最终奖励 loot table；
- encounter table 依赖的标签、移动方式和 habitat。

已有稳定内容必须复用，不能因名称相近再创建一份 actor、item 或 terrain。若最终奖励带
特殊被动或激活，先把真实机制和聚焦测试做完，再引用到奖励表；不能用展示词缀冒充。

### 4.3 C：增加地牢定义和荒野入口

主要共享文件是 `packs/rfb-demo-original/worlds/middle-earth.json`。

每个地牢在 `WorldDefinition.dungeons` 中至少有：

```json
{
  "id": "demo.dungeon.example",
  "legacyIndex": 99,
  "rootFloorId": "demo.floor.example-depth-10",
  "guardianActorKindId": "demo.actor.example-guardian"
}
```

`guardianActorKindId` 是普通随机生态排除守卫的权威来源。不要再给 actor 复制一份
`guardian` 标签。当前运行时只会保护启用地牢的守卫；被替代、已压制地牢的守卫不享受
这层保护，符合现有替代语义。

在 `world.wilderness.locations` 增加精确坐标的 dungeon location。根层的
`entryTerrainId` 必须对应局部荒野或城镇地图中真实存在的入口 terrain。独立入口通常要
增加：

- `packs/rfb-demo-original/terrain/<slug>-entrance.json`；
- 局部荒野 special location 或城镇 floor 上的入口位置；
- `locales/en-US/content.ftl` 和 `locales/zh-CN/content.ftl` 的名称/描述。

不要让世界地图直接绕过局部入口进入地牢。进入、返回地表和召回继续走现有地牢闭环。

### 4.4 D：增加生态表

新增 `packs/rfb-demo-original/encounterTables/<slug>.json`。普通原版偏好用
`globalAllocation` 表达：

```json
{
  "preferredGlyphs": ["o"],
  "preferredTags": ["orc"],
  "preferredMovementModes": ["swim"],
  "preferredHabitats": ["shore"],
  "specialDiv": 16,
  "ambientChanceOneIn": 2
}
```

四组偏好是 OR 关系：任一命中即保留完整基础权重；全部未命中时权重乘
`specialDiv / 64`。在加权前仍会排除：

- `wildOnly` 怪物；
- 其他地牢或任务限定怪物；
- 不匹配的 `legacyDungeonIndices`；
- 当前启用地牢的最终守卫；
- 其他既有召唤/生成限制。

因此不要为了让纯海洋荒野怪物进入水系地牢而放宽 ocean-only 边界。潮汐洞穴已经提供
`aquatic | swim | shore` 的正确参考。

### 4.5 E：增加楼层和生成内容

普通线性地牢的楼层放在 `middle-earth.json` 的 `proceduralFloors` 中。常用配套文件：

- `terrainFeatureTables/<slug>.json`：草、沼泽、浅水等房间地面替换；
- `lootTables/<slug>-final-reward.json`：最终固定奖励；
- 已有通用 `demo.loot-table.base-items`：普通楼层掉落。

每层至少要正确设置：

- `id`、`nameKey`、`lifecycle: "dungeon"`、`dungeonId`、`depth`；
- `returnFloorId`，非末层还要成对设置 `nextFloorId` 和 `downStairTerrainId`；
- encounter/loot table、生成预算和 layout；
- 宽高、墙、地面、上下楼梯、秘密门、陷阱；
- 根层 `entryTerrainId`；
- 末层 `finalFloor: true` 和唯一 guardian。

地牢拓扑遵守以下不变量：

1. 一个地牢只有一个根层，根层 `returnFloorId` 指向世界初始地表层。
2. 每个非根层恰好有一个较浅的普通楼梯父层，且 `returnFloorId` 必须指向它。
3. 竖井是跨两层的捷径，不参与父子树判定，不能替代普通楼梯主干。
4. 显式普通楼梯必须相差 1 层，显式竖井必须相差 2 层。
5. 只有叶子层可设置 `finalFloor` 和 guardian；所有叶子都必须有 guardian。
6. 地牢定义、末层 guardian 和 actor kind 三者必须一致。

简单线性层可以用 `layout.stairs` 自动放置。需要竖井、分支或精确连接时使用
`connections`：

- 有显式 `connections` 时不能同时设置 `layout.stairs`；
- 根层要用 `entryConnectionId` 指向返回地表的连接；
- 所有地牢内部连接必须通过 `targetConnectionId` 双向互指；
- terrain 必须可行走，并且恰好带一个 `stairs-up`/`stairs-down` 标签；
- 竖井 terrain 还必须带 `shaft` 标签；
- 即使使用显式连接，仍保留正确的 `returnFloorId`、`nextFloorId` 和上下楼梯 terrain，
  作为楼层主干语义。

显式根层示例：

```json
{
  "returnFloorId": "demo.floor.surface",
  "entryTerrainId": "demo.terrain.example-entrance",
  "entryConnectionId": "demo.connection.example-depth-10-stairs-up",
  "nextFloorId": "demo.floor.example-depth-11",
  "connections": [
    {
      "id": "demo.connection.example-depth-10-stairs-up",
      "kind": "stairs",
      "terrainId": "demo.terrain.stairs-up",
      "targetFloorId": "demo.floor.surface"
    },
    {
      "id": "demo.connection.example-depth-10-stairs-down",
      "kind": "stairs",
      "terrainId": "demo.terrain.stairs-down",
      "targetFloorId": "demo.floor.example-depth-11",
      "targetConnectionId": "demo.connection.example-depth-11-stairs-up"
    }
  ]
}
```

生成配置优先复用现有能力：矩形/十字/洞穴房间、streamer、river、lake、cavern、
destroyed、maze 和 pit。注意：

- `generationBudget.featurePlacements` 必须不大于 terrain feature table 的 `rolls`，
  且当前深度必须至少有一个合格条目；
- river 与 water lake 同层存在时应使用一致的深水/浅水 terrain 对；
- 深水通常不可行走、浅水可行走；当前 rubble lake 的合法边界是深层 rubble、浅层等于
  基础地面；
- 入口、所有楼梯和连接必须处于同一可达区域；不要只看 terrain 数量而忽略连通性；
- 计划里暂时无法精确还原的低概率布局标记可以保留在审计数据中，并在交接中明确适配，
  不要悄悄删除原版事实。

### 4.6 F：守卫、奖励和征服闭环

末层示例：

```json
{
  "finalFloor": true,
  "guardian": {
    "instanceId": "demo.guardian.example.1",
    "actorKindId": "demo.actor.example-guardian",
    "rewardLootTableId": "demo.loot-table.example-final-reward"
  }
}
```

现有核心负责：

- 末层固定生成守卫；
- 守卫死亡后设置 `DungeonState.guardianDefeated`；
- 普通地牢结算 `campaign.dungeonConquestPoints`（当前为 10,000）；
- 最终奖励只生成一次；
- 保存恢复后不重复生成守卫或奖励；
- 只有 `campaign.victoryDungeonIds` 中的地牢会触发整局胜利。

普通奖励使用 `rewardLootTableId`。固定神器可用现有神器奖励和替代路径，不能另写绕过
神器唯一生命周期的生成逻辑。

## 5. 共享入口与 SUBSTITUTE

现有替代关系使用：

```json
{
  "id": "demo.dungeon.primary",
  "substitution": {
    "alternateDungeonId": "demo.dungeon.alternate",
    "alternateGateOneIn": 32
  }
}
```

- 只有 primary 声明 substitution；alternate 不能再声明 substitution。
- 省略 `alternateGateOneIn` 时，现有实现按新局种子稳定做 50/50 选择。
- 带 gate 时先通过额外稳定门控；藏身处/隐秘天地以 `32` 实现约 1/64 替代。
- 选择不消耗普通游戏 RNG，并通过 `DungeonState.suppressed` 持久化和参与 state hash。
- 两个地牢都必须有完整 definition、floor、wilderness location；它们共享坐标和入口 terrain。
- 只有当前启用的地牢会显示、进入、召回和征服；被压制地牢不可进入。
- 两个根层可以共享同一个 `entryTerrainId`，但内容验证只允许正式 substitution pair 这样做。

巨魔洞穴/兽人洞穴是 50/50 共享荒野入口的参考；藏身处/隐秘天地是共享城镇入口并带
稀有替代门控的参考。

如果只是给新地牢对填现有 substitution 字段，这是内容改动。只有修改替代状态模型本身
才需要再次升级 state-hash/schema。

## 6. 入口守卫和进入条件

如果原版需要入口前的软战斗门，使用 `DungeonDefinition.entranceGuardian`；它与末层守卫
是两个独立生命周期。若需要硬条件，使用现有 `entryRequirements`：

- `task-status`；
- `dungeon-conquered`；
- `carried-item`。

所有进入条件是 AND 关系，并在创建地牢实例和消耗进入 RNG 前检查。不要把硬条件伪装成
入口 actor，也不要把入口守卫死亡误记为地牢征服。

## 7. 聚焦验收

每批使用独有前缀，例如 `p91a_`、`p91b_`，只运行新增测试。

### 7.1 原版计划测试

位置：`crates/rfb-legacy-import/src/content.rs`

断言 source index、坐标、深度、偏好、生成标记、守卫、奖励和替代关系。示例命令：

```powershell
cargo test -p rfb-legacy-import p91a_
```

### 7.2 内容绑定测试

位置：`crates/rfb-content/src/tests/world.rs`

至少断言：

- dungeon definition、wilderness location、入口 terrain；
- 深度集合连续且根/末层正确；
- encounter preference 和普通怪物降权；
- 地形表、生成预算、河湖/矿脉/竖井配置；
- 守卫 kind、奖励表和地牢专属 actor 范围；
- 替代地牢双方定义和共享入口关系。

```powershell
cargo test -p rfb-content p91
```

### 7.3 核心闭环测试

位置：`crates/rfb-core/src/game/tests/world.rs`；生态过滤也可放在
`crates/rfb-core/src/game/tests/monster_ecology.rs`。

至少覆盖：

- 从局部入口进入根层、逐层下降、向上返回地表和召回；
- 所有层楼梯/连接可达，竖井目标和回程正确；
- 地点限定、wild-only、其他地牢 actor 和守卫不会提前随机生成；
- 守卫唯一生命周期、10,000 分、最终奖励和征服状态只结算一次；
- 保存恢复后当前地牢实例、替代选择、守卫死亡和奖励状态不回退；
- 替代对用两个固定种子分别覆盖 primary 和 alternate。

```powershell
cargo test -p rfb-core p91
```

不要用全层逐步游玩的长 fixture 代替窄测试。与移动无关的前置状态直接设置玩家位置；只在
验证入口、楼梯、竖井和召回时实际执行移动/进入命令。

## 8. 内容包更新与日常命令

实现和聚焦测试通过后：

1. 提升 `packs/rfb-demo-original/pack.json` 的 patch version；
2. 运行 `inspect-source` 取得新内容 hash；
3. 把相同 version/hash 写入 `packs/rfb-demo-original/content.lock.json`；
4. 运行 `verify-source`。

```powershell
cargo run -q -p rfb-content --bin rfb-contentc -- inspect-source packs/rfb-demo-original
cargo run -q -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original
cargo check -p rfb-content -p rfb-core
git diff --check
```

若改了 Rust 文件，再运行相关 `cargo fmt`。若新增了内容 schema 字段，再运行：

```powershell
cargo run -p rfb-content --features schemas --bin generate-content-schemas -- --check
```

方向分支不要主动运行 `verify-all`、`refresh-all` 或完整桌面测试。合并时按最终组合改动判断
是否需要刷新 fixture。

## 9. 高冲突文件与合并规则

以下文件经常被其他方向同时修改：

- `packs/rfb-demo-original/worlds/middle-earth.json`；
- `packs/rfb-demo-original/legacy-wilderness-selection.json`；
- 两个 locale 的尾部；
- `crates/rfb-content/src/tests/world.rs`；
- `crates/rfb-core/src/game/tests/world.rs`；
- `pack.json` 和 `content.lock.json`；
- 若扩张状态：persistence、state hash、protocol 和 fixture。

合并冲突不能简单选择一侧文件：

- 保留双方稳定 ID、world location、dungeon definition 和 floor；
- 合并后的 `dungeons`、`wilderness.locations` 和 `proceduralFloors` 必须同时完整；
- locale 合并后检查 key 不重复；
- pack version/hash 以最终合并内容重新计算，不沿用任一分支的旧 hash；
- 不回滚其他工作树已增加的 schema、状态字段或测试。

## 10. 常见失败

- 只写 `dungeonPlans`，却没有正式 dungeon definition、location 和 floor：计划不可游玩。
- 只给 primary 增加 wilderness location：alternate 被选中时世界地图或入口解析失败。
- 根层 `entryTerrainId` 在局部地图不存在：地牢无法从游戏内进入。
- actor 自带 guardian 标签，地牢定义又写一份：产生两个权威来源。
- 显式 `connections` 与 `layout.stairs` 同时存在：内容验证失败。
- 内部连接缺少 reciprocal `targetConnectionId`：保存恢复或回程不稳定。
- 只用竖井连接楼层：破坏单根普通楼梯树。
- `nextFloorId` 与 `downStairTerrainId` 不成对：内容验证失败。
- 末层不是叶子，或叶子没有 guardian：地牢拓扑验证失败。
- 用水系偏好放宽 ocean-only：海洋荒野怪物错误进入地下城。
- 奖励只加显示文本，没有真实 affix/passive/activation：行为与原版不符。
- 结算守卫时另写分支：容易重复发奖励、分数或绕过神器唯一生命周期。
- 为内容新增不必要的持久状态：引入 schema 和 replay 成本。

## 11. 交接回报模板

完成一个地牢批次后，在交接中给出：

```text
分支 / 基线 / 提交：
地牢 ID / legacy index：
坐标 / 深度：
入口与替代关系：
守卫 / 最终奖励：
专属 actor 与生态：
新增 terrain / encounter / loot：
共享运行时改动：无 / 列出
存档、state hash、协议、RNG 影响：
已运行的聚焦测试：
未运行的全量测试：
已知适配或暂缓项：
高冲突文件：
```

接手者应先读本文件、`design/parallel-worktree-handoff.md`，再按需要读：

- `design/contract-v35-dungeon-expedition-lifecycle.md`；
- `design/contract-v63-dungeon-tree-guardian-mirrors.md`；
- `design/contract-v65-dungeon-instance-identity.md`；
- `design/contract-v67-dungeon-entrance-guardians.md`。
