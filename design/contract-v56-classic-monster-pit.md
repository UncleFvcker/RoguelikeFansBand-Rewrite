# Contract v56：原版式怪物 Pit 与等级阵列

状态：已实现

## 原版参照与收敛边界

FrogComposband 的 `lib/edit/rooms.txt::Monster Pit I` 使用独立 25×11 复合房间：外墙内保留环廊，第二层墙只开一个入口，19×5 内室由数字 formation 填满。`src/rooms.c::_init_formation()` 从主题过滤后的怪物池抽取十个候选，按等级从强到弱排序，模板中央使用较强阶位、外围使用较弱阶位。contract-v56 保留独立复合房间、单入口、主题池、密集填充和中心强化语义，但将隐式全局 hook、固定 25×11 尺寸和失败重试收敛为内容定义、显式预算与稳定候选顺序。

## 内容契约

- `layout.pit` 声明独立 `encounterTableId`、奇数 `innerWidth/innerHeight` 与 `rosterSize`；demo 使用 5×5 内室和五档 roster。
- `generationBudget.pitPlacements/pitActorSlots` 必须成对出现；v56 只接受一个 pit，actor slots 必须严格等于内室面积。
- 编译器验证 encounter table 引用、当前深度至少两个合法候选、尺寸/奇偶、地图容纳、总 actor 预算，以及与 legacy nest、动态 group、空间 Vault 的互斥关系。
- demo 新增 `demo.encounter-table.resonance-pit`，包含六个分深度、加权候选；内容包升级到 1.49.0。

## 确定性生成

深度 9 将最后一个 room placement 槽替换为 11×11 复合结构：外墙、可行走环廊、单门内墙和 5×5 密集内室。生成器从 pit table 按权重抽取五个 roster 项，再按 actor level 降序、稳定 ID 升序排序；每个内室格按相对中心的归一化 Chebyshev 阶位选种，因此中央不会弱于外围。整个 footprint 在普通 encounter、loot 与 terrain feature 落位前被保留，走廊只连接外环入口。

## 兼容性与验证

- 协议：1.56；内容包：1.49.0；content hash：`461242cb2164434a7ef44a3692f1c9fa4ffe9921f07c17e0857c96f2f2d95041`。
- save v1 / state-hash Schema v19 不变；v55 已生成楼层不会补建 pit，也不会推进 RNG。
- Core 共 100 项测试；`contract-v56` 从 v55 迁移 110 个 exact fixtures，并新增两个 pit seed，共 112 个 exact fixtures、0 waiver。
- 当前内容计数：terrain 42、actor 10、encounter table 4、loot table 5、theme table 2、terrain feature table 1、vault 5。

## 明确遗留

- nest 的独立复合房间变体、任意模板 formation、多个 pit 同层和成功落位失败回退；
- pack AI、召唤、繁殖、种群上限与 unique 过滤；
- 多入口、分支/shaft、同层多区域主题和完全替代普通房间的专用楼层模式。
