# Contract v146：属性损伤与恢复

状态：已实现（P96）

Contract v146 接入固定原版 `191f48c3` 的六种属性损伤药水（tval 75 / sval 16–21）与六种属性恢复药水（tval 75 / sval 42–47）。协议升至 `1.122`，demo 内容包为 `1.137.0`，save 容器保持 v1，state hash Schema 升至 `55`。active baseline 包含 450 条 exact fixtures、零 waiver，内置内容 hash 为 `ffd8f8111a5b956a26a6af12bd242aad04a322bb996f587a08fae9db4488925b`。

## 1. 属性历史与事务

玩家进度现在分别保存当前属性和历史最大自然属性。正常加点同时提升两者；属性损伤只降低当前值，恢复药水把当前值恢复到对应历史最大值。旧存档缺少最大属性时迁移为当前属性；当前值高于最大值的损坏存档拒绝载入。

内容只增加两种窄 self-only 消耗品效果：

```text
drain-attribute { attribute }
restore-attribute { attribute }
```

属性值不超过 18 时，损伤降低一点且不抽效果 RNG；高于 18 时按原版 18/xx 损伤公式抽一次有界 RNG，并至少回落 5 点但不低于 18；当前值为 3 时不再损伤。恢复不抽 RNG，只恢复到历史最大值。实际变化才把来源种类标记为 Aware；无变化的合法使用仍消费、推进时间并保持 Tried-only。

属性变化后复用既有派生刷新，更新 HP 上限、资源上限及其当前值比例；不建立通用成长事务、属性 effect DSL 或新的状态框架。

## 2. 协议、事件与内容

`AttributeValueDto` 增加 `maximumNatural`，`PlayerProgressSaveDto` 增加可选 `maximumAttributes` 以兼容旧存档。属性事件只投影来源、属性、变化前后值、历史最大值和 noticed；Web 只展示事件，不重复实现事务判断。中英文属性面板显示当前/历史最大值。

demo 新增原创 `Frailty Tonic` 与 `Strength Renewal Tonic`，分别固定 Strength 的损伤和恢复；原有 `Renewal Tonic` 保持 Restore Life Levels 语义。legacy importer 映射十二个 sval，并将 `consumable-effect` 从 65 降至 53；`food-nutrition` 保持 28，`scroll-effect` 保持 15。固定原版导入报告错误数为零，真实导入内容 hash 为 `450e3eeaa989e04f15747578abb45449ef9662507b47e6a0e8c823cc93dce867`，编译产物 SHA-256 为 `02CB716706AFFC503C1E38AA3D54E7A437B6054794BC1CF88A5C2EF3153E25BE`。

## 3. 契约边界

fixture 450 以 Strength 13 的角色连续使用 Frailty Tonic、两瓶 Strength Renewal Tonic，固定当前值 `13 → 12 → 13 → 13`、三次消费、一次损伤 RNG、首次变化 Aware、无变化保持 Tried-only，并覆盖 save round-trip。核心单测覆盖属性损伤的 3 点下限、18 以下无 RNG、高于 18 的一次 RNG，以及旧存档迁移和当前值高于历史最大值的拒绝。导入器只增加十二个 sval 的表驱动断言；不增加 save/replay、Web 单测、Schema 负例或 Tauri E2E。

旧正式 demo hash `136cc9508d1d45997f193c39689f8604e6e06db258e4a2d22e65b7a24b72f717` 已加入兼容列表。内容 hash 和 state hash Schema 变化使 449 条旧 fixture 只更新 `stateHash` 或 `saveRoundTripStateHash`；其他 assertions 保持不变。
