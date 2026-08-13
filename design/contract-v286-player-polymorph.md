# Contract v286：玩家临时变形与万变魔君

- 权威来源：RFB `master`（`efd63661302866038f58d8cd2553b23e6af3bf9d`）的
  `GF_OLD_POLY` 与怪物 source index `745`。
- 新增近战效果 `polymorph-player`：百分比门控后依次检查当前种族
  `polymorph-immune` 标签、以施术者等级进行豁免、选择原版 race index，最后掷
  `50 + 1d50` 持续时间。免疫或豁免成功不消耗形态/持续时间 RNG。
- 内容保存原版 `legacyIndex`；随机分支维持 0–74 的拒绝重掷，排除原版不合法索引，
  不改写为压缩候选表抽样。
- 临时形态复用既有 `grantedRaceId` 状态。所有授予种族的状态互斥；形态身体槽立即
  对齐，不合槽装备移入背包（超容量时落在脚下），恢复永久身体时不自动重新装备。
- 导入 44 个临时形态 profile（含 Android 免疫 profile、小狗头人和疥癣麻风病人）
  与万变魔君；这些 profile 没有出生 build，不会开放为新游戏选项。
- pack：`1.299.0`；content hash：
  `3d83f462010420e8054c18476f7589d859c8e2e9a1c175a08bd3797e120d4c83`。
- 本批没有新增存档字段或状态哈希输入字段，沿用 Protocol `1.189`、State Hash Schema
  `94`；现有 status/bodySlots 序列化已覆盖保存恢复。
