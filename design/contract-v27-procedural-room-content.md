# Contract v27：程序化房间怪物与地面掉落分配

状态：协议 1.27 / contract-v27 active baseline

## 已完成边界

- `ProceduralFloorDefinition` 新增正整数 `depth`、按稳定实例 ID 排序的 `actorSpawns` 和按稳定来源 ID 排序的 `lootSpawns`。当前双房间布局暴露 `entry` 与 `remote` 两个稳定房间来源。
- 怪物生成项声明房间、稳定 actor 实例 ID 和候选种类。核心只从 `actor.level <= floor.depth` 的候选中抽取，候选 ID 在内容编译时规范化排序。
- 地面掉落生成项声明房间、稳定来源 ID 和 `lootTableId`。`LootContext` 现携带楼层 ID、深度与来源类型；当前房间来源包含 room/spawn ID，死亡和出生携带来源包含 actor ID。
- 首次生成顺序固定为：四次布局抽取 → 每个怪物的种类与位置 → 按 actor ID 生成携带物 → 每个地面掉落的位置 → 每个 loot roll 的物品、品质、词条三次抽取。位置从房间内按行优先排列的空格集合中抽取，已经被玩家、怪物或前一个掉落占用的格子会被排除。
- 当前内容在 `remote` 房间生成一个怪物和一份地面掉落。固定种子 27 产生 `demo.monster.echo-depth-1.1` 回声猎犬与 2 个发光碎片；包括入口层怪物携带物在内，总 RNG draw counter 在首次下楼后为 13。
- 离层后怪物实例、当前生命/能量/状态、位置、地面物品实例和探索记忆继续由 `FloorState` 保存。返回并再次进入不会重新选择怪物、位置或 loot，也不会重复分配实例 ID。

## 协议、内容、存档与基线

- 协议升至 1.27，用于拒绝旧客户端/回放按 1.26 解释新的首次楼层生成 RNG 与实体集合；本切片没有新增传输 DTO。
- 内容包升至 1.22.0，content hash 为 `51ffdccfe19a9f159adc15c2f62965ff4a5d44b55990eb9f29df96870937a043`，loot table 数量增至 3。
- save schema 继续为 v1，state hash 继续为 Schema v14；怪物、地面物品、实例分配器、RNG 与离层仓库已经被现有投影完整覆盖。
- v26 存档继续可读。已经访问过程序化层的旧存档不会补生成新怪物或物品；尚未生成该层的旧存档会在首次进入时使用 v27 规则。
- contract-v27 从 v26 迁移 63 个 exact fixtures，并新增“首次生成、返回入口层、再次进入同一层、存档往返”场景，共 64 个。

## 与原版 RFB v1.3.0.7 对比

原版 `src/generate.c` 的 `cave_gen()` 在布局、门、楼梯和玩家位置完成后，依次调用 `_cave_gen_monsters()`、`_cave_gen_traps()` 与 `_cave_gen_objects()`。怪物数量受地牢定义、深度和地图面积影响，`alloc_monster()` 在合法空格中随机尝试并使用 `monster_level` 选择种类；地面物品通过 `alloc_object()` 区分房间/走廊来源，并以 `object_level`、`base_level` 和深度生成。模板房间与 vault 还会在房间生成阶段自行放置实体。

相同点：

- 布局完成后才在合法、未占用、可行走格分配普通怪物和地面物品；
- 楼层深度参与怪物/物品内容选择边界，房间来源可以影响掉落分配；
- 怪物和物品位置使用权威 RNG，生成结果随楼层保存，返回时不重建为另一批实例；
- 玩家、怪物和物品不会在初始分配时占用同一格。

主动差异：

- 重构使用内容声明的稳定 actor/source ID、显式 `depth`/room `LootContext` 和固定排序；原版主要依赖 `dun_level`、`monster_level`、`object_level`、全局分配表和数组索引。
- 当前每层只声明一个怪物生成项和一个房间掉落项；原版按密度批量生成怪物、房间物品、全图物品、金币、食物、光源、回城资源、陷阱、碎石、守护者以及模板房间专属内容。
- 重构从有限房间候选格中一次抽取，RNG 消耗数量可精确测试；原版位置分配使用带上限的随机重试，失败还可能使整个 `generate_cave()` 重新生成楼层。
- 当前深度只过滤候选怪物等级，掉落表仍由内容显式指定；原版支持越级怪物、群体、睡眠、地牢主题、房间/vault 规则和随深度变化的物品生成等级。
- fixtures 约束重构的跨平台 RNG、实例 ID、存档和 state hash，不复现原版的怪物数量、物品表或随机序列。

本切片复刻的是“布局后按深度和空间来源分配实体，并把结果固化为楼层状态”的规则关系，不移植原版全局分配器与生成数据。

## 下一步

首个可变地形状态已由 [contract-v28](contract-v28-door-terrain-state.md) 建立：内容定义互反门地形转换，核心和前端支持方向性开门/关门，碰撞、视线、存档与 state hash 同步更新。锁与破门的最小检定闭环随后由 [contract-v29](contract-v29-locked-door-checks.md) 建立；陷阱、搜索、解除、挖掘和多深度连接继续拆分。
