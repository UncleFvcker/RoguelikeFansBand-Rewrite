# 城镇系统交接

更新时间：2026-08-09

## 1. 当前工作上下文

- 工作树：`D:\codex\rfb-town`
- 分支：`codex/town`
- 基线：`40570042 merge: integrate town worktree updates`
- `W0`–`W5` 已在基线中；`L0`–`L2` 是本工作树中尚未提交的修改。
- 当前工作树内容包版本为 `1.224.0`，内容哈希为
  `0385e44785fa472bbc711adabac88ee34862dc749ca56ed351d0eb586eab770d`。
- 当前协议为 `1.157`，state-hash Schema 为 `77`。`L0`–`L2` 没有修改协议、
  state-hash 输入结构或 save 容器版本。
- 本项目使用新存档测试，不为旧开发存档增加兼容路径。

其他对话接手时，应直接在此工作树继续或先提交并合并这里的修改。不要在
`D:\codex\RoguelikeFansBand-Rewrite` 中重新实现 `L0`–`L2`，也不要覆盖这里
尚未提交的共享文件修改。

## 2. 权威来源

所有 RFB 原版事实以 `D:/codex/Frogcomposband` 仓库的 `master` Git ref 为准，
通过 Git objects 读取，不读取该仓库当前检出的工作树文件。

城镇和下一条路线主要使用：

- `lib/edit/w_info.txt`：世界地图、城镇和地牢坐标；
- `lib/edit/t_info.txt`、`lib/edit/t_ana.txt`：阿南巴标准设施与旅店；
- `lib/edit/d_info.txt`：兽人洞穴深度、生成规则、生态偏好和守卫；
- `lib/edit/f_info.txt`：设施名称与入口地形；
- `src/bldg.c`、`src/wild.c`、`src/cmd2.c`：设施、城镇旅行和地点切换行为。

中文名称必须逐字采用 RFB `master` 的中文表或源字符串。没有权威中文名称时
标记 unresolved，不自行翻译。

## 3. 已完成的系统底座

### 3.1 既有前哨站

前哨站已经提供八类可交易商店、共享交易逻辑、Home、任务服务和 Warrens
入口。购买、出售、库存维护、Home 存取、商店访问状态、存档和投影均是 Core
权威行为。

### 3.2 W0–W5：城镇依赖的世界系统

- `W0` 导入 `99 x 66` RFB wilderness、15 类基础地形、道路、危险等级和正式
  地点；未实现地点不会生成占位内容。
- `W1` 增加 local/world 地图尺度、世界坐标和世界种子，并复用现有地图渲染、
  镜头与查看模式。
- `W2` 实现世界格移动、132 倍时间、坐标确定性局部荒野、道路连接和从局部地图
  边缘切换邻格。
- `W3` 从 `worldTick` 派生昼夜、地表光照、荒野生态资格和原版伏击流程。
- `W4` 完成前哨站与 Warrens 的正式地点闭环：世界地图不能直接进入地牢，
  必须先返回局部地图再使用入口；返回地表恢复原位置和楼层状态。
- `W5` 增加世界自动旅行、特殊荒野房间、坐骑与环境通行/伤害、宠物和召回确认。
  城镇传送的底层条件已经明确为“已访问的正式城镇”，但尚未暴露旅店传送服务。

对应说明见 `design/wilderness-w0-authoritative-data.md` 至
`design/wilderness-w5-original-extensions.md`。

## 4. L0–L2 当前成果

### 4.1 L0：阿南巴与兽人洞穴权威选择

`packs/rfb-demo-original/legacy-wilderness-selection.json` 已升级到 schema 3，
并由 `rfb-legacy-import sync-demo-wilderness` 校验以下事实：

- 阿南巴：原版城镇索引 5，坐标 `(26,39)`；
- 兽人洞穴：原版地牢索引 3，坐标 `(30,45)`，深度 15–32；
- 兽人洞穴使用 `MONSTER_DIV_16`；偏好
  `ORC | R_CHAR_oOTC | ANIMAL | TROLL`；
- 生成标记为
  `CAVE | WATER_RIVER | CAVERN | LAKE_TREE | DESTROY | BIG`；
- 最终守卫是索引 1185 的“半兽人之王奥斯罗德”，等级 32；
- 最终物品为 `(tval 45, sval 0)`，最终 ego 索引为 206。

阿南巴已经从计划位置升级为正式 `wilderness.locations` 城镇。兽人洞穴仍是
planned dungeon，不会投影、进入或生成占位楼层。详细差距见
`design/location-l0-anambar-orc-cave.md`。

### 4.2 L1：多城镇运行时

