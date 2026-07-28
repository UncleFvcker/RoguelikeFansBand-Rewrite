# Contract v116：装备诅咒与解除卷轴

状态：已实现

Contract v116 接入原版卷轴 sval 2/3/14/15：Curse Armor、Curse Weapon、Remove Curse 与 *Remove Curse*。协议为 `1.116`，demo 内容包为 `1.107.0`，state hash Schema 为 `52`，active baseline 包含 413 条 exact fixtures、零 waiver。内置内容 hash 为 `9d1c6c1e01fb4533aa5a9868f0adfcbe876148d98585412783d0da93f4019dff`。

## 1. 内容模型

物品效果新增 self-only 的 `curse-equipped-item { target }` 与 `remove-equipped-curses { includeHeavy }`。施咒目标只允许 `weapon` 或 `armor`；解除效果以 `includeHeavy` 区分普通与强力卷轴。装备定义可用 `initialCurse` 盖章 `normal`、`heavy` 或 `permanent`，但没有装备槽的物品不得声明初始诅咒。

`initialCurse` 只在实例生成时读取。实例进入存档后，当前诅咒状态是唯一权威来源，不因内容包更新重新派生。demo 新增四种卷轴、普通可施咒武器、初始 heavy 护甲和 initial permanent 护符，用于覆盖三档状态与神器抵抗。

## 2. 施咒选择与神器保护

Curse Weapon/Armor 只在当前装备中选择带对应 tag 的实例。候选先按槽 ID、再按实例 ID 稳定排序；只有多候选时才作一次有界选择。没有合法目标时卷轴仍被消耗并推进正常行动时间，但不抽施咒 RNG，种类知识只从 `Unknown` 进入 `Tried`。

找到目标后，普通物品必定获得 `normal` 诅咒；已有 heavy/permanent 状态不会被降级。带 `artifact` tag 的目标在落地前有 50% 抵抗检定。无论诅咒落地还是神器抵抗，来源卷轴都会变为 aware。结构化 `ItemCurseResolutionDto` 返回目标实例/种类、before/after 和 resisted；contract 的 `debugItemCursesLand` 与 `debugItemCursesResisted` 只用于固定这道概率门，二者互斥。

## 3. 解除与装备事务

普通 Remove Curse 解除所有已装备实例的 normal 诅咒，跳过 heavy/permanent；只有实际解除至少一件时才变为 aware。强力 *Remove Curse* 解除 normal/heavy，永久诅咒始终保留；强力卷轴即使没有可解除目标也变为 aware。两种卷轴都正常消费，结果以稳定实例 ID 列表返回已解除物品和保留的永久诅咒物品。

任意严重度的诅咒装备都不能卸下，也不能通过装备同槽替代品绕过。`Unequip` 和触发替换的 `Equip` 在消费行动、推进 world tick 或抽取 RNG 前返回 `item.unequip.cursed`；装备栏和资源派生保持不变。

## 4. 实例、存档与 Web

地面、背包、装备和怪物携带四类实例存档都增加可选 `curse`。旧档缺字段确定性迁移为无诅咒，不读取内容表、不补抽 RNG。拆分保留状态；堆叠只合并诅咒及其他运行时属性均兼容的实例。新增权威字段使 state hash 升至 Schema v52，save 容器仍为 v1。

协议新增 `ItemCurseSeverityDto`、`ItemCurseResolutionDto`、`ItemCurseRemovalResolutionDto` 和两种 outcome。Web 在背包、装备及地面投影严重度，并格式化施咒落地、神器抵抗、无目标、解除成功/无效果和诅咒卸装拒绝事件；显示知识继续不入档。

## 5. 旧版映射与明确差异

legacy importer 将 sval 2/3 映射为武器/护甲施咒，将 sval 14/15 映射为普通/强力解除。固定旧版提交的真实导入结果为 937 items、128 affixes、1260 abilities、4 ability books，`scroll-effect` 从 42 降至 38；严格源校验、二进制编译和产物回读 hash 均为 `b517b3dc48395c91b3c9864028cce2f4ae5f97d94dc41264c1afe1ac9af9fb70`。

原版施咒路径还会通过 `blast_object` 抹除 ego/artifact、基础骰和加值。当前物品 kind ID 与基础定义不可变，P66 只实现实例诅咒与神器抵抗；物品损坏、去词条和负强化留给独立事务，不通过改写 kind ID 近似。

## 6. Fixtures 与验证

`contract-v116` 从 v115 原样迁移 405 条历史场景，并新增 406–413：

- 武器/护甲诅咒落地与神器抵抗；
- 无匹配装备的消费、零 RNG 和 Tried 知识；
- 普通解除跳过 heavy，强力解除 heavy 并保留 permanent；
- 诅咒装备卸下/替换的零时间拒绝；
- 三档诅咒、四类物品位置和旧档缺字段回读兼容。

验证至少包括 workspace test/check/clippy、Schema 与 TypeScript bindings、demo/真实包源编译和二进制回读、413 条 exact fixtures、基线策略、回放、Web test/typecheck/build 与 Windows Tauri E2E。
