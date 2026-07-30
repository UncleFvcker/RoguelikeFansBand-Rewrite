# Contract v145：Detonations 药水

状态：已实现（P95）

Contract v145 接入固定原版 `191f48c3` 的 Detonations Potion（tval 75 / sval 22）。协议保持 `1.121`，demo 内容包为 `1.136.0`，save 容器保持 v1，state hash Schema 保持 `54`。active baseline 包含 449 条 exact fixtures、零 waiver，内置内容 hash 为 `136cc9508d1d45997f193c39689f8604e6e06db258e4a2d22e65b7a24b72f717`。

## 1. 效果与事务

原版 `src/devices.c` 的事务顺序为：`damroll(50, 20)` 的 `DAMAGE_NOESCAPE` 直接伤害，`MAX(stun, STUN_MASSIVE)`（75），再 `cut + 5000`，最后无条件 noticed。本轮加入静态 self-only 消耗品效果：

```text
apply-detonation {
  damageDice,
  damageSides,
  stunTicks,
  bleedingTicks
}
```

合法使用由既有物品入口先记录 Tried、消费物品并支付 100 energy。resolver 只掷 50 个 d20，不经护甲且以 Normal resistance 构造伤害结果，随后复用既有 `incomingDamagePercent` 缩放；因此不读取 Physical resistance，但 Wraithform 等已有入伤缩放仍有效。

伤害后若玩家仍存活，Stun 以 intensity 1、75 ticks、KeepStrongest 应用，Bleeding 以 intensity 1、5000 ticks、Extend 应用。既有 status immunity 继续受尊重。直接伤害致死则跳过两项后续状态，但仍无条件将物品升级为 Aware。不会建立通用物品伤害序列、状态 helper、AbilityEffectDefinition 入口或 debug 开关。

## 2. 事件、内容与导入

`ItemDetonation` 只投影来源、伤害结果和 fatal；最终权威状态表达 Stun/Bleeding，事件不复制状态应用明细。Web 只展示伤害或致死消息，不重新实现状态或致死判断。demo 使用原创 Shatterburst Draught。

legacy importer 把 tval 75 / sval 22 映射为 `{ damageDice: 50, damageSides: 20, stunTicks: 75, bleedingTicks: 5000 }`。固定源码严格导入成功、解析诊断为零，`consumable-effect` 从 66 降至 65，`food-nutrition` 保持 28，`scroll-effect` 保持 15。真实包为 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races；source verify、编译与二进制回读 hash 均为 `e724905cda4f306f6080e80844e61af0a51f1cc692ae678bedbcf7850f33adb6`。

## 3. 契约边界

fixture 449 让默认 Vanguard 使用一瓶 Shatterburst Draught：50 个效果骰合计 494，伤害致死、物品被消费并变为 Aware，且没有后续 Stun/Bleeding。唯一新增核心单测以高生命值覆盖 50 次 RNG、`incomingDamagePercent`、Stun KeepStrongest 和 Bleeding Extend。导入器只增加 `(75, 22)` 的既有表驱动映射测试；不增加免疫专项、Schema 负例、save/replay、Web 单测或 Tauri E2E。

旧正式 demo hash `9f28bf79c8fc72bbcf97beec23da1c1fa0a10045b5c363defcb59e9a29457ed5` 已加入兼容列表。内容 hash 属于 state hash 输入，因此 448 条旧 fixture 只更新 `stateHash` 或 `saveRoundTripStateHash`，其他 assertions 保持不变。
