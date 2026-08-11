# Contract v260：兽人洞穴与奥斯罗德

## 1. 权威来源

本批次以 `D:/codex/Frogcomposband` 的 RFB `master` Git 对象为权威，固定来源
commit `efd63661302866038f58d8cd2553b23e6af3bf9d`。兽人洞穴使用 dungeon 3：
世界坐标 `(30,45)`、深度 15–32、`MONSTER_DIV_16`，偏好 `oOTC` 与
`ANIMAL | ORC | TROLL`。最终守卫为 1185
`Othrod, Lord of the Orcs` / `半兽人之王奥斯罗德`。

## 2. 地牢与入口

- Middle-earth 正式激活 `demo.dungeon.orc-cave`，并建立连续的 15–32 层。
- 普通荒野在对应世界格中央合成专用入口；专用 terrain ID 避免与 Outpost 的
  Warrens 入口混淆。
- 连续荒野可以直接进入地牢。离层荒野保存在既有 `FloorState` 中；存档、读档和
  返回地表恢复同一个世界坐标与局部地形，不增加新存档 DTO。
- 地牢使用 96×33 的大层、洞室优先的自由房间、矿脉、原版食物/光源保障，以及
  深度 15–32 的独立掉落表。

## 3. 生态与守关者

- encounter table 继续复用全局 actor 分配器：`oOTC`、animal/orc/troll 使用完整
  偏好权重，其他合格怪物按 `16/64` 权重参与，基础自然补怪为 `1/160`。
- 奥斯罗德保留 180d10 强制满生命、四段近战、击晕、火/冷/毒/暗抗性、开门破门、
  escort、尸体/骨骸和 `1_IN_6` 同族召唤。
- `S_KIN` 以 glyph `o` 分类召唤两个等级不高于 32 的同族，不允许复制唯一守关者。
- 击败奥斯罗德只征服 Orc Cave 并增加 10000 分；当前 campaign 的 Warrens 胜利条件
  不变，因此不会提前触发整局胜利。

## 4. 最终奖励

最终奖励固定生成一枚 Fine `Ring`，并强制附加 ego 206 `of Combat` / `战斗之`。
该 ego 的三次物化加权 roll 复用现有 affix 实例状态，可得到力量、敏捷、体质、
近战命中、近战伤害、组合战斗加值或恐惧免疫。普通奥斯罗德掉落使用独立的
Orc Cave 与 warrior 表，不再错误引用只覆盖 0–9 层的 Warrens 表。

## 5. 兼容边界

- Protocol：1.171（不变）
- State Hash Schema：v86（不变）
- Contract：contract-v260
- 内容包：1.251.0
- save 容器：v1（不变，不兼容旧开发存档）
- content hash：`587db0f265c15d2714c238bb7a0cac4c18c8efa3efd55151bd379a4f1c6bf64f`

新增 dungeon 初始状态会改变共同初始化投影，因此 22 个 active exact fixtures 按政策
统一刷新并保持零 waiver。聚焦核心验收覆盖入口、15–32 层、奥斯罗德死亡、战斗之戒、
非 campaign 胜利、存档回环和返回 `(30,45)`；按项目政策不运行旧 E2E。
