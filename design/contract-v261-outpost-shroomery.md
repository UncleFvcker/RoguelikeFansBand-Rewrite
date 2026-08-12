# Contract v261：Outpost 蘑菇店

状态：implemented

## 权威来源

- `D:/codex/Frogcomposband/master` 的 `master` ref，核对提交
  `efd63661302866038f58d8cd2553b23e6af3bf9d`。
- `lib/edit/t_pref.txt` 与 `lib/edit/t_outp.txt`：`0` 为 Shroomery。
- `src/shop.c`：类别中文名“蘑菇店”、Human 店主“马丁”、108 greed、10000 purse，
  库存只接受有正价值的蘑菇类食物。
- `lib/edit/k_info.txt`、`src/cmd6.c`：Fast Recovery / Wrinkled，等级 15、重量 2、
  价值 30；治疗 2d8，将流血改为 `cut / 2 - 50`，再生持续 101–200 回合。

## Rewrite 边界

- Outpost 在独立设计地图的空闲南东地块增加 `demo.terrain.shroomery-entrance`，保留
  原版可辨认的 `0` glyph，不复制原版城镇地图。
- 四种既有 `TOWN` 蘑菇从 Outpost General Store 移入 Shroomery，并新增快速恢复
  蘑菇；Anambar 没有新增该建筑，原有库存不变。
- 快速恢复以窄复合物品效果保持三次 RNG 的权威顺序：2 次治疗骰、1 次持续时间骰。
  定时再生复用通用状态和自然恢复管线；商店购买、维护与保存复用既有状态。

## 版本与验收

- Protocol 1.172；State Hash Schema v86；save v1。
- 内容包 1.252.0；contract-v261；23 条 active exact fixtures，零 waiver。
- 聚焦验收覆盖入口与类别投影、购买、食用、补货、商店/再生状态存档往返，以及
  内容来源身份和地图入口。
