# Contract v285：戒灵有限生命周期

状态：已实现

## 规则

- 怪物内容可声明 `lifetimeInstanceLimit`。普通 `unique` 未声明时隐式为 1；`unique2`
  仍只限制同时存活一只，死亡后可再次出现。
- 剩余额度统一为“生命周期上限 - 已死亡数量 - 当前及 stored floors 的存活数量”。普通
  分配、固定召唤、分类召唤与读档验证使用同一口径。
- 只有真实死亡永久消耗额度；非死亡移除与楼层重置不增加死亡计数。
- 戒灵（source index 696）声明 `lifetimeInstanceLimit: 5`，并沿用唯一怪物的变形免疫。

## 持久化与验证

- `SavePayloadV1.defeatedLimitedActorCounts` 按稳定 actor ID 保存非零死亡数量，拒绝重复、
  未知、无有限生命周期、越界或 guardian 记录；不兼容缺少该字段的旧开发存档。
- Protocol：`1.189`；State Hash Schema：`v94`；save 容器：`v1`。
- pack：`1.298.0`；content hash：
  `ea49544398120d561c201e480e8dce5b918c75d326a73b786ea4c0d371ad7a7b`。
- active baseline：`contract-v285`，26 条 exact fixture，零 waiver；因状态哈希输入结构变化
  全量刷新并验证。
