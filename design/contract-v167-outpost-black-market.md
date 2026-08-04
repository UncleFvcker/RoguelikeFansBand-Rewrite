<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->

# Contract v167: Outpost Black Market

状态：active baseline；Outpost Black Market 完成。协议为 `1.134`，save 容器保持 v1，state hash Schema 保持 `61`。demo 内容包为 `1.157.0`，content hash 为 `796b48c16924b0d89a5c98443122b3802b920e9ef460aab447e0468a3f99d7ea`；active baseline 包含 467 条 exact fixtures、零 waiver。

## 原版建筑审计

- 固定参考为 RFB commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 的 `lib/edit/t_pref.txt`、`lib/edit/t_outp.txt`、`lib/edit/k_info.txt`、`src/shop.h` 与 `src/shop.c`。
- 原版 Outpost 地图实际使用 `0-9`、`M`、`a`、`b`、`n` 共 14 类常驻或轮换服务入口。contract-v167 完成数字 `7` Black Market 后，重写版已覆盖数字 `1-9` 的九类基础入口；地图仍缺 `0` Shroomery、Museum、`a/b` 两条轮换任务服务线以及 `n` Bounty Office，共五类。地图中的 `o/p` 被 `t_outp.txt` 局部重定义为任务入口，不能按 `t_pref.txt` 的全局 BUILDING 槽重复计数。
- `SHOP_JEWELER` 与 `SHOP_DRAGON` 存在于原版 shop 系统，但没有出现在固定提交的 Outpost 地图中；它们是全局商店类型缺口，不计入上述五个 Outpost 地图入口。`BUILDING_0..31` 是服务槽，不是 32 座独立建筑。
- 黑市位于 `(40,14)`，使用原版数字 glyph `7`。建筑占地 `4x3`，除入口外全部为不可行走墙体；terrain 继续禁止 ASCII 大小写字母，所有字母保留给 actor。

## 商店边界

- 稳定店主采用原版 Human `公平的托皮(?)`：greed `150`、单件收购上限 `30000`。Warrior 没有 Burglary realm 或 Black Marketeer mutation，因此买价在普通报价后乘二，卖价在普通收购价后减半。
- 首批库存只采用原版 Black Channels 与 Necronomicon，基础价值分别为 `15000` 与 `100000`。两本书在固定 `k_info.txt` 中均不带 `TOWN`，明确区别于 Bookstore 的普通城镇库存；商店物品继续按原版视为已知。
- seed 42 的 Warrior 价格因子为 `140%`，Black Channels 买价为 `42000`，Necronomicon 买价为 `280000`。价格、钱包、负重、数量、相容堆叠、维护和交易原子性复用既有权威商店管线。
- 正义德行、Burglary realm、Black Marketeer mutation、手动付费刷新和随机高阶物品生成尚无所需领域状态，本契约不伪造这些规则。

## 验收

- fixture 467 以 1000000 金币的 Warrior 从出生点进入黑市，按 `42000` 购买一本 Black Channels，并锁定最终金币 `958000`、完整识别、库存转移和 save round-trip hash。
- 核心测试覆盖黑市买入加倍、卖出减半和 `30000` 单件上限；内容测试锁定两本非 `TOWN` 法书、店主参数、入口位置和严格类别。
- 桌面 supply-loop E2E 在进入 Warrens 前访问黑市，断言标题、稳定店主及两行聚合库存；原有购物、地牢消耗、拾取金币、回城补给和 Home 存档恢复流程保持完整。
- 当前仍在开发期，不为旧开发存档新增兼容迁移；测试始终从新存档开始。
