<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Contract v164: Outpost Armoury and Weaponsmith

状态：active baseline；Outpost 首批护甲店与武器店完成。协议为 `1.131`，save 容器保持 v1，state hash Schema 保持 `60`。demo 内容包为 `1.154.0`，content hash 为 `dcf62b45fb72e47b8190bb98b8d59db534f5ad53cc9ec47ac918effb2e22d52c`；active baseline 包含 464 条 exact fixtures、零 waiver。

## 原版建筑边界

- 固定参考为 RFB commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`。基础城镇入口共有九类，v163 已完成 General Store、Temple、Alchemist、Magic Shop；本契约增加 Armoury 与 Weapon Smiths。基础入口仍剩 Black Market、Home、Bookstore。
- 新版 `shop.c` 另定义 Jeweler、Shroomery、Dragon 三类正式商店；这些与 Home 共计后续还有六类商店或仓储设施。`BUILDING_0..31` 是服务槽，不代表 32 座独立商店。
- 护甲店与武器店共享城内西南侧一栋工坊，入口分别为 `(15,14)` 与 `(19,14)`，使用原版数字 glyph `2/3`。墙体继续使用 `▓`，所有 terrain glyph 仍禁止 ASCII 大小写字母。

## 商品与店主

- 护甲店首批为 Leather Gloves、Soft Leather Boots、Hard Leather Cap、Small Leather Shield、Chain Mail，原版基础价值为 `3/7/12/15/750`。
- 武器店首批为 Spear、Sabre、Short Sword、Broad Sword、Short Bow、Arrow，原版基础价值为 `36/50/80/255/400/1`。
- 只选择项目中已有完整穿戴、近战或射击行为的物品，不增加空交互商品。两店库存持久、按既有时间维护；兼容的箭矢实例即使跨越单栈上限，UI 仍聚合为一个条目。
- 护甲店使用 Human 店主冷酷的达格罗，钱包上限 `20000`、greed `111`；武器店使用 Human 店主屠兽者阿恩达尔，钱包上限 `20000`、greed `110`。原版两店的 `will_buy` 都复用普通商店通用边界，因此本契约不自创按装备类别限制出售的规则。

## 确定性与验收

- 新游戏按规范化 shop ID 初始化六家库存；新增店铺改变开发期新存档的库存实例序号与 RNG 顺序。项目仍处于开发期，不为 `1.153.0` 内容 hash 增加旧存档兼容。
- fixture 464 覆盖从出生点进入护甲店、购买并装备 Leather Gloves，再访问共享建筑的武器店入口。核心测试另覆盖六店投影、原版库存集合、箭矢跨栈聚合和存档回环。
