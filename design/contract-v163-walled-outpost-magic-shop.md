<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Contract v163: Walled Outpost and Magic Shop

状态：active baseline；Outpost 城镇轮廓与首批 Magic Shop 完成。协议为 `1.130`，save 容器保持 v1，state hash Schema 保持 `60`。demo 内容包为 `1.153.0`，content hash 为 `cbcca1349df4d40a76a5de10759d3a2bffa17bfe4c71fc486389c5b21b4d525e`；active baseline 包含 463 条 exact fixtures、零 waiver。

## 城镇轮廓

- Outpost 城墙围住 `x=8..50、y=2..19`，只在 `(8,11)` 西门和 `(50,11)` 东门断开。主街横穿两门，城内保留空地供后续真实商店扩建。
- Warrens 入口移到东墙外 `(59,11)`，由道路穿过东门连接，并以岩壁围出独立洞口。出生位置为城内主街 `(29,11)`。
- 杂货店为独立建筑，入口 `(17,8)`；圣殿为主街南侧的对称独立建筑，入口 `(29,14)`；炼金店与魔法店共享东北建筑，入口分别为 `(38,8)` 与 `(42,8)`。
- 所有 terrain 都禁止使用 ASCII 大小写字母作为 glyph，字母完整保留给 actor。城防墙、普通建筑、圣殿石墙分别使用 `█`、`▓`、`▒`；城门使用 `+`，树木使用 `♣`，任务裂隙使用 `○/×`，四家商店继续使用 RFB 数字入口 `1/4/5/6`，Warrens 使用 `>`。

## RFB Magic Shop 边界

- 固定参考为 RFB commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 的 `src/shop.c`、设备定义与价值公式。Magic Shop 首批只录入原版可售且当前效果完整的 Magic Missile Wand、Detect Objects Staff、Identify Staff，不自创商品。
- 基础价值分别为 `850/1500/1500`。Human 店主 Edrin Sol 的 greed 为 `110`、钱包上限为 `20000`；当前 Warrior 购买价格因子为 `102`，seed 42 单价为 `867/1530/1530`。
- 三种设备使用既有动态设备实例语义。seed 42 的出生能量为 `17/45`、`11/45`、`16/45`；购买不重掷或补满。fixture 463 购买 Detect Objects Staff 后使用一次，能量从 `11` 降为 `7`，并验证存档回环。

## 确定性与兼容

- 新游戏按规范化 shop ID 顺序生成四家库存，seed 42 出生库存 RNG draw 为 `30`。这是新增持久状态，不改变 state hash 结构版本。
- 项目仍在开发期，只按新存档测试；旧 `1.152.0` content hash 不加入迁移白名单。
- 新内容会改变所有 exact fixture 的权威 content hash/state hash；全量刷新前后抽样确认非 Warrens 场景的命令事件、玩家状态、RNG、实体和地图结果不变。fixtures 456、460、461、462 的路线按新地图定向修正，fixture 463 覆盖四店访问、Magic Shop 交易、设备使用和存档读取。
