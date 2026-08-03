<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Contract v162: Outpost supply court

状态：active baseline；Gate 6 后的首批商店扩展完成。协议为 `1.129`，save 容器保持 v1，state hash Schema 保持 `60`。demo 内容包为 `1.152.0`，content hash 为 `8481a945e6d627244cf7ad1b8af4f77ef0ef2013baa2a1b360ef2821527f1433`；active baseline 包含 462 条 exact fixtures、零 waiver。

## 共享建筑与独立商店

- Outpost 的杂货店、圣殿和炼金店各占 `4x3`，横向相接为一座 `12x3` 补给院；地图 terrain 表达连续外墙，不新增没有运行时职责的建筑实体。
- 三店入口分别为杂货店 `(16,8)`、圣殿 `(20,8)`、炼金店 `(24,8)`，使用原版可辨认的 `1`、`4`、`5` glyph。每个入口只激活对应的商店页面。
- 三店各有稳定 ID、类别、店主、库存、维护时间和访问状态。返回 Outpost、Warrens 刷新和存档回环不合并或清除这些状态。

## RFB 内容边界

- 固定参考仍为 RFB commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 的 `src/shop.c` 与 `lib/edit/k_info.txt`。圣殿负责治疗、勇气、召回与解诅；炼金店负责其余首批药水和卷轴。
- 圣殿首批严格库存为 Light Healing `20`、Valor/Heroism `35`、Homeward/Word of Recall `150`、Cleansing/Remove Curse `100`。
- 炼金店首批严格库存为 Flicker/Phase Door `15`、Farstep/Teleportation `40`、Seeking/Object Detection `8`、Trapfinding/Trap Detection `8`、Temperate/Resist Heat and Cold `120`。
- Temple 与 Alchemist 的 owner greed/purse cap 采用原版对应 Human 档的 `109/30000` 与 `111/10000`。当前 Warrior 的价格因子分别为 `101` 与 `103`。
- 原版这三类普通商店最终共享 `_will_buy`。重写版因此继续收购所有正价值且非 corpse/remains 的背包物品；商店库存直接显示已知物品名，但只在购买后写入玩家的 aware/identified 状态。交易的原子性、数量、负重、识别、相容堆叠和维护规则复用 contract-v161。

## 确定性与验收

- 新游戏按规范化 shop ID 顺序生成三家库存，seed 42 的出生库存 RNG draw 从 9 增至 18；这属于新增持久状态，不更改 state hash 结构版本。
- 项目仍在开发期，本阶段只按新存档测试，不为 1.151.0 开发存档增加商店状态迁移或旧 content hash 白名单。
- fixtures 455、457、458、460、461 只刷新新增商店导致的实际状态；fixture 461 仍验证杂货店 `3/2` 口粮交易、余额 342 和 `worldTick = 190`。fixture 462 新增圣殿与炼金店独立访问及 save round-trip。
- 内容测试覆盖三入口连续建筑和类别严格库存；核心测试覆盖三店投影、独立状态、圣殿购买、炼金店访问与存档；桌面 supply-loop E2E 核对三家标题、店主和库存后继续完整补给闭环。
