# Contract v113：卷轴地图与侦测事务

状态：已实现

Contract v113 按剩余 59 条 `scroll-effect` 的真实 sval 分布，选择覆盖最高的地图/侦测族。协议为 `1.113`，demo 内容包为 `1.104.0`，state hash Schema 保持 `49`，active baseline 包含 389 条 exact fixtures、零 waiver。内置内容 hash 为 `10d3813ec933dd881c23229b604c5f64e67716a56ebdb20b6a844c98593a7653`。

## 1. 原版审计与内容模型

固定来源 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 的 59 条未映射卷轴逐个对照 `SV_SCROLL_*`、`k_info.txt` 与 `_do_scroll()`。地图/侦测是最大可复用族，共七个独立 sval：Mapping 25、Detect Gold 26、Detect Item 27、Detect Trap 28、Detect Door/Stairs 29、Detect Invisible 30、Detect Monsters 57。

`AbilityDetectSubjectDefinition` / DTO 新增 `item` 主体；物品 `detect` 增加缺省为 false 的 `throughWalls`：

- terrain `category: map`、`persistent: true` 把半径内尚未探索的格写入既有 `explored` 地图记忆，不揭露隐藏地形真值；
- 其他 persistent terrain 侦测按 category 返回位置，只把具有 `concealedAsTerrainId` 的命中写入 `revealedTerrain`；
- actor 与 item 侦测只返回瞬时位置和稳定实例 ID，不建立跨回合实体知识；
- `throughWalls: false` 保持 contract-v83 的 FOV 过滤；原版卷轴显式使用 true，因此能探测 LOS/FOV 外但仍在 Chebyshev 半径内的目标；
- 结果统一按距离、y、x、实例 ID 排序，空结果仍是成功使用。

静态 `useAction` 首次允许 self-target detect；动态 device activation 保持原有目标、检定和充能语义。demo 新增 Cartography Scroll、Trapfinding Scroll 与 Seeking Scroll，均共享未知卷轴外观。

## 2. 事务、事件与 Web

成功阅读先消费一张卷轴，再执行零 RNG 侦测并把来源种类标为 aware。显式错误目标在消费、RNG 和 world tick 前返回 `item.use-unavailable`；省略目标或传 self 都是合法的自目标读取。

静态卷轴使用独立事件 `item.use-detected`，结果复用 `GameEventOutcomeDto::AbilityDetect`。`subject` 可为 terrain/actor/item，`detectedPositions` 与 `detectedEntityIds` 保留规范顺序。动态设备继续使用 `item.activation-detected`，不会伪造 profile ID。Web 无需新目标对话框，背包“使用”直接发送现有 use-item 命令，并增加静态侦测事件的中英文显示。

## 3. Legacy 导入

七个 sval 全部映射为 `detect`：

- 25 → terrain/map/persistent；
- 26 → item/gold；27 → item/item；
- 28 → terrain/trap/persistent；29 → terrain/passage/persistent；
- 30 → actor/invisible；57 → actor/legacy-import（任意导入怪物）。

导入器同时把 TV_GOLD 标记为 `gold`，f_info 的 DOOR/STAIRS 标记为 `passage`，r_info 的 INVISIBLE 标记为 `invisible`。独立生成的真实包包含 937 items、128 affixes、1260 abilities 和 4 ability books；`scroll-effect` 59→52。严格源校验、编译和二进制回读 hash 均为 `43b02c9e94aaa8b962d54f3e9b55cf31ab16a3c1a6573e677b2d23df32636abe`。

## 4. 存档、回放与 fixtures

Mapping 复用 save v1 的 `explored`，隐藏地形复用 `revealedTerrain`，实体侦测不新增存档字段。协议只扩展枚举与事件，state hash 的权威输入结构不变，因此 Schema 保持 49。旧 built-in 内容 hash 加入迁移白名单，旧档不补发新卷轴、不重写探索知识、不抽 RNG。

fixtures 387–389 固定：

- Cartography 在 8 格范围内写入 37 个此前未探索格，并完成存档回读；
- Trapfinding 揭示 `(10,10)` 的隐藏陷阱；该格在圆形 FOV 外，证明 through-walls 与持久知识；
- Seeking 按稳定顺序返回 5 个地面物品的位置与实例 ID，消费一张后保留同实例的剩余堆叠并完成存档回读。

## 5. 后续边界

剩余 52 条 `scroll-effect` 中，传送/回城族包含 Phase Door、Teleport、Teleport Level、Word of Recall 与 Reset Recall，覆盖五条且可复用既有位移/楼层事务，作为 P64 首选。装备附魔五条、召唤四条、解除/施加诅咒四条紧随其后。全层感知、持续 telepathy、怪物回忆、物品自动拾取、盲读失败率和地图 UI 标记不在本轮范围。
