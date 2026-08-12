# Contract v278：挖矿熟练度与只读材料袋

## 权威来源

- 挖矿熟练度上限、等级提升判断与中文提示来自 RFB `master:src/skills.c` 的
  `skills_mining_max/skills_mining_gain`。
- 矿脉成长触发点和普通/富矿公式来自 `master:src/cave.c` 的
  `_mining_gain_for_feat/cave_alter_feat`。
- 十种材料的身份与中文名来自 `master:src/materials.c`。所有来源均通过
  `D:/codex/Frogcomposband` 的 `master` Git 对象读取，不依赖工作树。

## 权威状态与成长

- `CharacterProgress.miningProficiency` 出生为 0、上限为 8000；等级边界与逐武器熟练度
  共用 `Unskilled / Beginner / Skilled / Expert / Master`（生疏/入门/熟练/专家/大师）。
- 只有玩家的 `DigTerrain` 成功把带 `veinYield` 的矿脉变为目标地形时增长：普通矿脉为
  `8 + power / 2 + depth / 8`，富矿为 `50 + power + depth / 2`。非矿脉、失败挖掘、
  魔法地形变化、怪物破墙和其他来源不进入此入口。
- 仅跨熟练度等级时发出 `progress.mining-proficiency-improved`，中文严格使用原版
  “你的挖矿熟练度提升了。”；成长本身不消费 RNG。

## 材料袋

- `CharacterProgress.materials` 是稳定材料 ID 到 `u32` 数量的稀疏表；本批保留并投影
  原版十种身份：`rfb.material.iron-ore`、`silver-ore`、`mithril-dust`、
  `crystal-shard`、`herb`、`beast-meat`、`dragon-scale`、`demon-ichor`、
  `arcane-essence`、`rare-catalyst`（均使用完整 `rfb.material.*` ID）。这些不是 item ID，
  不占用 items 分支的物品命名空间。
- 存档只写非零材料；载入拒绝重复、未知或零数量条目。快照按原版固定顺序投影全部十种
  材料及当前数量。材料袋目前只读，不生成矿脉材料，也不导入烹饪、炼药或材料转化。

## 协议、界面与契约

- `WeaponProficiencyRankDto` 泛化为 `ProficiencyRankDto`；`PlayerProgressDto` 新增
  `miningProficiency` 与 `materials`。角色面板新增默认折叠的“杂项熟练度 → 挖矿”和
  “材料”，显示挖掘力、等级、当前值/8000 与十种材料数量。
- `PlayerProgressSaveDto` 新增必填 `miningProficiency` 和 `materials`；不兼容缺字段的旧
  开发存档。新权威状态进入 State Hash Schema v90，save 容器保持 v1。
- Protocol 升至 1.180；内容包保持 1.276.0，content hash 保持
  `ee561b30744f44fd627805d8ed0a45eb64b21ec4c13991033b6f6254b29156b9`。共享投影和哈希
  结构要求刷新全部 active fixture，并新增挖矿/材料存档回放，active baseline 推进为
  contract-v278，共 25 条 exact fixture、零 waiver。
