# Contract v23：物品鉴别与完整识别

状态：协议 1.23 / contract-v23 active baseline

## 已建立边界

- 物品实例新增 `ordinary`、`fine`、`exceptional` 质量真值；非普通质量只允许数量为一、不可堆叠的实例。首个回声护符质量为 `fine`。
- `Appraise { itemId }` 是标准耗能行动，只作用于当前背包实例。首次鉴别把实例从 `unexamined` 推进到 `appraised`，公开质量但不公开 affix、真实词条修正或完整识别状态。
- 首次装备把实例推进到 `identified`，公开全部当前 affix。质量、鉴别级别和词条知识由核心投影，HTML 不从内容 ID 或数值变化反推隐藏信息。
- save v1 在物品位置 DTO 中保存质量，并在 `itemPropertyKnowledge` 中保存 `appraised`、`identified` 与已知词条。载入 v22 存档时，会从已有词条知识和装备位置推导完整识别。
- 协议升至 1.23，内容包升至 1.18.0，state hash 升至 Schema v12。contract-v23 从 v22 迁移 59 个 exact fixtures，并新增鉴别质量场景，共 60 个。

## 下一步

物品知识状态机已经可以承载外部识别来源。内容驱动掉落表与确定性 `LootContext` 已由 [contract-v24](contract-v24-deterministic-loot-generation.md) 建立；鉴定卷轴、技能、诅咒知识和逐项发现继续复用本状态机。