多城镇没有增加第二套注册表：

- `WorldDefinition.townId` 仍只表示出生城镇；
- 正式城镇集合从 `world.wilderness.locations` 的 town 记录派生；
- `town_at_wilderness_position`、`town_for_floor` 和 `current_town` 统一解析当前城镇；
- 商店、Home、任务服务、城镇 DTO、入口检查和商店维护都读取当前楼层所属城镇；
- `FloorLifecycle::Town` 表示正式城镇楼层；每个城镇楼层继续存入现有
  `storedFloors`，没有第二套楼层状态；
- 从任意正式城镇进入世界地图后，在其他正式城镇坐标离开世界地图会进入或恢复
  该城镇楼层；离开城镇时保存完整楼层状态；
- 城镇、Home 和商店状态采用稀疏初始化。未访问城镇不创建状态，商店第一次走到
  入口时才生成库存，因此阿南巴不会提前消耗出生 RNG；
- 存档恢复和运行时验证只接受正式、已访问的城镇及其已初始化商店状态。

主要实现位于：

- `crates/rfb-core/src/game/town.rs`
- `crates/rfb-core/src/game/wilderness.rs`
- `crates/rfb-core/src/game/persistence.rs`
- `crates/rfb-core/src/game/validation.rs`
- `crates/rfb-content/src/definitions/worlds.rs`
- `crates/rfb-content/src/validation/worlds.rs`

### 4.3 L2：阿南巴正式城镇

已增加：

- `demo.town.anambar`
- `demo.floor.anambar`
- 阿南巴固定 `23 x 11` 紧凑地图、东西道路、东侧城门和十个设施入口；
- 八类正式商店：杂货店、护甲店、武器店、神殿、炼金店、魔法店、黑市、书店；
- 旅店和店主奥托；
- 中英文城镇、楼层、商店、店主和入口文本。

八类商店只复用当前已经有完整使用行为的前哨站货品。旅店目前复用现有商店交易
入口，只出售食物和饮料，因此不是死门。住宿、传闻、城镇传送和声望询问尚未实现。
赌场、银行、警察局和任务建筑没有运行时支持，因此没有画出入口。

### 4.4 跨城镇共享 Home

`TownFacilityDefinition` 新增可选 `storageId`：

- 前哨站 Home 的 `storageId` 指向自身，是唯一权威仓库；
- 阿南巴 Home 的 `storageId` 指向前哨站 Home；
- `home_states`、Home 中物品的 location、保存数据和状态验证都使用规范化后的
  storage ID；
- 两个 Home 不复制、不镜像、不进行双写同步。

内容校验要求每个 Home 都声明 `storageId`，且目标必须是一个指向自身的正式
Home；非 Home 设施不能声明该字段。现有 `HomeStateSaveDto.facilityId` 继续保存规范
storage ID，因此无需修改协议结构。

## 5. 当前权威模型

| 概念 | 唯一来源 | 不要另建 |
| --- | --- | --- |
| 正式城镇集合 | `wilderness.locations` | town registry |
| 出生城镇 | `WorldDefinition.townId` | 当前城镇状态 |
| 当前城镇 | 当前 `floorId` 对应的 `TownDefinition.floorId` | 可变 `currentTownId` |
| 城镇地图 | `WorldDefinition.floors` 与 `storedFloors` | town floor cache |
| 商店定义 | `TownDefinition.shopIds` → `ShopDefinition` | 城镇专用商店引擎 |
| 商店状态 | `shop_states[shopId]`，首次进入时创建 | 预生成全世界库存 |
| Home 库存 | `home_states[storageId]` | 每城库存或同步层 |
| 已访问城镇 | 稀疏 `town_states[townId]` | 全量初始化列表 |
| 世界时间 | `worldTick` | 城镇时钟 |

## 6. 当前可玩边界

- 可以从前哨站进入世界地图，手动或自动旅行到 `(26,39)`，再进入阿南巴。
- 阿南巴楼层会独立保存，来回旅行后恢复。
- 阿南巴全部八类商店可买卖，库存第一次进入时生成并独立维护。
- 阿南巴旅店可以买食物和饮料。
- 可以在任一城镇的 Home 存取同一批物品。
- 阿南巴没有任务服务；前哨站任务服务继续只属于前哨站。
- 尚不能通过旅店住宿、听传闻、查询声望或传送城镇。
- 尚不能进入兽人洞穴；世界地图也不会把 planned dungeon 当作正式地点。

## 7. 后续推进计划

### L2.1：集成收口（下一步）

