<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Contract v165: Outpost Bookstore

状态：active baseline；Outpost 书店完成。协议为 `1.132`，save 容器保持 v1，state hash Schema 保持 `60`。demo 内容包为 `1.155.0`，content hash 为 `3f12b3a62351b245edb8223b324c72a7bd01e3cc53f2ffb3fcd402dce5109435`；active baseline 包含 465 条 exact fixtures、零 waiver。

## 原版建筑与库存边界

- 固定参考为 RFB commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 的 `lib/edit/f_info.txt`、`lib/edit/k_info.txt` 与 `src/shop.c`。基础城镇九类入口现已完成 General Store、Armoury、Weaponsmith、Temple、Alchemist、Magic Shop、Bookstore，仍剩 Black Market 与 Home。
- 新版 `shop.c` 另有 Jeweler、Shroomery、Dragon。因而尚未实现的正式商店/仓储设施共五类：Black Market、Home、Jeweler、Shroomery、Dragon。Museum 等属于后续建筑服务；`BUILDING_0..31` 是服务槽，不是 32 家独立商店。
- 书店位于 Alchemist/Magic Shop 共享建筑中间，入口 `(40,8)`，使用原版数字 glyph `9`。墙体仍为 `▓`，terrain 的 ASCII 大小写字母计数保持为零。

## 商品、店主与交互

- 原版书店从法术书种类中取库存，但当前地表库存模式只允许带 `TOWN` 的书。本批次因此仅出售 Stench of Death 与 Sepulchral Ways，原版基础价值 `100/1000`、重量均为 3.0 磅。
- Black Channels 与 Necronomicon 没有 `TOWN`，不进入书店。项目早期的 Echo Primer 与 Stillwater Notes 不是 RFB 原版书店商品，也不拿来填充货架。
- Stench of Death 新增独立物品和能力书定义，绑定已有完整实现的死亡魔法第一册八项能力；购买实例可正常学习、持久化和回放。Sepulchral Ways 复用既有第二册完整能力。
- 店主采用原版 Human 贪婪的多拉夫，钱包上限 `10000`、greed `108`。原版 Bookstore 的 `will_buy` 复用普通合法物品边界，本契约不自创按书籍类别限制出售的规则。

## 确定性与验收

- 新增书店改变开发期新存档的商店初始化顺序、物品实例序号和 RNG 顺序；不为 `1.154.0` 开发存档增加兼容哈希。
- fixture 465 覆盖 Scholar 从出生点进入书店、购买 Stench of Death、从购买实例学习 Detect Evil，并完成存档往返。核心测试另锁死两本城镇库存、原版价值、严格类别和完整学习事务；桌面 supply-loop E2E 覆盖三入口奥术建筑。
