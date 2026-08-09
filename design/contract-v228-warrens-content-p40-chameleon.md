# Contract v228：P40 真实变色龙形态

状态：已实现。协议 `1.157`，内容包 `1.224.0`，State Hash Schema `v77`，
active baseline `contract-v228`；save 容器保持 v1，不兼容旧开发存档。

## 权威来源与内容范围

- 怪物记录固定读取 RFB `master:lib/edit/r_info.txt` 的索引 `1040`。
- 中文显示名固定使用权威表中的“变色龙”。
- 严格同步清单新增 `demo.actor.chameleon`，总计 368 条；正式包增至 433 actors，
  abilities 保持 174。
- `CHAMELEON` 映射为窄标签 `chameleon`。它与 P20 的 `SHAPECHANGER` 不同：
  后者继续只改变显示投影，不获得形态规则。

## 行为边界

- 生成时立即选择一个形态。之后每次怪物清醒行动开始前先抽取一次 `1/13`
  判定；失败不再抽取形态或 HP。
- 成功后从不高于变色龙等级、仍在该深度范围、非友善、非 Unique、非繁殖、
  非变色龙、非自爆且可通过当前位置地形的正式分配记录中，按稀有度权重选择。
- 新形态重新掷最大生命；当前生命按旧、新最大生命比例缩放。速度和抗性同步
  盖章，派生攻防、近战 routine、移动方式、感知和怪物施法读取当前形态。
- `Actor.kindId` 始终为 `demo.actor.chameleon`；当前形态只保存在既有
  `appearanceKindId`。死亡归属、掉落与本体身份不会改写为形态。

## 持久化与确定性

- 没有新增运行时字段、协议 DTO、save 字段或兼容层。
- `appearanceKindId` 已进入 save 与 State Hash；载入时按当前形态验证 HP、速度
  和施法冷却范围，因此协议保持 `1.157`，State Hash Schema 保持 `v77`。
- 聚焦测试固定 `1/13` RNG 门、形态 HP/速度/抗性、近战/施法/穿墙行为，以及
  本体与形态的 save / State Hash 往返。

正式内容 hash：
`f2f6891805e8b6b23673e2b6f48abcdf894cfc0578a39bc798f15eb66f7af267`。