1. 将当前 `L0`–`L2` 修改作为一个完整提交提交到 `codex/town`。
2. 由集成工作树合并；不要拆开多城镇运行时、阿南巴内容和共享 Home，否则中间提交
   不是可验证状态。
3. 合并其他方向后重新生成一次最终内容锁，并运行受共享初始化影响的契约验证。
4. 独立桌面分发构建仍由主工作树负责。

### L3：阿南巴旅店的最小完整服务

按可复用性分三个小纵切，不预建通用 building action DSL：

进展：住宿一晚已由 contract-v251 完成；下一项是城镇传送。

1. **住宿一晚（已完成）**：费用、拒绝条件、时间边界、恢复和设备充能均按
   `src/bldg.c` 实现；旅店入口使用窄命令 `StayAtInn`，不扩建通用 building action DSL。
2. **城镇传送**：只列出 `town_states` 中已访问的正式城镇；扣除原版费用后原子切换
   `wildernessPosition` 和目标城镇楼层。未访问城镇不投影，也不制造目的地占位。
3. **传闻与声望**：只有在权威传闻数据、声望状态和实际调用方存在时才实现；在此之前
   保持延期，不用固定文本或固定数值伪装支持。

住宿和城镇传送应分别验证时间、费用、取消、存档往返和不同 Home/商店状态不受影响。

### L4：兽人洞穴内容前置

这一步适合由怪物和物品工作树分别完成，town 工作树只消费最终内容 ID：

1. 导入并验证兽人洞穴 21–32 级候选怪物，补齐实际阻塞能力；
2. 导入守卫 1185“半兽人之王奥斯罗德”及其完整行为；
3. 审计 15–32 层通用掉落；导入最终物品 `(45,0)` 和 ego 206；
4. 更新 `legacy-wilderness-selection.json` 中对应 gap，只有完整运行时存在后才能从
   deferred/audit-required 移除。

不要为了激活地牢而用低层怪物、通用宝箱或原创守卫替代原版缺口。

### L5：兽人洞穴正式地点闭环

内容前置完成后再激活：

1. 新增 `demo.dungeon.orc-cave`、深度 15–32 的楼层定义和专用分配表；
2. 实现原版洞穴、水道、洞窟、树林湖泊、毁坏区和大型地图组合；
3. 将 `(30,45)` 从 planned dungeon 移入正式 `wilderness.locations`；
4. 在该坐标的局部荒野生成正式入口，保持“先离开世界地图，再走到入口”的 W4 规则；
5. 最终层接入奥斯罗德、最终奖励、胜利状态和返回地表位置；
6. 聚焦验证入口位置、层间连接、召回/返回、守卫唯一性、奖励只结算一次和存档往返。

### L6：第三座城镇（长期）

在阿南巴旅店和兽人洞穴路线稳定前，不新增第三座城镇。之后每座城镇继续采用同一
纵切：权威选择与差距清单 → 现有运行时可用的固定地图和设施 → 共享 Home → 邻近
地牢或任务路线。不要先实现 `t_*.txt` 通用条件地图解释器；只有多个正式城镇证明
固定同步无法维护时再评估。

## 8. 共享文件与并行开发注意事项

以下文件是城镇、怪物、物品和发布方向的高冲突点：

- `crates/rfb-legacy-import/src/content.rs`
- `packs/rfb-demo-original/worlds/warrens-journey.json`
- `packs/rfb-demo-original/legacy-wilderness-selection.json`
- `packs/rfb-demo-original/pack.json`
- `packs/rfb-demo-original/content.lock.json`
- `crates/rfb-core/src/game/persistence.rs`
- `crates/rfb-core/src/game/validation.rs`
- generated Schema 与 exact fixtures

怪物/物品工作树应只交付内容定义、能力及聚焦测试，并在交接中列出提供给兽人洞穴
的最终 ID。town 工作树负责地点、楼层、入口和城镇服务。协议版本、state-hash Schema、
最终内容锁和受影响 baseline 应在集成工作树统一收口。

## 9. 已完成验证

当前工作树已通过：

```text
cargo test -p rfb-core game::tests::town:: --lib          25 passed
cargo test -p rfb-core game::tests::persistence:: --lib   7 passed
cargo test -p rfb-content --lib                           90 passed
cargo test -p rfb-legacy-import --lib                     54 passed
cargo test -p rfb-localization --lib                      6 passed
cargo run -p rfb-content --features schemas --bin generate-content-schemas -- --check
cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original
cargo run -p rfb-contract -- verify-all tests/fixtures/active/baseline-policy.json
# 471 fixtures verified
cargo check --workspace
cargo fmt --all -- --check
git diff --check
```

尚未执行独立桌面构建；这是此前明确留给主工作树的发布动作。
