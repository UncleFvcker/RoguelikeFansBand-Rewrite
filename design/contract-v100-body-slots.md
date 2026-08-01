# Contract v100：身体/槽位模板（双戒指/光源槽/槽实例化）

状态：已实现。协议 1.100 / 内容包 1.91.0（hash `1380958f…`）/ state hash Schema v41 / 320 个 exact fixtures、零 waiver。

> Historical engine-default baseline. Contract v151 leaves `STANDARD_BODY_SLOTS` unchanged for old content but gives production `demo.race.rfb-human` an explicit 13-slot RFB Standard approximation, including a quiver and excluding the original-demo charm.

## 1. 动机

此前 RFB 的装备模型是"物品自声明槽字符串 + 同名槽只挂一件 + 玩家无槽位清单"——玩家隐式拥有一切槽、每种恰一个。原版（FrogComposband）以 `b_info.txt` 的 113 个身体模板按**种族/身体**绑定槽位清单（Standard 身体 14 槽：双持手 ×2、射击、箭袋、戒指 ×2、项链、光源、五件护甲），双戒指与光源槽在旧模型中无从表达（P45 光源神器的六维修正因此进了缺口报告）。

## 2. 内容层

- `RaceDefinition.bodySlots: [BodySlotDefinition]`（可选，缺省空）——`{ id, slotType }` 槽实例表：`slotType` 对齐物品的 `equipmentSlot` 声明，`id` 命名实例，因此一个身体可以有多个同类型槽（ring-1/ring-2）。空表示使用引擎标准身体。
- 校验：实例 id 与类型走装备槽同款格式校验（小写 ASCII ≤64）、实例 id 唯一、≤64 条；违规报 `InvalidBodySlots`。
- demo：elf 种族显式声明标准 13 槽（锻炼内容路径）；human/gnome 留空走默认。新增演示物品 `demo.item.resonant-band`（共鸣指环，ring 槽，defense +1）。

## 3. 标准身体（核心常量）

`STANDARD_BODY_SLOTS` 13 槽：weapon / launcher / body / head / shield / cloak / gloves / boots / **ring-1 + ring-2**（类型 ring）/ amulet / **light** / charm。单实例槽的实例 id 等于类型名，因此模板化之前的存档（如 `charm`）无需迁移即合法。与原版 Standard 的**记录差异**：双持手（WEAPON_SHIELD ×2）与箭袋（QUIVER）未纳入——RFB 近战为单武器模型、弹药不占槽，待各自系统再议。

## 4. 运行时语义

- 生成期从构筑的种族解析身体（无构筑或种族未声明→标准身体），**存档权威**：`player.bodySlots` 入档入 state hash（Schema v41）；旧档为空时按同规则零 RNG 派生。
- 装备：按物品槽**类型**在身体顺序中找**首个空实例**；全满则**顶替首个实例**的占用者（确定性、无提示，`item.equip.swap` 事件已有语义）。身体没有该类型槽→拒绝装备。起始装备走同一实例解析。
- 卸下/射击/近战按实例 id 反查类型（`body_slot_type`），武器/发射器判定不再依赖实例名字面量。
- 载入校验：被占用实例必须存在于身体且类型与物品声明一致、实例不得重复占用；DTO 层只保留"物品可装备"检查，实例↔类型匹配统一由状态校验持有。

## 5. 协议与前端

快照新增 `bodySlots: [BodySlotDto { id, slotType }]`（更新流不带——模板游戏内不变）。装备面板改为按身体模板渲染全部槽位：占用行沿用物品行为，空槽显示"空缺"；同类型多实例以序号区分（`戒指（1）/（2）`）。槽类型名 12 键双语入 Fluent（charm 沿用）。

## 6. 契约

- contract-v100 基线：318 条 v99 fixtures 迁移后**零漂移**（剥离 stateHash/saveRoundTripStateHash 后，唯一差异为协议格式扩展 bodySlots 与 protocolVersion 字符串）。
- 新场景 319（双戒指：两枚共鸣指环依序落 ring-1/ring-2，defense 1+2）与 320（顶替：第三枚触发 `item.equip.swap`，band-1 回背包、band-3 入 ring-1）。
- 核心单测 `ring_slots_fill_in_body_order_and_replace_deterministically` 锁住实例落位、顶替与存档回读。

## 7. 导入器（同轮）

tval 39 光源从无槽壳改接 `light` 槽（equipment 标签、maxStack 1——原版火把可堆叠记差异），光源神器（加拉德瑞尔水晶瓶、帕蓝提尔、知识之石等 8 件）六维修正当轮回收（帕蓝提尔 wisdom/charisma +3），对应旗标退出 `unmappedArtifactFlags`。

## 8. 遗留

- 双持（两只武器手）与箭袋槽位：待近战双持与弹药系统设计。
- 非标准身体（蛇 4 戒指、龙身体等）：内容表面已就绪（bodySlots 任意声明），怪物种族/附身玩法引入时启用。
- 光源半径/燃料语义仍为行为缺口。
