# Contract v62：区域组合生成

## 目标

contract-v60 建立了同层区域，但首版要求区域楼层与 Vault、动态群体、terrain feature、pit、分阶段地貌、守护者和显式连接隔离。contract-v62 解除这些阶段隔离，让区域继续负责局部生态，同时复用 v50–v59 已有的特殊生成管线。

## 内容契约

- 区域楼层仍不能同时声明楼层级 `encounterTableId/lootTableId`、显式 `themeId/vaultId`、内联 actor/loot spawn、nest 或 maze-only；`themeTableId`、`terrainFeatureTableId`、显式 connections、guardian/final floor 和普通 room layout 的特殊阶段现在可以组合。
- 启用 `groupPlacements/groupActorSlots` 时，每个深度合格区域 encounter 表都必须同时提供普通候选与动态群体候选；未启用群体预算时，合格候选仍只能是普通遭遇。
- actor 总预算先预留 pit、guardian 与 Vault 固定成员，再支付动态群体，余量至少为每个区域保留一个普通 actor。loot 总预算扣除 Vault 固定 loot 后，至少为每个区域保留一个普通掉落。
- pit 独占最后一个 room placement；`regionPlacements` 不得超过剩余普通房间数。区域 + Vault/pit 的候选数量、面积和 actor/loot 预算继续执行原有上限。

## 生成语义

1. 先生成普通房间并选择区域；pit 专用房间不参加普通区域房间分配。区域房间使用局部主题 terrain，走廊继续使用全层基础 terrain。
2. 全层 theme/Vault、地貌、连接与 feature 沿既有确定性阶段执行。room feature 同时接受所有已选区域的 floor terrain，corridor feature 仍只接受全层基础 terrain。
3. 每个 Vault 或 pit 的完整 footprint 从其他区域转移给距离入口最近的宿主区域；同距按区域顺序决胜。固定 Vault 成员与 loot、guardian、feature 和 pit footprint 在普通内容选位前占位。
4. 区域普通 actor/loot 在持久 region cells 的当前可行走空格中联合分配：先为每个区域各保留一个 actor 与 loot，再按区域顺序分配余量，避免特殊结构或随机房间形状耗尽单一区域。
5. 区域动态群体放入第一个已选区域，使用该区域 encounter 表和稳定 region 前缀；普通 actor 使用独立 `encounter.plain` ID 空间。怪物寻路不能离开其当前区域，跨区域群体协作继续延后。
6. pit roster 成为独立阶段，不再依赖楼层级 encounter 表，因此区域最终层仍能生成完整 5x5 pit 阵列和 guardian。

## Demo 覆盖

- echo depth 2：区域 + 全层 theme/Vault + 四条显式连接；
- resonance depth 6/7：区域 + dynamic friends/escort pack + terrain feature；
- resonance depth 8：区域 + 两个 spatial Vault + terrain feature；
- resonance depth 10：区域 + cavern/lake/river/destroyed/streamer + pit + guardian + terrain feature。

## 持久状态与兼容

- Vault/pit footprint、局部表引用和区域边界继续写入现有 `FloorRegionSaveDto`；save 容器仍为 v1，state hash 仍为 Schema v23，没有新增字段。
- v61 及更早内容 hash 继续按历史迁移列表载入；已保存楼层不补生成区域组合、不重绘特殊阶段，也不推进 RNG。
- 区域集合保持规范排序、互斥且覆盖区域楼层中的 actor 与地面/携带 loot；区域怪物移动受边界限制，因此行动后的存档仍满足该不变量。

## 验证

- 内容测试覆盖组合合法性、深度候选、缺失 group 配对预算、pit 房间预留，以及特殊 actor 后普通区域预算不足。
- Core 测试覆盖多 seed 区域回环、局部动态 pack/feature、legacy 与 spatial Vault、pit footprint、完整晚期地貌、guardian、确定性和 v59–v61 存档兼容。
- contract-v62 从 v61 刷新 121 个 fixture，并新增 4 个不同 seed 的区域组合命令流；active baseline 为 125 个 exact fixtures、0 waiver。

## 版本

- 协议：1.62；
- 内容包：1.55.0；
- content hash：`9d25687c1296bc6f9953024bd76bb9eefc4c1e3955280b96d34d565ff7ca289d`；
- save：v1；
- state hash：Schema v23；
- active baseline：contract-v62，125 exact fixtures。

## 明确延后

- 任意多边形/噪声区域边界、走廊区域归属、区域专属门与跨区域群体协作；
- 多个 pit、独立 nest 房间、任意 formation 模板，以及召唤/繁殖/种群上限；
- Vault 多入口、大模板连通性证明、跨走廊拼接和更一般的分支连接图。
